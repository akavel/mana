use anyhow::{anyhow, bail, Context, Result};
use fn_error_context::context;
use log::debug;
use mlua::prelude::{IntoLua, Lua, LuaMultiValue, LuaTable, LuaValue};
use path_slash::PathBufExt as _;
use thiserror::Error;

use std::collections::BTreeMap;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process;

use script::Effectors as Spec;

pub fn serve(mut args: std::env::Args) -> Result<()> {
    let Some(name) = args.next() else {
        bail!("subcommand 'effector' requires name of effector");
    };
    use effectors::Callee;
    match name.as_str() {
        "*zeroinstall" => f_zeroinstall::Effector::serve(args),
        "*scp" => f_ssh::Effector::serve(args),
        _ => Err(anyhow!("unknown effector name: {name:?}")),
    }
}

type ChildProcs = BTreeMap<String, ChildProc>;

pub struct ChildProc {
    pub proc: process::Child,
    pub buf_out: BufReader<process::ChildStdout>,
}

impl ChildProc {
    #[context("spawning effector {name}")]
    pub fn new_effector(name: &str, args: &[String]) -> Result<Self> {
        let arg0 = std::env::args().next().unwrap();
        let args = ["effector", name]
            .into_iter()
            .chain(args.iter().map(|s| s.as_ref()));
        let mut proc = process::Command::new(arg0)
            .args(args)
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::inherit())
            .spawn()?;
        let buf_out = BufReader::new(proc.stdout.take().unwrap());
        let mut child = ChildProc { proc, buf_out };
        {
            let mut child_in = child.proc.stdin.as_ref().unwrap();
            // TODO: print error details in case of error
            writeln!(child_in, "{}", effectors::HANDSHAKE_RQ)?;
            child_in.flush()?;
            let rs = child.read_line()?;
            if rs.as_str().trim_end() != effectors::HANDSHAKE_RS {
                bail!("expected v2 handshake from {name}, got: {rs:?}");
            }
        }
        Ok(child)
    }

    pub fn read_line(&mut self) -> Result<String> {
        use std::io::BufRead;
        let mut buf = String::new();
        self.buf_out.read_line(&mut buf)?;
        Ok(buf)
    }

    pub fn detect(&mut self, path: &Path) -> Result<bool> {
        let mut child_in = self.proc.stdin.as_ref().unwrap();
        writeln!(
            child_in,
            "detect {}",
            urlencoding::encode(path.to_str().unwrap())
        )?;
        let rs = self.read_line()?;
        match rs.trim_end() {
            "detected present" => Ok(true),
            "detected absent" => Ok(false),
            _ => bail!("unexpected 'detect' response: {:?}", rs),
        }
    }

    pub fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        use urlencoding::encode;
        let mut child_in = self.proc.stdin.as_ref().unwrap();
        writeln!(
            child_in,
            "gather {} {}",
            encode(path.to_str().unwrap()),
            encode(shadow_prefix.to_str().unwrap())
        )?;
        let rs = self.read_line()?;
        if !rs.starts_with("gathered") {
            bail!("unexpected 'gather' response: {:?}", rs);
        }
        Ok(())
    }

    pub fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        use urlencoding::encode;
        let mut child_in = self.proc.stdin.as_ref().unwrap();
        writeln!(
            child_in,
            "affect {} {}",
            encode(path.to_str().unwrap()),
            encode(shadow_prefix.to_str().unwrap())
        )?;
        let rs = self.read_line()?;
        if !rs.starts_with("affected") {
            bail!("unexpected 'affect' response: {:?}", rs);
        }
        Ok(())
    }
}

pub struct Effectors<'lua> {
    lua: LuaTable<'lua>,
    rust: RustEffectors,
    child_procs: ChildProcs,
}

impl<'lua> Effectors<'lua> {
    pub fn init(lua: &'lua Lua, spec: &Spec) -> Result<Effectors<'lua>> {
        let lua_effectors = lua.create_table().unwrap();
        let mut rust_effectors = RustEffectors {
            map: BTreeMap::new(),
        };
        let mut child_procs = ChildProcs::new();
        for (root, cmd) in spec {
            if init_lua_effector(lua, &lua_effectors, root.clone(), cmd.clone()).is_ok() {
                continue;
            }
            match &cmd[..] {
                [s] if s == "zeroinstall" => {
                    rust_effectors
                        .map
                        .insert(root.clone(), Box::new(f_zeroinstall::Effector::new()?));
                }
                [s, args @ ..] if s == "*scp" => {
                    // TODO[LATER]: check no duplicates
                    child_procs.insert(root.clone(), ChildProc::new_effector(s, args)?);
                }
                _ => {
                    bail!("unknown effector command: {cmd:?}");
                }
            }
        }
        Ok(Self {
            lua: lua_effectors,
            rust: rust_effectors,
            child_procs,
        })
    }

    #[context("detecting at {prefix}/{subpath}")]
    pub fn detect(&mut self, prefix: &str, subpath: &str) -> Result<bool> {
        if let Some(v) = self.child_procs.get_mut(prefix) {
            return v.detect(&PathBuf::from_slash(subpath));
        }

        if let Some(r) = self
            .rust
            .maybe_detect(prefix, &PathBuf::from_slash(subpath))
        {
            r
        } else {
            call_effector_method(&self.lua, prefix, "exists", subpath)
                .with_context(|| format!("calling effectors[{prefix:?}]:exists({subpath:?})"))
        }
    }

    #[context("gathering at {prefix}/{subpath}")]
    pub fn gather(&mut self, prefix: &str, subpath: &str, shadow_root: &str) -> Result<()> {
        if let Some(v) = self.child_procs.get_mut(prefix) {
            let subpath = &PathBuf::from_slash(subpath);
            let shadow_root = &PathBuf::from_slash(shadow_root);
            return v.gather(subpath, &shadow_root.join(prefix));
        }

        let rust_result = self.rust.maybe_gather(
            prefix,
            &PathBuf::from_slash(subpath),
            &PathBuf::from_slash(shadow_root),
        );
        if let Some(rs) = rust_result {
            rs
        } else {
            let shadow_path = PathBuf::from(&shadow_root)
                .join(prefix)
                .join(PathBuf::from_slash(subpath));
            call_effector_method(
                &self.lua,
                prefix,
                "query",
                (subpath, shadow_path.to_str().unwrap()),
            )
            .with_context(|| format!("calling effectors[{prefix:?}]:query({subpath:?})"))
        }
    }

    #[context("affecting at {prefix}/{subpath}")]
    pub fn affect(&mut self, prefix: &str, subpath: &str, shadow_root: &str) -> Result<()> {
        if let Some(v) = self.child_procs.get_mut(prefix) {
            let subpath = &PathBuf::from_slash(subpath);
            let shadow_root = &PathBuf::from_slash(shadow_root);
            return v.affect(subpath, &shadow_root.join(prefix));
        }

        let rust_result = self.rust.maybe_affect(
            prefix,
            &PathBuf::from_slash(subpath),
            &PathBuf::from_slash(shadow_root),
        );
        if let Some(rs) = rust_result {
            rs
        } else {
            let shadow_path = PathBuf::from(&shadow_root)
                .join(prefix)
                .join(PathBuf::from_slash(subpath));
            call_effector_method(
                &self.lua,
                prefix,
                "apply",
                (subpath, shadow_path.to_str().unwrap()),
            )
            .with_context(|| format!("calling effectors[{prefix:?}]:apply({subpath:?})"))
        }
    }
}

#[derive(Error, Debug)]
enum InitEffectorError {
    #[error("not a Lua-based effector")]
    NotLua,
}

fn init_lua_effector(lua: &Lua, dst: &LuaTable, root: String, cmd: Vec<String>) -> Result<()> {
    if cmd.len() < 2 || cmd[0] != "lua53" {
        return Err(anyhow::Error::new(InitEffectorError::NotLua));
    }
    //if cmd.len() < 2 {
    //    bail!("Effector for {root:?} has too few elements - expected 2+, got: {cmd:?}");
    //}
    //if cmd[0] != "lua53" {
    //    bail!("FIXME: currently effector[0] must be 'lua53'");
    //}
    let source = std::fs::read_to_string(&cmd[1])
        .with_context(|| format!("reading Lua script for effector for {root:?}"))?;
    // TODO: eval and load returned module - must refactor scripts first
    let mut v = lua.load(source).set_name(&cmd[1]).eval::<LuaValue>()?;
    let LuaValue::Table(ref t) = v else {
        bail!("Effector for {root:?} expected to return Lua table, but got: {v:?}");
    };
    if let Ok(init) = t.get::<&str, LuaValue>("init") {
        debug!("INIT for {root:?} = {init:?}");
        if let LuaValue::Function(ref f) = init {
            let args = cmd[2..]
                .iter()
                .map(|v| v.clone().into_lua(lua).unwrap())
                .collect::<LuaMultiValue>();
            let ret = f.call(args).with_context(|| {
                format!("calling 'init({:?})' on effector for {root:?}", &cmd[2..])
            })?;
            let LuaValue::Table(_) = ret else {
                bail!("calling 'init(...)' on effector for {root:?} expected to return Lua table, got; {ret:?}");
            };
            v = ret;
        }
    }
    dst.set(root.as_str(), v).unwrap();
    Ok(())
}

fn call_effector_method<'a, V: mlua::FromLuaMulti<'a>>(
    effectors: &LuaTable<'a>,
    prefix: &str,
    method: &str,
    args: impl mlua::IntoLuaMulti<'a>,
) -> Result<V> {
    let effector: LuaValue = effectors.get(prefix).unwrap();
    let LuaValue::Table(ref t) = effector else {
        bail!("expected Lua effector for {prefix:?} to be a table, but got: {effector:?}");
    };
    let method_val: LuaValue = t.get(method).unwrap();
    let LuaValue::Function(ref f) = method_val else {
        bail!("expected '{method}' in Lua effector for {prefix:?} to be a function, but got: {method_val:?}");
    };
    let res: V = f.call(args)?;
    Ok(res)
}

struct RustEffectors {
    map: BTreeMap<String, Box<dyn effectors::Callee>>,
}

impl RustEffectors {
    fn maybe_detect(&mut self, prefix: &str, subpath: &Path) -> Option<Result<bool>> {
        self.map.get_mut(prefix).map(|h| h.detect(subpath))
    }

    fn maybe_gather(
        &mut self,
        prefix: &str,
        subpath: &Path,
        shadow_root: &Path,
    ) -> Option<Result<()>> {
        self.map
            .get_mut(prefix)
            .map(|h| h.gather(subpath, &shadow_root.join(prefix)))
    }

    fn maybe_affect(
        &mut self,
        prefix: &str,
        subpath: &Path,
        shadow_root: &Path,
    ) -> Option<Result<()>> {
        self.map
            .get_mut(prefix)
            .map(|h| h.affect(subpath, &shadow_root.join(prefix)))
    }
}

use anyhow::{bail, Context, Result};
use mlua::prelude::{IntoLua, Lua, LuaMultiValue, LuaTable, LuaValue};
use path_slash::PathBufExt as _;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::handler::zeroinstall;
use crate::manaprotocol::callee;
use crate::script::Handlers as Spec;


pub struct Handlers<'lua> {
    lua: LuaTable<'lua>,
    rust: RustHandlers,
}

impl<'lua> Handlers<'lua> {
    pub fn init(lua: &'lua Lua, spec: &Spec) -> Result<Handlers<'lua>> {
        let lua_handlers = lua.create_table().unwrap();
        let mut rust_handlers = RustHandlers {
            map: BTreeMap::new(),
        };
        for (root, cmd) in spec {
            if let Ok(_) = init_lua_handler(&lua, &lua_handlers, root.clone(), cmd.clone()) {
                continue;
            }
            match &cmd[..] {
                [s] if s == "zeroinstall" => {
                    rust_handlers
                        .map
                        .insert(root.clone(), Box::new(zeroinstall::Handler::new()?));
                }
                _ => {
                    bail!("unknown handler command: {cmd:?}");
                }
            }
        }
        Ok(Self{lua: lua_handlers, rust: rust_handlers})
    }

    pub fn detect(&mut self, prefix: &str, subpath: &str) -> Result<bool> {
        if let Some(r) = self.rust.maybe_detect(&prefix, &PathBuf::from_slash(subpath)) {
            r
        } else {
            call_handler_method(&self.lua, prefix, "exists", subpath)
                .with_context(|| format!("calling handlers[{prefix:?}]:exists({subpath:?})"))
        }
    }

    pub fn gather(&mut self, prefix: &str, subpath: &str, shadow_root: &str) -> Result<()> {
        let rust_result = self.rust
            .maybe_gather(
                &prefix,
                &PathBuf::from_slash(subpath),
                &PathBuf::from_slash(&shadow_root),
            );
        if let Some(rs) = rust_result {
            rs
        } else {
            let shadow_path = PathBuf::from(&shadow_root)
                .join(&prefix)
                .join(PathBuf::from_slash(&subpath));
            call_handler_method(
                &self.lua,
                prefix,
                "query",
                (subpath, shadow_path.to_str().unwrap()),
            )
            .with_context(|| format!("calling handlers[{prefix:?}]:query({subpath:?})"))
        }
    }

    pub fn affect(&mut self, prefix: &str, subpath: &str, shadow_root: &str) -> Result<()> {
        let rust_result = self.rust.maybe_affect(
            &prefix,
            &PathBuf::from_slash(subpath),
            &PathBuf::from_slash(&shadow_root),
        );
        if let Some(rs) = rust_result {
            rs
        } else {
            let shadow_path = PathBuf::from(&shadow_root)
                .join(&prefix)
                .join(PathBuf::from_slash(&subpath));
            call_handler_method(
                &self.lua,
                prefix,
                "apply",
                (subpath, shadow_path.to_str().unwrap()),
            )
            .with_context(|| format!("calling handlers[{prefix:?}]:apply({subpath:?})"))
        }
    }
}


#[derive(Error, Debug)]
enum InitHandlerError {
    #[error("not a Lua-based handler")]
    NotLua,
}

fn init_lua_handler(lua: &Lua, dst: &LuaTable, root: String, cmd: Vec<String>) -> Result<()> {
    if cmd.len() < 2 || cmd[0] != "lua53" {
        return Err(anyhow::Error::new(InitHandlerError::NotLua));
    }
    //if cmd.len() < 2 {
    //    bail!("Handler for {root:?} has too few elements - expected 2+, got: {cmd:?}");
    //}
    //if cmd[0] != "lua53" {
    //    bail!("FIXME: currently handler[0] must be 'lua53'");
    //}
    let source = std::fs::read_to_string(&cmd[1])
        .with_context(|| format!("reading Lua script for handler for {root:?}"))?;
    // TODO: eval and load returned module - must refactor scripts first
    let mut v = lua.load(source).set_name(&cmd[1]).eval::<LuaValue>()?;
    let LuaValue::Table(ref t) = v else {
        bail!("Handler for {root:?} expected to return Lua table, but got: {v:?}");
    };
    if let Ok(init) = t.get::<&str, LuaValue>("init") {
        println!("INIT for {root:?} = {init:?}");
        if let LuaValue::Function(ref f) = init {
            let args = cmd[2..]
                .iter()
                .map(|v| v.clone().into_lua(&lua).unwrap())
                .collect::<LuaMultiValue>();
            let ret = f.call(args).with_context(|| {
                format!("calling 'init({:?})' on handler for {root:?}", &cmd[2..])
            })?;
            let LuaValue::Table(_) = ret else {
                bail!("calling 'init(...)' on handler for {root:?} expected to return Lua table, got; {ret:?}");
            };
            v = ret;
        }
    }
    dst.set(root.as_str(), v).unwrap();
    Ok(())
}

fn call_handler_method<'a, V: mlua::FromLuaMulti<'a>>(
    handlers: &LuaTable<'a>,
    prefix: &str,
    method: &str,
    args: impl mlua::IntoLuaMulti<'a>,
) -> Result<V> {
    let handler: LuaValue = handlers.get(prefix).unwrap();
    let LuaValue::Table(ref t) = handler else {
        bail!("expected Lua handler for {prefix:?} to be a table, but got: {handler:?}");
    };
    let method_val: LuaValue = t.get(method).unwrap();
    let LuaValue::Function(ref f) = method_val else {
        bail!("expected '{method}' in Lua handler for {prefix:?} to be a function, but got: {method_val:?}");
    };
    let res: V = f.call(args)?;
    Ok(res)
}


struct RustHandlers {
    map: BTreeMap<String, Box<dyn callee::Handler>>,
}

impl RustHandlers {
    fn maybe_detect(&mut self, prefix: &str, subpath: &Path) -> Option<Result<bool>> {
        self.map.get_mut(prefix).map(|h| h.detect(&subpath))
    }

    fn maybe_gather(
        &mut self,
        prefix: &str,
        subpath: &Path,
        shadow_root: &Path,
    ) -> Option<Result<()>> {
        self.map
            .get_mut(prefix)
            .map(|h| h.gather(&subpath, &shadow_root.join(prefix)))
    }

    fn maybe_affect(
        &mut self,
        prefix: &str,
        subpath: &Path,
        shadow_root: &Path,
    ) -> Option<Result<()>> {
        self.map
            .get_mut(prefix)
            .map(|h| h.affect(&subpath, &shadow_root.join(prefix)))
    }
}

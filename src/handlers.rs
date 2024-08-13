use anyhow::{bail, Context, Result};
use mlua::prelude::{IntoLua, Lua, LuaMultiValue, LuaTable, LuaValue};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::handler::zeroinstall;
use crate::manaprotocol::callee;
use crate::script::Handlers as Spec;


pub struct Handlers<'lua> {
    pub lua: LuaTable<'lua>,
    pub rust: RustHandlers,
}

pub fn init<'lua>(lua: &'lua Lua, spec: &Spec) -> Result<Handlers<'lua>> {
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
    Ok(Handlers{lua: lua_handlers, rust: rust_handlers})
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


pub struct RustHandlers {
    map: BTreeMap<String, Box<dyn callee::Handler>>,
}

impl RustHandlers {
    pub fn maybe_detect(&mut self, prefix: &str, subpath: &Path) -> Option<Result<bool>> {
        self.map.get_mut(prefix).map(|h| h.detect(&subpath))
    }

    pub fn maybe_gather(
        &mut self,
        prefix: &str,
        subpath: &Path,
        shadow_root: &Path,
    ) -> Option<Result<()>> {
        self.map
            .get_mut(prefix)
            .map(|h| h.gather(&subpath, &shadow_root.join(prefix)))
    }

    pub fn maybe_affect(
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

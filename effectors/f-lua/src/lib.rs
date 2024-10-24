use anyhow::{bail, Result};
use fn_error_context::context;
use mlua::prelude::{IntoLua, Lua, LuaMultiValue, LuaTable, LuaValue};
// use mlua::prelude::*;

use std::path::Path;

pub struct Effector {
    lua: Lua,
}

impl Effector {
    fn load_embedded_packages(&self) {
        self.load_pkg("effectors.winfs", include_str!("../../winfs.lua"));
        self.load_pkg("effectors.winhome", include_str!("../../winhome.lua"));
        self.load_pkg("effectors.winpath", include_str!("../../winpath.lua"));
        self.load_pkg("effectors.posixfiles", include_str!("../../posixfiles.lua"));
        self.load_pkg("effectors.posixdirs", include_str!("../../posixdirs.lua"));
        self.load_pkg("effectors.posixfs", include_str!("../../posixfs.lua"));
        self.load_pkg("effectors.systemctl", include_str!("../../systemctl.lua"));
    }

    fn load_pkg(&self, name: &str, code: &str) {
        let lua = &self.lua;
        // Load, parse, and evaluate `code` into Lua.
        let lib_: LuaValue = lua.load(code).set_name(name).eval().unwrap();
        // Expect the result of the evaluation to be a Lua table.
        let LuaValue::Table(ref lib) = lib_ else {
            panic!("*lua {name} expected to return a table, but got: {lib_:?}");
        };
        // Assign the table into `package.loaded[$name]` in Lua
        let package_: LuaValue = lua.globals().get("package").unwrap();
        let LuaValue::Table(ref package) = package_ else {
            panic!("*lua failed to find _G.package table");
        };
        let loaded_: LuaValue = package.get("loaded").unwrap();
        let LuaValue::Table(ref loaded) = loaded_ else {
            panic!("*lua failed to find _G.package.loaded table");
        };
        loaded.set(name, lib.clone()).unwrap();
    }

    // Initialize a Lua effector package. Runs a Lua script:
    // `_G._MANA = require($name).init(table.unpack($args))`
    #[context("*lua initializing effector {name:?}")]
    fn init_pkg(&self, name: &str, args: std::env::Args) -> Result<()> {
        let lua = &self.lua;
        let g = lua.globals();
        let require_: LuaValue = g.get("require").unwrap();
        let LuaValue::Function(ref require) = require_ else {
            panic!("*lua failed to find _G.require function");
        };
        let lib_: LuaValue = require.call(name)?;
        let LuaValue::Table(ref lib) = lib_ else {
            bail!("*lua expected a table from `require({name:?})`, got: {lib_:?}");
        };
        let init_: LuaValue = lib.get("init")?;
        let LuaValue::Function(ref init) = init_ else {
            bail!("*lua expected a function at `require({name:?}).init`, got: {init_:?}");
        };
        // collect args to pass to init()
        let args_: LuaMultiValue = args
            .into_iter()
            .map(|a| LuaValue::String(lua.create_string(a).unwrap()))
            .collect();
        // call `init($args...)`
        let obj_: LuaValue = init.call(args_)?;
        let LuaValue::Table(ref obj) = obj_ else {
            bail!("*lua expected a table from `require({name:?}).init(...)`, got: {obj_:?}");
        };
        // store result in `_G._MANA`
        let _ = g.set(MANA_GLOBAL, obj_)?;
        Ok(())
    }
}

const MANA_GLOBAL: &str = "_MANA";

impl effectors::Callee for Effector {
    fn start(mut args: std::env::Args) -> Result<Self> {
        let f = Self { lua: Lua::new() };
        f.load_embedded_packages();
        let pkg = args.next();
        let Some(pkg) = pkg else {
            bail!("*lua requires an argument: name of a Lua-based effector package");
        };
        f.init_pkg(&pkg, args)?;
        Ok(f)
    }

    fn detect(&mut self, path: &Path) -> Result<bool> {
        let lua = &self.lua;
        let obj_: LuaValue = lua.globals().get(MANA_GLOBAL)?;
        let LuaValue::Table(ref obj) = obj_ else {
            bail!("*lua expected a table at `_G.{MANA_GLOBAL}`, got: {obj_:?}");
        };
        let func_: LuaValue = obj.get("exists")?;
        let LuaValue::Function(ref func) = func_ else {
            bail!("*lua expected a function at `_G.{MANA_GLOBAL}.exists`, got: {func_:?}");
        };
        // FIXME: change .unwrap() to .ok_or_else(...)
        let res: bool = func.call(path.to_str().unwrap())?;
        Ok(res)
    }

    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        todo!();
    }

    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        todo!();
    }
}

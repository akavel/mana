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
        // Load, parse, and evaluate `code` into Lua.
        let chunk = self.lua.load(code).set_name(name).eval::<LuaValue>().unwrap();
        // Expect the result of the evaluation to be a Lua table.
        let LuaValue::Table(ref t_chunk) = chunk else {
            panic!("*lua {name} expected to return a table, but got: {chunk:?}");
        };
        // Assign the table into `package.loaded[$name]` in Lua
        let package: LuaValue = self.lua.globals().get("package").unwrap();
        let LuaValue::Table(ref t_package) = package else {
            panic!("*lua failed to find _G.package table");
        };
        let loaded: LuaValue = t_package.get("loaded").unwrap();
        let LuaValue::Table(ref t_loaded) = loaded else {
            panic!("*lua failed to find _G.package.loaded table");
        };
        t_loaded.set(name, t_chunk.clone()).unwrap();
    }
}

impl effectors::Callee for Effector {
    fn start(args: std::env::Args) -> Result<Self> {
        let f = Self { lua: Lua::new() };
        f.load_embedded_packages();
        Ok(f)
    }

    fn detect(&mut self, path: &Path) -> Result<bool> {
        todo!();
    }

    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        todo!();
    }

    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        todo!();
    }
}

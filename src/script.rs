use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;
use toml::macros::Deserialize;

#[derive(Debug)]
pub struct Script {
    pub shadow_dir: String,
    pub handlers: Handlers,
    pub paths: PathContentMap,
}

pub type Handlers = BTreeMap<String, Vec<String>>;
pub type PathContentMap = BTreeMap<String, String>;

impl Script {
    pub fn parse_ncl_file(ncl_path: PathBuf) -> Result<Self> {
        let username = whoami::username();
        let hostname = whoami::hostname();
        let field_path_raw = format!("{username}@{hostname}");

        use nickel_lang_core::{
            error::report::ErrorFormat, eval::cache::lazy::CBNCache, identifier::LocIdent,
            pretty::ident_quoted, program::Program as Prog,
        };
        let field_path = ident_quoted(&LocIdent::new(field_path_raw));
        // println!("FIELD: {field_path:?}");
        use std::io::stderr;
        let mut prog = Prog::<CBNCache>::new_from_file(&ncl_path, stderr())?;
        let res_field = prog.parse_field_path(field_path.clone());
        let Ok(field) = res_field else {
            prog.report(res_field.unwrap_err(), ErrorFormat::Text);
            bail!("failed to parse {field_path:?} as Nickel path");
        };
        prog.field = field;
        let res_term = prog.eval_full_for_export();
        let Ok(term) = res_term else {
            prog.report(res_term.unwrap_err(), ErrorFormat::Text);
            bail!("script {ncl_path:?} failed");
        };
        let mut toml = toml::Table::deserialize(term).context("loading Nickel output to TOML")?;
        Self::parse_toml(&mut toml)
    }

    fn parse_toml(toml: &mut toml::Table) -> Result<Self> {
        // println!("PARSED: {toml:?}");

        // Extract `shadow_dir` from toml
        // TODO[LATER]: use serde instead to extract, maybe
        let Some(shadow_dir) = toml.remove("shadow_dir") else {
            bail!("Missing 'shadow_dir' in stdin");
        };
        let toml::Value::String(shadow_dir) = shadow_dir else {
            bail!("Expected 'shadow_dir' to be text, got: {shadow_dir:?}");
        };
        println!("SHAD: {shadow_dir:?}");

        // Extract `handlers` from toml
        // TODO[LATER]: use serde instead to extract, maybe
        let Some(raw_handlers) = toml.remove("handlers") else {
            bail!("Missing 'handlers' in stdin");
        };
        let toml::Value::Table(raw_handlers) = raw_handlers else {
            bail!("Expected 'handlers' to be table, got: {raw_handlers:?}");
        };

        // Extract `tree` from toml
        // TODO[LATER]: use serde instead to extract, maybe
        let Some(raw_tree) = toml.remove("tree") else {
            bail!("Missing 'tree' in stdin");
        };
        let toml::Value::Table(raw_tree) = raw_tree else {
            bail!("Expected 'tree' to be table, got: {raw_tree:?}");
        };

        // Convert handlers to a simple map
        // TODO[LATER]: preserve original order
        let mut handlers = Handlers::new();
        for (k, v) in raw_handlers {
            let toml::Value::String(s) = v else {
                bail!("Unexpected type of handler {k:?}, want String, got: {v:?}");
            };
            handlers.insert(k, s.split_whitespace().map(str::to_string).collect());
        }
        println!("HANDL: {handlers:?}");

        // Convert tree to paths map
        let mut paths = PathContentMap::new();
        let mut todo = vec![(String::new(), raw_tree)];
        loop {
            let Some((parent, subtree)) = todo.pop() else {
                break;
            };
            for (key, value) in subtree {
                let path = parent.clone() + &key;
                match value {
                    toml::Value::String(s) => {
                        paths.insert(path, s);
                    }
                    toml::Value::Table(t) => {
                        todo.push((path + "/", t));
                    }
                    _ => {
                        bail!("Unexpected type of value at {path:?} in tree: {value}");
                    }
                }
            }
        }
        // for (k, v) in &paths {
        //     let n = v.len();
        //     println!(" * {k:?} = {n}");
        // }

        Ok(Script {
            shadow_dir,
            handlers,
            paths,
        })
    }
}


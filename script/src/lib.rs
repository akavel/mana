use anyhow::{bail, Result};
use log::debug;
use thiserror::Error;

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug)]
#[cfg_attr(test, derive(Default))]
pub struct Script {
    pub shadow_dir: String,
    pub effectors: Effectors,
    pub paths: PathContentMap,
}

pub type Effectors = BTreeMap<String, Vec<String>>;
pub type PathContentMap = BTreeMap<String, String>;

impl Script {
    pub fn parse_ncl_file(ncl_path: PathBuf) -> Result<Self> {
        let mut toml = parse_ncl::from_file(ncl_path)?;
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
        debug!("SHAD: {shadow_dir:?}");

        // Extract `effectors` from toml
        // TODO[LATER]: use serde instead to extract, maybe
        let Some(raw_effectors) = toml.remove("effectors") else {
            bail!("Missing 'effectors' in stdin");
        };
        let toml::Value::Table(raw_effectors) = raw_effectors else {
            bail!("Expected 'effectors' to be table, got: {raw_effectors:?}");
        };

        // Extract `tree` from toml
        // TODO[LATER]: use serde instead to extract, maybe
        let Some(raw_tree) = toml.remove("tree") else {
            bail!("Missing 'tree' in stdin");
        };
        let toml::Value::Table(raw_tree) = raw_tree else {
            bail!("Expected 'tree' to be table, got: {raw_tree:?}");
        };

        // Convert effectors to a simple map
        // TODO[LATER]: preserve original order
        let mut effectors = Effectors::new();
        for (k, v) in raw_effectors {
            let toml::Value::String(s) = v else {
                bail!("Unexpected type of effector {k:?}, want String, got: {v:?}");
            };
            effectors.insert(k, s.split_whitespace().map(str::to_string).collect());
        }
        debug!("HANDL: {effectors:?}");

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
            effectors,
            paths,
        })
    }

    pub fn validate(&self) -> ValidationResult {
        use ValidationError::*;
        // TODO: instead, canonicalize path & detect diff, to also find `/../` etc.
        // TODO: also, find dupes in paths, incl. case-insensitively
        fn path_error_of(p: &String) -> Option<ValidationError> {
            if p.ends_with("/") {
                return Some(TrailingSlashInPath(p.clone()));
            } else if p.contains("//") {
                return Some(DoubleSlashInPath(p.clone()));
            } else if p.contains("/../") {
                return Some(DoubleDotInPath(p.clone()));
            }
            None
        }
        if let Some(err) = self.paths.keys().flat_map(path_error_of).next() {
            return Err(err);
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum ValidationError {
    #[error("path `{0}` contains double slash `//`")]
    DoubleSlashInPath(String),
    #[error("path `{0}` ends with a slash `/`")]
    TrailingSlashInPath(String),
    #[error("path `{0}` contains double dot `/../`")]
    DoubleDotInPath(String),
}

type ValidationResult = std::result::Result<(), ValidationError>;

#[cfg(test)]
mod tests {
    use super::*;

    fn s(string: &str) -> String {
        string.to_string()
    }

    #[test]
    fn validate_problems_in_paths() {
        // validate a Script built with given paths
        fn vsp<'a>(paths: impl IntoIterator<Item = &'a str>) -> ValidationResult {
            Script {
                paths: paths
                    .into_iter()
                    .map(|s| (s.to_string(), "".to_string()))
                    .collect(),
                ..<_>::default()
            }
            .validate()
        }

        use ValidationError::*;
        assert_eq!(vsp(["a/../b"]), Err(DoubleDotInPath(s("a/../b"))));
        assert_eq!(vsp(["foo//bar"]), Err(DoubleSlashInPath(s("foo//bar"))));
        assert_eq!(
            vsp(["foo/bar//baz"]),
            Err(DoubleSlashInPath(s("foo/bar//baz")))
        );
        assert_eq!(
            vsp(["ok_a/ok_b", "foo/bar//baz"]),
            Err(DoubleSlashInPath(s("foo/bar//baz")))
        );
        assert_eq!(
            vsp(["ok_a/ok_b", "foo/bar/"]),
            Err(TrailingSlashInPath(s("foo/bar/")))
        );
        assert_eq!(
            vsp(["ok_a/ok_b", "foo/"]),
            Err(TrailingSlashInPath(s("foo/")))
        );
    }
}

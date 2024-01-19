use anyhow::{bail, Context, Result};
use git2::Repository;
use mlua::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
// Trait for extending std::path::PathBuf
use path_slash::PathBufExt as _;
use unicase::UniCase;

fn main() -> Result<()> {
    println!("Hello, world!");
    // Read and parse input - just parse TOML for now.
    // TODO: would prefer to somehow do it in streamed way, maybe
    let input = std::io::read_to_string(std::io::stdin()).context("failure reading stdin")?;
    let script = parse_input_toml(&input)?;

    // open git repo and check if it's clean
    let repo = Repository::open(&script.shadow_dir).context("failure opening 'shadow_dir'")?;
    // check if repo is clean
    {
        if repo.state() != git2::RepositoryState::Clean {
            bail!(
                "git 'shadow_dir' repository has pending unfinished operation {:?}",
                repo.state()
            );
        }
        let mut stat_opt = git2::StatusOptions::new();
        stat_opt.include_untracked(true);
        let stat = repo.statuses(Some(&mut stat_opt))?;
        if !stat.is_empty() {
            bail!("git 'shadow_dir' repository is not clean (see: git status)");
        }
    }

    // TODO: initialize handlers
    let lua = Lua::new();
    let lua_handlers = lua.create_table().unwrap();
    for (root, cmd) in script.handlers {
        if cmd.len() < 2 {
            bail!("Handler for {root:?} has too few elements - expected 2+, got: {cmd:?}");
        }
        if cmd[0] != "lua53" {
            bail!("FIXME: currently handler[0] must be 'lua53'");
        }
        let source = std::fs::read_to_string(&cmd[1])
            .with_context(|| format!("reading Lua script for handler for {root:?}"))?;
        // TODO: eval and load returned module - must refactor scripts first
        let v = lua.load(source).set_name(&cmd[1]).eval::<LuaValue>()?;
        let LuaValue::Table(ref t) = v else {
            bail!("Handler for {root:?} expected to return Lua table, but got: {v:?}");
        };
        if let Ok(init) = t.get::<&str, LuaValue>("init") {
            println!("INIT for {root:?} = {init:?}");
        }
        lua_handlers.set(&*root, v).unwrap();

        // let mut args = cmd.into_iter();
        // let Some(arg) = args.next
    }

    // Make a list of paths in 'tree' and in git
    let head = repo.head()?;
    let head_tree = head.peel_to_tree()?;
    let mut paths = PathSet::new();
    // TODO: unicode normaliz.: https://stackoverflow.com/q/47813162/#comment82595250_47813878
    let mut case_insensitive_slash_paths =
        std::collections::HashMap::<UniCase<String>, String>::new();
    head_tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let name = entry.name().unwrap();
            let slash_path = root.to_string() + name;
            // TODO: also check if entry already existed here
            case_insensitive_slash_paths.insert(slash_path.clone().into(), slash_path);
            let parent = PathBuf::from_slash(root);
            paths.insert(parent.join(name));
        }
        git2::TreeWalkResult::Ok
    })?;
    // for k in &paths {
    //     println!(" - {k:?}");
    // }
    for path in script.paths.keys() {
        let slash_path = path.to_slash().unwrap();
        let unicase = slash_path.clone().into();
        if let Some(found) = case_insensitive_slash_paths.get(&unicase) {
            if found.as_str() != slash_path {
                bail!("Found casing difference between git path {found:?} and input path {slash_path:?}");
            }
        }
        // TODO: case_insensitive_slash_paths.insert(slash_path, slash_path);
        paths.insert(path.clone());
    }
    for k in &paths {
        println!(" - {k:?}");
    }

    // TODO: run 'gather' on appropriate handlers for all listed paths, fetching files into the git workspace

    // TODO: two-way compare: current git <-> results of handlers.gather (use git-workspace)
    // TODO: 3-way compare: curr git <-> handlers.query results <-> parsed input
    // TODO: https://github.com/akavel/drafts/blob/main/20231122-001-mana2.md
    Ok(())
}

type PathMap = BTreeMap<PathBuf, String>;
type PathSet = BTreeSet<PathBuf>;
type Handlers = BTreeMap<String, Vec<String>>;

#[derive(Debug)]
struct Script {
    pub shadow_dir: String,
    pub handlers: Handlers,
    pub paths: PathMap,
}

// Parse input - just parse TOML for now.
fn parse_input_toml(input: &str) -> Result<Script> {
    let mut toml = input
        .parse::<toml::Table>()
        .context("failed to parse stdin as TOML")?;
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
    let mut paths = PathMap::new();
    let mut todo = vec![(PathBuf::new(), raw_tree)];
    loop {
        let Some((parent, subtree)) = todo.pop() else {
            break;
        };
        for (key, value) in subtree {
            let path = parent.join(key);
            match value {
                toml::Value::String(s) => {
                    paths.insert(path, s);
                }
                toml::Value::Table(t) => {
                    todo.push((path, t));
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

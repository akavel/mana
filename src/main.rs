use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use git2::Repository;
// mlua::prelude::* except ErrorContext; TODO: can we do simpler?
use mlua::prelude::{
    FromLua, FromLuaMulti, IntoLua, IntoLuaMulti, Lua, LuaMultiValue, LuaResult, LuaTable, LuaValue,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
// Trait for extending std::path::PathBuf
use path_slash::PathBufExt as _;
use unicase::UniCase;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check actual state of the machine and serialize it into git
    /// working directory at 'shadow_dir'.
    Query,
    /// Serialize input into git working directory at 'shadow_dir'.
    Draft,
    /// Apply the contents of the git working directory to the state
    /// of the machine. For each successfully applied file, perform
    /// `git add` on it.
    Apply,
}

fn main() -> Result<()> {
    // TODO: 3 commands:
    // TODO: query: real world -> git; then compare/diff by hand
    // TODO: draft: Nickel -> git; then compare/diff by hand
    // TODO: apply: git -> real world, + git add each successful

    println!("Hello, world!");

    let cli = Cli::parse();
    match &cli.command {
        Command::Query => query(),
        Command::Draft => bail!("TODO"),
        Command::Apply => bail!("TODO"),
    }

    // TODO[LATER]: licensing information in --license flag
}

fn query() -> Result<()> {
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
        if !check_git_statuses_empty(&repo)? {
            bail!("git 'shadow_dir' repository is not clean (see: git status)");
        }
    }

    // Initialize handlers
    let lua = Lua::new();
    let lua_handlers = lua.create_table().unwrap();
    for (root, cmd) in script.handlers {
        init_handler(&lua, &lua_handlers, root, cmd)?;
    }

    // Make a list of paths in 'tree' and in git
    let head = repo.head()?;
    let head_tree = head.peel_to_tree()?;
    let mut paths = PathSet::new();
    // TODO: unicode normaliz.: https://stackoverflow.com/q/47813162/#comment82595250_47813878
    let mut case_insensitive_paths = std::collections::HashMap::<UniCase<String>, String>::new();
    head_tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let name = entry.name().unwrap();
            let slash_path = root.to_string() + name;
            // TODO: also check if entry already existed here
            case_insensitive_paths.insert(slash_path.clone().into(), slash_path.clone());
            paths.insert(slash_path);
        }
        git2::TreeWalkResult::Ok
    })?;
    // for k in &paths {
    //     println!(" - {k:?}");
    // }
    for path in script.paths.keys() {
        let unicase = path.clone().into();
        if let Some(found) = case_insensitive_paths.get(&unicase) {
            if found.as_str() != path {
                bail!("Found casing difference between git path {found:?} and input path {path:?}");
            }
        }
        // TODO: case_insensitive_paths.insert(path, path);
        paths.insert(path.clone());
    }
    for k in &paths {
        println!(" - {k:?}");
    }

    // Run 'query' on appropriate handlers for all listed paths, fetching files into the git workspace
    for path in &paths {
        let (prefix, subpath) = split_handler_path(&path);
        let found: bool = call_handler_method(&lua_handlers, prefix, "exists", subpath)
            .with_context(|| format!("calling handlers[{prefix:?}]:exists({subpath:?})"))?;
        // println!(" . {prefix:?} {subpath:?} {found:?}");
        let shadow_path = PathBuf::from(&script.shadow_dir).join(PathBuf::from_slash(path));
        if !found {
            std::fs::remove_file(shadow_path);
            continue;
        }
        call_handler_method(
            &lua_handlers,
            prefix,
            "query",
            (subpath, shadow_path.to_str().unwrap()),
        )
        .with_context(|| format!("calling handlers[{prefix:?}]:query({subpath:?})"))?;
    }

    // Two-way compare: current git <-> results of handlers.query
    if !check_git_statuses_empty(&repo)? {
        bail!(
            "real disk contents differ from expected prerequisites; check git diff in shadow repo: {:?}", script.shadow_dir,
        );
    }

    // TODO: 3-way compare: curr git <-> handlers.query results <-> parsed input
    // TODO: https://github.com/akavel/drafts/blob/main/20231122-001-mana2.md
    Ok(())
}

type PathContentMap = BTreeMap<String, String>;
type PathSet = BTreeSet<String>;
type Handlers = BTreeMap<String, Vec<String>>;

#[derive(Debug)]
struct Script {
    pub shadow_dir: String,
    pub handlers: Handlers,
    pub paths: PathContentMap,
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

fn check_git_statuses_empty(repo: &Repository) -> Result<bool> {
    let mut stat_opt = git2::StatusOptions::new();
    stat_opt.include_untracked(true);
    let stat = repo.statuses(Some(&mut stat_opt))?;
    Ok(stat.is_empty())
}

fn init_handler(lua: &Lua, dst: &LuaTable, root: String, cmd: Vec<String>) -> Result<()> {
    if cmd.len() < 2 {
        bail!("Handler for {root:?} has too few elements - expected 2+, got: {cmd:?}");
    }
    if cmd[0] != "lua53" {
        bail!("FIXME: currently handler[0] must be 'lua53'");
    }
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

fn split_handler_path(path: &str) -> (&str, &str) {
    let Some(idx) = path.find('/') else {
        panic!("slash not found in path: {path:?}");
    };
    let (start, rest) = path.split_at(idx);
    let (_slash, end) = rest.split_at(1);
    return (start, end);
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

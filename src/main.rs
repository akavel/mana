use anyhow::{bail, Context, Result};
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use clap::{Parser, Subcommand};
use git2::Repository;
// mlua::prelude::* except ErrorContext; TODO: can we do simpler?
use mlua::prelude::{IntoLua, Lua, LuaMultiValue, LuaTable, LuaValue};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use thiserror::Error;
// Trait for extending std::path::PathBuf
use path_slash::PathBufExt as _;
use unicase::UniCase;

use mana2::handler::zeroinstall;
use mana2::manaprotocol::callee;
use mana2::script::Script;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to a file containing a Nickel script to evaluate.
    #[arg(short, long, default_value="mana.ncl")]
    ncl: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check actual state of the machine and serialize it into git
    /// working directory at 'shadow_dir'.
    Query,
    /// Serialize desired state (as read from input) into git working
    /// directory at 'shadow_dir'.
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
    let script = Script::parse_ncl_file(cli.ncl)?;
    match &cli.command {
        Command::Query => query(script),
        Command::Draft => draft(script),
        Command::Apply => apply(script),
    }

    // TODO[LATER]: licensing information in --license flag
}

fn query(script: Script) -> Result<()> {
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
    let mut rust_handlers = RustHandlers {
        map: BTreeMap::new(),
    };
    for (root, cmd) in script.handlers {
        if let Ok(_) = init_lua_handler(&lua, &lua_handlers, root.clone(), cmd.clone()) {
            continue;
        }
        match &cmd[..] {
            [s] if s == "zeroinstall" => {
                rust_handlers
                    .map
                    .insert(root, Box::new(zeroinstall::Handler::new()?));
            }
            _ => {
                bail!("unknown handler command: {cmd:?}");
            }
        }
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
    let dir = Dir::open_ambient_dir(&script.shadow_dir, ambient_authority())?;
    for path in &paths {
        if let Some(parent) = parent_dir(&PathBuf::from_slash(&path)) {
            dir.create_dir_all(parent).context("in shadow_dir")?;
        }
        let (prefix, subpath) = split_handler_path(&path);
        let found: bool =
            if let Some(r) = rust_handlers.maybe_detect(&prefix, &PathBuf::from_slash(subpath)) {
                r?
            } else {
                call_handler_method(&lua_handlers, prefix, "exists", subpath)
                    .with_context(|| format!("calling handlers[{prefix:?}]:exists({subpath:?})"))?
            };
        // println!(" . {prefix:?} {subpath:?} {found:?}");
        let shadow_path = PathBuf::from(&script.shadow_dir).join(PathBuf::from_slash(path));
        if !found {
            std::fs::remove_file(shadow_path);
            continue;
        }
        if rust_handlers
            .maybe_gather(
                &prefix,
                &PathBuf::from_slash(subpath),
                &PathBuf::from_slash(&script.shadow_dir),
            )
            .is_none()
        {
            call_handler_method(
                &lua_handlers,
                prefix,
                "query",
                (subpath, shadow_path.to_str().unwrap()),
            )
            .with_context(|| format!("calling handlers[{prefix:?}]:query({subpath:?})"))?;
        };
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

fn draft(script: Script) -> Result<()> {
    // Make a list of paths in git
    let repo = Repository::open(&script.shadow_dir).context("failure opening 'shadow_dir'")?;
    let head = repo.head()?;
    let head_tree = head.peel_to_tree()?;
    // TODO: unicode normaliz.: https://stackoverflow.com/q/47813162/#comment82595250_47813878
    //let mut case_insensitive_paths = std::collections::HashMap::<UniCase<String>, String>::new();
    let mut paths = PathSet::new();
    head_tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let name = entry.name().unwrap();
            let slash_path = root.to_string() + name;
            // FIXME: bring back case_insensitive_paths
            // TODO: also check if entry already existed here
            //case_insensitive_paths.insert(slash_path.clone().into(), slash_path.clone());
            paths.insert(slash_path);
        }
        git2::TreeWalkResult::Ok
    })?;

    // TODO[LATER]: maybe check if git status clean at script.shadow_dir

    // TODO[LATER]: validate that paths were not already added (and do it case insensitively)
    // TODO[LATER]: allow case-sensitive check with an explicit CLI flag
    let dir = Dir::open_ambient_dir(script.shadow_dir, ambient_authority())?;
    for (path, contents) in script.paths {
        println!(" - {path}");
        // TODO[LATER]: try if things will "just work" on Windows without explicit from_slash conversions
        let os_path = PathBuf::from_slash(&path);
        if let Some(parent) = parent_dir(&os_path) {
            dir.create_dir_all(parent).context("in shadow_dir")?;
        }
        dir.write(&path, contents).context("in shadow_dir")?;

        paths.remove(&path);
    }

    // Delete files found on disk but not found in script
    for path in &paths {
        dir.remove_file(path);
    }

    // TODO[LATER]: add support for binary files, maybe somehow

    Ok(())
}

fn apply(script: Script) -> Result<()> {
    // open repo and verify it has no pending operation
    let repo = Repository::open(&script.shadow_dir).context("failure opening 'shadow_dir'")?;
    if repo.state() != git2::RepositoryState::Clean {
        bail!(
            "git 'shadow_dir' repository has pending unfinished operation {:?}",
            repo.state()
        );
    }

    // Initialize handlers
    let lua = Lua::new();
    let lua_handlers = lua.create_table().unwrap();
    let mut rust_handlers = RustHandlers {
        map: BTreeMap::new(),
    };
    for (root, cmd) in script.handlers {
        if let Ok(_) = init_lua_handler(&lua, &lua_handlers, root.clone(), cmd.clone()) {
            continue;
        }
        match &cmd[..] {
            [s] if s == "zeroinstall" => {
                rust_handlers
                    .map
                    .insert(root, Box::new(zeroinstall::Handler::new()?));
            }
            _ => {
                bail!("unknown handler command: {cmd:?}");
            }
        }
    }

    // iterate modified files in repo, incl. untracked
    // TODO: also iterate unmodified?
    let mut git_index = repo.index()?;
    let mut stat_opt = git2::StatusOptions::new();
    stat_opt.include_untracked(true);
    stat_opt.recurse_untracked_dirs(true);
    // stat_opt.include_unmodified(true);
    for stat in &repo.statuses(Some(&mut stat_opt))? {
        println!(" * {:?}", stat.path());
        let Some(path) = stat.path() else {
            bail!(
                "Path from 'git status' cannot be parsed as utf8: {:?}",
                stat.path()
            );
        };
        let os_rel_path = PathBuf::from_slash(path);
        let shadow_path = PathBuf::from(&script.shadow_dir).join(&os_rel_path);
        let (prefix, subpath) = split_handler_path(&path);
        let rust = rust_handlers.maybe_affect(
            &prefix,
            &PathBuf::from_slash(subpath),
            &PathBuf::from_slash(&script.shadow_dir),
        );
        if let Some(rs) = rust {
            let _ = rs?;
        } else {
            call_handler_method(
                &lua_handlers,
                prefix,
                "apply",
                (subpath, shadow_path.to_str().unwrap()),
            )
            .with_context(|| format!("calling handlers[{prefix:?}]:apply({subpath:?})"))?;
        }
        use git2::Status;
        match stat.status() {
            Status::WT_NEW | Status::WT_MODIFIED => {
                git_index.add_path(&os_rel_path)?;
            }
            Status::WT_DELETED => {
                git_index.remove_path(&os_rel_path)?;
            }
            s @ _ => {
                bail!("unsupported git status {s:?} for path {path:?} in 'shadow_dir'");
            }
        }
        git_index.write()?;
    }

    Ok(())
}

type PathSet = BTreeSet<String>;

fn check_git_statuses_empty(repo: &Repository) -> Result<bool> {
    let mut stat_opt = git2::StatusOptions::new();
    stat_opt.include_untracked(true);
    let stat = repo.statuses(Some(&mut stat_opt))?;
    Ok(stat.is_empty())
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

fn parent_dir(path: &Path) -> Option<&Path> {
    // can we simplify this somehow?
    path.parent().filter(|p| *p != Path::new(""))
}

fn split_handler_path(path: &str) -> (&str, &str) {
    let Some(idx) = path.find('/') else {
        panic!("slash not found in path: {path:?}");
    };
    let (start, rest) = path.split_at(idx);
    let (_slash, end) = rest.split_at(1);
    return (start, end);
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

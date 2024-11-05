use anyhow::{bail, Context, Result};
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use clap::{Parser, Subcommand};
use git2::Repository;
use log::debug;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
// Trait for extending std::path::PathBuf
use path_slash::PathBufExt as _;
use unicase::UniCase;

use script::Script;

use care::effectors::{self, Effectors};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to a file containing a Nickel script to evaluate.
    #[arg(short, long, default_value = "care.ncl")]
    ncl: PathBuf,

    /// Turn debugging information on.
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check actual state of the machine and serialize it into git
    /// working directory at 'shadow_dir'.
    Check,
    /// Serialize desired state (as read from input) into git working
    /// directory at 'shadow_dir'.
    Draft,
    /// Apply the contents of the git working directory to the state
    /// of the machine. For each successfully applied file, perform
    /// `git add` on it.
    Apply,
}

fn main() -> Result<()> {
    // Handle hidden (internal) subcommand: `effector`.
    // If used, pass all subsequent args to it.
    {
        let mut args = std::env::args();
        if args.nth(1).as_deref() == Some("effector") {
            effectors::serve(args)?;
            return Ok(());
        }
    }

    let cli = Cli::parse();

    let log_level = match cli.debug {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        2.. => log::LevelFilter::Trace,
    };
    env_logger::Builder::new().filter_level(log_level).init();
    debug!("Hello, world!");

    println!("care: Processing Nickel script");
    let script = Script::parse_ncl_file(&cli.ncl)?;
    script.validate()?;
    match &cli.command {
        Command::Check => check(script),
        Command::Draft => draft(script),
        Command::Apply => apply(script),
    }

    // TODO[LATER]: licensing information in --license flag
}

// open repo and verify it has no pending operation
fn open_shadow_repo(script: &Script) -> Result<Repository> {
    let repo = Repository::open(&script.shadow_dir).context("failure opening 'shadow_dir'")?;
    if repo.state() != git2::RepositoryState::Clean {
        bail!(
            "git 'shadow_dir' repository has pending unfinished operation {:?}",
            repo.state()
        );
    }
    Ok(repo)
}

fn check(script: Script) -> Result<()> {
    println!("care: Opening shadow repository");
    let repo = open_shadow_repo(&script)?;
    // check if repo is clean
    if !check_git_statuses_empty(&repo, &script.ignores)? {
        bail!("git 'shadow_dir' repository is not clean (see: git status)");
    }

    // Initialize effectors
    println!("care: Starting effectors:");
    let mut effectors = Effectors::init(&script.effectors)?;

    // Make a list of paths in 'tree' and in git
    println!("care: Collecting paths in git");
    let head = repo.head()?;
    let head_tree = head.peel_to_tree()?;
    let mut paths = PathSet::new();
    // TODO: unicode normaliz.: https://stackoverflow.com/q/47813162/#comment82595250_47813878
    let mut case_insensitive_paths = std::collections::HashMap::<UniCase<String>, String>::new();
    head_tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let name = entry.name().unwrap();
            let slash_path = root.to_string() + name;
            if script.ignores_path(&slash_path) {
                return git2::TreeWalkResult::Skip;
            }
            // TODO: also check if entry already existed here
            case_insensitive_paths.insert(slash_path.clone().into(), slash_path.clone());
            paths.insert(slash_path);
        }
        git2::TreeWalkResult::Ok
    })?;
    // for k in &paths {
    //     println!(" - {k:?}");
    // }
    println!("care: Collecting paths in script");
    for path in script.paths.keys() {
        if script.ignores_path(&path) {
            bail!("Path {path:?} from script matches an ignored prefix");
        }
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
        debug!(" - {k:?}");
    }

    // Run 'check' on appropriate effectors for all listed paths, fetching files into the git workspace
    println!("care: Checking:");
    let dir = Dir::open_ambient_dir(&script.shadow_dir, ambient_authority())?;
    for path in &paths {
        if let Some(parent) = parent_dir(&PathBuf::from_slash(path)) {
            dir.create_dir_all(parent).context("in shadow_dir")?;
        }
        let (prefix, subpath) = split_effector_path(path);
        println!("care:   {prefix}: {subpath}");
        let found = effectors.detect(prefix, subpath)?;
        let shadow_path = PathBuf::from(&script.shadow_dir).join(PathBuf::from_slash(path));
        if !found {
            let removal = std::fs::remove_file(shadow_path);
            if let Err(ref err) = removal {
                if err.kind() == std::io::ErrorKind::NotFound {
                    continue;
                }
            }
            _ = removal?;
            continue;
        }
        effectors.gather(prefix, subpath, &script.shadow_dir)?;
    }

    // Two-way compare: current git <-> results of effectors.query
    if !check_git_statuses_empty(&repo, &script.ignores)? {
        bail!(
            "real disk contents differ from expected prerequisites; check git diff in shadow repo: {:?}", script.shadow_dir,
        );
    }

    // TODO: 3-way compare: curr git <-> effectors.query results <-> parsed input
    // TODO: https://github.com/akavel/drafts/blob/main/20231122-001-mana2.md
    Ok(())
}

fn draft(script: Script) -> Result<()> {
    // Make a list of paths in git
    println!("care: Opening shadow repository");
    let repo = open_shadow_repo(&script)?;
    println!("care: Collecting paths in git");
    let head = repo.head()?;
    let head_tree = head.peel_to_tree()?;
    // TODO: unicode normaliz.: https://stackoverflow.com/q/47813162/#comment82595250_47813878
    //let mut case_insensitive_paths = std::collections::HashMap::<UniCase<String>, String>::new();
    let mut paths = PathSet::new();
    head_tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let name = entry.name().unwrap();
            let slash_path = root.to_string() + name;
            if script.ignores_path(&slash_path) {
                return git2::TreeWalkResult::Skip;
            }
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
    let dir = Dir::open_ambient_dir(script.shadow_dir.clone(), ambient_authority())?;
    println!("care: Processing paths in script");
    for (path, contents) in &script.paths {
        debug!(" - {path}");
        if script.ignores_path(&path) {
            bail!("Path {path:?} from script matches an ignored prefix");
        }
        // TODO[LATER]: try if things will "just work" on Windows without explicit from_slash conversions
        let os_path = PathBuf::from_slash(&path);
        if let Some(parent) = parent_dir(&os_path) {
            dir.create_dir_all(parent).context("in shadow_dir")?;
        }
        dir.write(&path, contents).context("in shadow_dir")?;

        paths.remove(path);
    }

    // Delete files found on disk but not found in script
    for path in &paths {
        dir.remove_file(path)?;
    }

    // TODO[LATER]: add support for binary files, maybe somehow

    Ok(())
}

fn apply(script: Script) -> Result<()> {
    println!("care: Opening shadow repository");
    let repo = open_shadow_repo(&script)?;

    // Initialize effectors
    println!("care: Starting effectors:");
    let mut effectors = Effectors::init(&script.effectors)?;

    // iterate modified files in repo, incl. untracked
    // TODO: also iterate unmodified?
    println!("care: Collecting pending paths in git");
    let mut git_index = repo.index()?;
    let mut stat_opt = git2::StatusOptions::new();
    stat_opt.include_untracked(true);
    stat_opt.recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut stat_opt))?;
    // stat_opt.include_unmodified(true);
    println!("care: Affecting:");
    for stat in &statuses {
        let Some(path) = stat.path() else {
            bail!(
                "Path from 'git status' cannot be parsed as utf8: {:?}",
                stat.path()
            );
        };
        if script.ignores_path(path) {
            debug!(" ^ {:?}", path);
            continue;
        }
        debug!(" * {:?}", path);
        let os_rel_path = PathBuf::from_slash(path);
        let (prefix, subpath) = split_effector_path(path);
        println!("care:   {prefix}: {subpath}");
        effectors.affect(prefix, subpath, &script.shadow_dir)?;
        use git2::Status;
        match stat.status() {
            Status::WT_NEW | Status::WT_MODIFIED => {
                git_index.add_path(&os_rel_path)?;
            }
            Status::WT_DELETED => {
                git_index.remove_path(&os_rel_path)?;
            }
            s => {
                bail!("unsupported git status {s:?} for path {path:?} in 'shadow_dir'");
            }
        }
        git_index.write()?;
    }

    Ok(())
}

type PathSet = BTreeSet<String>;

fn check_git_statuses_empty(
    repo: &Repository,
    ignores: impl IntoIterator<Item: AsRef<str>>,
) -> Result<bool> {
    let mut stat_opt = git2::StatusOptions::new();
    stat_opt.include_untracked(true);
    for ign in ignores {
        stat_opt.pathspec("/".to_owned() + ign.as_ref());
    }
    let stat = repo.statuses(Some(&mut stat_opt))?;
    Ok(stat.is_empty())
}

fn parent_dir(path: &Path) -> Option<&Path> {
    // can we simplify this somehow?
    path.parent().filter(|p| *p != Path::new(""))
}

fn split_effector_path(path: &str) -> (&str, &str) {
    let Some(idx) = path.find('/') else {
        panic!("slash not found in path: {path:?}");
    };
    let (start, rest) = path.split_at(idx);
    let (_slash, end) = rest.split_at(1);
    (start, end)
}

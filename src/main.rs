use anyhow::{bail, Context, Result};
use git2::Repository;

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

    // TODO: make a list of paths in 'tree' and in git
    let head = repo.head()?;
    let head_tree = head.peel_to_tree()?;
    head_tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        println!(
            " - {root}{}  {:?}",
            entry.name().unwrap_or("?"),
            entry.kind()
        );
        git2::TreeWalkResult::Ok
    })?;
    // TODO: run 'gather' on appropriate handlers for all listed paths, fetching files into the git workspace

    // TODO: two-way compare: current git <-> results of handlers.gather (use git-workspace)
    // TODO: 3-way compare: curr git <-> handlers.query results <-> parsed input
    // TODO: https://github.com/akavel/drafts/blob/main/20231122-001-mana2.md
    Ok(())
}

#[derive(Debug)]
struct Script {
    pub shadow_dir: String,
    pub handlers: toml::Table,
    pub tree: toml::Table,
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
    let Some(handlers) = toml.remove("handlers") else {
        bail!("Missing 'handlers' in stdin");
    };
    let toml::Value::Table(handlers) = handlers else {
        bail!("Expected 'handlers' to be table, got: {handlers:?}");
    };
    println!("HANDL: {handlers:?}");

    // Extract `tree` from toml
    // TODO[LATER]: use serde instead to extract, maybe
    let Some(tree) = toml.remove("tree") else {
        bail!("Missing 'tree' in stdin");
    };
    let toml::Value::Table(tree) = tree else {
        bail!("Expected 'tree' to be table, got: {tree:?}");
    };
    Ok(Script {
        shadow_dir,
        handlers,
        tree,
    })
}

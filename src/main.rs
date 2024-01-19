use anyhow::{bail, Context, Result};
use git2::Repository;

fn main() -> Result<()> {
    println!("Hello, world!");
    // Read and parse input - just parse TOML for now.
    // TODO: would prefer to somehow do it in streamed way, maybe
    let input = std::io::read_to_string(std::io::stdin()).context("failure reading stdin")?;
    let script = parse_input_toml(&input)?;

    // TODO: create temporary git worktree
    let repo = Repository::open(&script.shadow_dir)
        .context("failure opening 'shadow_dir'")?;
    // let head_commit = repo.refname_to_id("HEAD")?;

    let head_ref = repo.find_reference("HEAD")?;
    let work_dir = tempfile::tempdir()?;
    let work_subdir = work_dir.path().join("tree");
    // TODO[LATER]: would prefer something that doesn't litter $shadow_dir/.git/ dir
    let mut worktree_opt = git2::WorktreeAddOptions::new();
    worktree_opt.reference(Some(&head_ref));
    // Workaround to try ensuring that created branch is random yet a valid branch name
    // TODO: would prefer to checkout in --detached mode with no branch creation
    let branch = "mana".to_string() + work_dir.path().file_name().unwrap().to_str().unwrap();
    // let work_tree = repo.worktree(work_dir.path().file_name().unwrap().to_str().unwrap(), &work_subdir, Some(&worktree_opt))
    // let work_tree = repo.worktree("", &work_subdir, Some(&worktree_opt))
    let work_tree = repo.worktree(&branch, &work_subdir, None)
        .context("failure initializing git worktree")?;
    println!("WORK: {:?}", &work_subdir);
    std::mem::forget(work_dir);

    // TODO: make a list of paths in 'tree' and in git
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

use anyhow::{bail, Context};

fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    // Parse input - just parse TOML for now.
    // TODO: would prefer to somehow do it in streamed way, maybe
    let input = std::io::read_to_string(std::io::stdin()).context("failure reading stdin")?;
    let toml = input
        .parse::<toml::Table>()
        .context("failed to parse stdin as TOML")?;
    // println!("PARSED: {toml:?}");

    // Extract `shadow_dir` from toml
    // TODO[LATER]: use serde instead to extract, maybe
    let Some(shadow_dir) = toml.get("shadow_dir") else {
        bail!("Missing 'shadow_dir' in stdin");
    };
    let toml::Value::String(shadow_dir) = shadow_dir else {
        bail!("Expected 'shadow_dir' to be text, got: {shadow_dir:?}");
    };
    println!("SHAD: {shadow_dir:?}");

    // Extract `handlers` from toml
    // TODO[LATER]: use serde instead to extract, maybe
    let Some(handlers) = toml.get("handlers") else {
        bail!("Missing 'handlers' in stdin");
    };
    let toml::Value::Table(handlers) = handlers else {
        bail!("Expected 'handlers' to be table, got: {handlers:?}");
    };
    println!("HANDL: {handlers:?}");

    // Extract `tree` from toml
    // TODO[LATER]: use serde instead to extract, maybe
    let Some(tree) = toml.get("tree") else {
        bail!("Missing 'tree' in stdin");
    };
    let toml::Value::Table(tree) = tree else {
        bail!("Expected 'tree' to be table, got: {tree:?}");
    };

    // TODO: two-way compare: current git <-> results of handlers.gather (use git-workspace)
    // TODO: 3-way compare: curr git <-> handlers.query results <-> parsed input
    // TODO: https://github.com/akavel/drafts/blob/main/20231122-001-mana2.md
    Ok(())
}

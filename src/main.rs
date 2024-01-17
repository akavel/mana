use anyhow::Context;

fn main() -> anyhow::Result<()> {
    println!("Hello, world!");
    // Parse input - just parse TOML for now.
    // TODO: would prefer to somehow do it in streamed way, maybe
    let input = std::io::read_to_string(std::io::stdin()).context("failure reading stdin")?;
    let toml = input
        .parse::<toml::Table>()
        .context("failed to parse stdin as TOML")?;
    // println!("PARSED: {toml:?}");

    // TODO: two-way compare: current git <-> results of handlers.gather (use git-workspace)
    // TODO: 3-way compare: curr git <-> handlers.query results <-> parsed input
    // TODO: https://github.com/akavel/drafts/blob/main/20231122-001-mana2.md
    Ok(())
}

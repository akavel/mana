use anyhow::{bail, Context, Result};
use clap::Parser;

use std::path::{Path, PathBuf};

#[derive(Parser)]
struct Cli {
    #[arg(env="XSSH_HOST")]
    host: String,
    #[arg(env="XSSH_USER")]
    user: String,
    #[arg(env="XSSH_KEY")]
    key_path: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    //FIXME: try connecting to rpi4
    //FIXME: try reading home dir /home/pi
    Ok(())
}


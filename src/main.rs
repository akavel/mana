use anyhow::{bail, Context, Result};
use clap::Parser;
use remotefs::RemoteFs;
use remotefs_ssh::{ScpFs, SshOpts};

use std::path::{Path, PathBuf};

#[derive(Parser)]
struct Cli {
    #[arg(env = "XSSH_HOST")]
    host: String,
    #[arg(env = "XSSH_USER")]
    user: String,
    #[arg(env = "XSSH_KEY")]
    key_path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    //FIXME: try connecting to rpi4
    let mut client: ScpFs = SshOpts::new(&cli.host)
        .username(&cli.user)
        .key_storage(Box::new(SshKeyPath { path: cli.key_path }))
        .into();
    println!("auth? {:?}", client.connect());

    //FIXME: try reading home dir /home/pi
    let mut h = crate::Handler { client };
    let f = Path::new("/home/pi/xssh-test");
    use crate::callee::Handler;
    println!("detect? {f:?} = {:?}", h.detect(f));

    Ok(())
}

#[derive(Clone)]
struct SshKeyPath {
    path: PathBuf,
}

impl remotefs_ssh::SshKeyStorage for SshKeyPath {
    fn resolve(&self, _host: &str, _username: &str) -> Option<PathBuf> {
        Some(self.path.clone())
    }
}

pub mod callee {
    use anyhow::Result;
    use std::path::Path;

    pub trait Handler {
        fn detect(&mut self, path: &Path) -> Result<bool>;
        // fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;
        // fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;
    }
}

pub struct Handler {
    client: ScpFs,
}

impl callee::Handler for Handler {
    fn detect(&mut self, path: &Path) -> Result<bool> {
        Ok(self.client.exists(path)?)
    }
}

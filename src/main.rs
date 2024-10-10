use anyhow::Result;
use clap::Parser;
use fn_error_context::context;
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
    // let f = Path::new("/home/pi/xssh-test");
    let f = Path::new("xssh-test");
    use crate::callee::Handler;
    println!("detect? {f:?} = {:?}", h.detect(f));

    let pid = format!("{}", std::process::id());
    let temp = std::env::temp_dir().join(&pid);
    println!("gather... {f:?} into {temp:?}");
    let shadow_f = temp.join(f);
    let parent = shadow_f.parent().unwrap();
    std::fs::create_dir_all(parent)?;
    h.gather(f, parent)?;

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
        fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;
        // fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;
    }
}

pub struct Handler {
    client: ScpFs,
}

impl callee::Handler for Handler {
    #[context("detecting SSH {path:?}")]
    fn detect(&mut self, path: &Path) -> Result<bool> {
        Ok(self.client.exists(path)?)
    }

    #[context("gathering SSH {path:?} to {shadow_prefix:?}")]
    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        let mut r = self.client.open(path)?;
        let mut f = std::fs::File::create(shadow_prefix.join(path))?;
        std::io::copy(&mut r, &mut f)?;
        Ok(())
    }
}

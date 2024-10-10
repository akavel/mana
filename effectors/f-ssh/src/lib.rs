use anyhow::Result;
use fn_error_context::context;
use remotefs::RemoteFs;
use remotefs_ssh::{ScpFs, SshOpts};

use std::fs::File;
use std::path::{Path, PathBuf};

use effectors::callee;

pub struct Args {
    host: String,
    user: String,
    key_path: PathBuf,
    // TODO: add base_dir
}

pub struct Effector {
    client: ScpFs,
}

impl Effector {
    #[context("creating SSH effector for {}@{}, key {:?}", &args.user, &args.host, &args.key_path)]
    pub fn new(args: Args) -> Result<Self> {
        let mut client: ScpFs = SshOpts::new(&args.host)
            .username(&args.user)
            .key_storage(Box::new(SshKeyPath { path: args.key_path.clone() }))
            .into();
        client.connect()?;
        Ok(Self{ client })
    }
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

impl callee::Handler for Effector {
    #[context("detecting SSH {path:?}")]
    fn detect(&mut self, path: &Path) -> Result<bool> {
        Ok(self.client.exists(path)?)
    }

    #[context("gathering SSH {path:?} to {shadow_prefix:?}")]
    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        let mut r = self.client.open(path)?;
        let mut w = File::create(shadow_prefix.join(path))?;
        std::io::copy(&mut r, &mut w)?;
        Ok(())
    }

    #[context("affecting {path:?} to SSH {shadow_prefix:?}")]
    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        let mut r = File::open(shadow_prefix.join(path))?;
        let meta = r.metadata()?;
        let mut w = self.client.create(path, &meta.into())?;
        std::io::copy(&mut r, &mut w)?;
        Ok(())
    }
}


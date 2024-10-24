use anyhow::{bail, Result};
use clap::Parser;
use fn_error_context::context;
use remotefs::RemoteFs;
use remotefs_ssh::{ScpFs, SshAgentIdentity, SshOpts};

use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long)]
    host: String,
    #[arg(long)]
    user: String,
    #[arg(long)]
    key_path: Option<PathBuf>,
    #[arg(long)]
    key_agent: bool,
    #[arg(long)]
    base_dir: Option<PathBuf>,
}

pub struct Effector {
    client: ScpFs,
    base_dir: Option<PathBuf>,
}

impl Effector {
    #[context("creating *scp effector for {}@{}, key {:?}", &args.user, &args.host, &args.key_path)]
    pub fn new(args: Args) -> Result<Self> {
        let opts = SshOpts::new(&args.host).username(&args.user);
        let opts = match (&args.key_agent, &args.key_path) {
            (false, None) | (true, Some(..)) => {
                bail!("*scp requires exactly one of: --key-path OR --key-agent")
            }
            (false, Some(path)) => opts.key_storage(Box::new(SshKeyPath { path: path.clone() })),
            (true, None) => opts.ssh_agent_identity(Some(SshAgentIdentity::All)),
        };
        let mut client = ScpFs::from(opts);
        client.connect()?;
        Ok(Self {
            client,
            base_dir: args.base_dir.clone(),
        })
    }

    fn based(&self, path: &Path) -> PathBuf {
        match &self.base_dir {
            None => path.to_path_buf(),
            Some(dir) => dir.join(path),
        }
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

impl effectors::Callee for Effector {
    fn start(args: std::env::Args) -> Result<Self> {
        let dummy_arg0 = Some("".to_string());
        let args = Args::parse_from(dummy_arg0.into_iter().chain(args));
        Effector::new(args)
    }

    #[context("*scp detecting {path:?}")]
    fn detect(&mut self, path: &Path) -> Result<bool> {
        Ok(self.client.exists(&self.based(path))?)
    }

    #[context("*scp gathering {path:?} to {shadow_prefix:?}")]
    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        let mut r = self.client.open(&self.based(path))?;
        let mut w = File::create(shadow_prefix.join(path))?;
        std::io::copy(&mut r, &mut w)?;
        Ok(())
    }

    #[context("*scp affecting {path:?} to {shadow_prefix:?}")]
    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        let maybe_r = File::open(shadow_prefix.join(path));
        // Handle file-not-found scenario - remove remote file
        // TODO: merge two ifs once let-chains are stabilized
        if let Err(ref err) = maybe_r {
            if err.kind() == ErrorKind::NotFound {
                self.client.remove_file(&self.based(path))?;
                return Ok(());
            }
        }
        let mut r = maybe_r?;

        let meta = r.metadata()?;
        let mut w = self.client.create(&self.based(path), &meta.into())?;
        std::io::copy(&mut r, &mut w)?;
        Ok(())
    }
}

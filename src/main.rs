use anyhow::{bail, Context, Result};
use clap::Parser;
use ssh2::Session;

use std::net::TcpStream;
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
    let tcp = TcpStream::connect((cli.host.as_str(), 22))?;
    let mut sess = Session::new()?;
    sess.set_tcp_stream(tcp);
    sess.handshake()?;
    sess.userauth_pubkey_file(&cli.user, None, &cli.key_path, None)?;

    println!("auth? {}", sess.authenticated());

    //FIXME: try reading home dir /home/pi
    let mut h = crate::Handler { sess };
    let f = Path::new("/home/pi/xssh-test");
    use crate::callee::Handler;
    println!("detect? {f:?} = {:?}", h.detect(&f));

    Ok(())
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
    sess: Session,
}

impl callee::Handler for Handler {
    fn detect(&mut self, path: &Path) -> Result<bool> {
        let (chan, stat) = self.sess.scp_recv(path)?;
        Ok(stat.is_file())
    }
}

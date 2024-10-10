use anyhow::{bail, Context, Result};
use clap::Parser;
use ssh2::Session;

use std::path::{Path, PathBuf};
use std::net::TcpStream;

#[derive(Parser)]
struct Cli {
    #[arg(env="XSSH_HOST")]
    host: String,
    #[arg(env="XSSH_USER")]
    user: String,
    #[arg(env="XSSH_KEY")]
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
    Ok(())
}


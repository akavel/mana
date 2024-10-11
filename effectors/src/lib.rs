use anyhow::Result;

use std::path::Path;

pub trait Callee {
    fn start(args: std::env::Args) -> Result<Self> where Self: Sized;

    fn detect(&mut self, path: &Path) -> Result<bool>;
    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;
    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;

    fn serve(args: std::env::Args) -> Result<()> where Self: Sized {
        let mut c = Self::start(args)?;

        use std::io::{BufRead, Write};
        let mut in_lines = std::io::stdin().lock().lines();
        let mut out = std::io::stdout().lock();

        // Handshake
        use anyhow::{anyhow, bail};
        let handshake = in_lines.next().ok_or(anyhow!("expected handshake, got EOF on stdin"))??;
        // let Some(handshake) = in_lines.next() else {
        //     bail!("expected handshake, got EOF on stdin");
        // };
        // let handshake = handshake?;
        if !handshake.starts_with("com.akavel.mana.v1.rq") {
            bail!("expected v1 handshake, got: {handshake:?}");
        }
        writeln!(out, "com.akavel.mana.v1.rs")?;
        out.flush()?;

        loop {
            let Some(line) = in_lines.next() else {
                return Ok(());
            };
            let line = line?;
            let Some((cmd, args)) = line.split_once(' ') else {
                bail!("expected command with args, got: {line:?}");
            };
            let mut args = args.split(' ');
            use std::path::PathBuf;
            use std::str::FromStr;
            match cmd {
                "detect" => {
                    let Some(arg1) = args.next() else {
                        bail!("expected 1 arg to 'detect', got none");
                    };
                    // FIXME: add error context pointing to 1st arg of 'detect'
                    let arg1 = urlencoding::decode(arg1)?;
                    // TODO: should we verify path is relative? or absolute?
                    // TODO: should/can this be simplified?
                    let path = PathBuf::from_str(&arg1)?;
                    let res = c.detect(&path)?;
                    writeln!(out, "detected {} {}",
                             // TODO: use path instead? (normalized?)
                             urlencoding::encode(&arg1),
                             if res { "present" } else {"absent"})?;
                    out.flush()?;
                }
                _ => bail!("unknown command: {cmd:?}"),
            }
        }
    }
}

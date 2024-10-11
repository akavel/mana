use anyhow::Result;

use std::path::Path;

pub trait Callee {
    fn start(args: std::env::Args) -> Result<Self>
    where
        Self: Sized;

    fn detect(&mut self, path: &Path) -> Result<bool>;
    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;
    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;

    fn serve(args: std::env::Args) -> Result<()>
    where
        Self: Sized,
    {
        use anyhow::{anyhow, bail};
        use itertools::Itertools;
        use std::io::{BufRead, Write};
        use std::path::PathBuf;
        use std::str::FromStr;

        let mut c = Self::start(args)?;
        let mut in_lines = std::io::stdin().lock().lines();
        let mut out = std::io::stdout().lock();

        // Handshake
        let handshake = in_lines
            .next()
            .ok_or(anyhow!("expected handshake, got EOF on stdin"))??;
        if !handshake.starts_with("com.akavel.mana.v1.rq") {
            bail!("expected v1 handshake, got: {handshake:?}");
        }
        writeln!(out, "com.akavel.mana.v1.rs")?;
        out.flush()?;

        // Dispatch commands to appropriate trait functions
        loop {
            let Some(line) = in_lines.next() else {
                return Ok(());
            };
            let line = line?;

            let Some((cmd, args)) = line.split_once(' ') else {
                bail!("expected command with args, got: {line:?}");
            };
            let mut args = args.split(' ').map(urlencoding::decode);
            match cmd {
                "detect" => {
                    let Some(arg1) = args.next() else {
                        bail!("expected 1 arg to 'detect', got none");
                    };
                    let res = c.detect(&PathBuf::from_str(&arg1?)?)?;
                    writeln!(out, "detected {}", if res { "present" } else { "absent" })?;
                }
                "gather" => {
                    let Some((arg1, arg2)) = args.next_tuple() else {
                        bail!("expected 2 args to 'gather', got less");
                    };
                    c.gather(&PathBuf::from_str(&arg1?)?, &PathBuf::from_str(&arg2?)?)?;
                    writeln!(out, "gathered")?;
                }
                "affect" => {
                    let Some((arg1, arg2)) = args.next_tuple() else {
                        bail!("expected 2 args to 'affect', got less");
                    };
                    c.affect(&PathBuf::from_str(&arg1?)?, &PathBuf::from_str(&arg2?)?)?;
                    writeln!(out, "affected")?;
                }
                _ => bail!("unknown command: {cmd:?}"),
            }
            out.flush()?;
        }
    }
}

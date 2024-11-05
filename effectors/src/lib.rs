use anyhow::Result;

use std::path::{Path, PathBuf};

pub const HANDSHAKE_RQ: &str = "com.akavel.care.v2.rq";
pub const HANDSHAKE_RS: &str = "com.akavel.care.v2.rs";

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

        let mut c = Self::start(args)?;
        let mut in_lines = std::io::stdin().lock().lines();
        let mut out = std::io::stdout().lock();

        // Handshake
        let handshake = in_lines
            .next()
            .ok_or(anyhow!("expected handshake, got EOF on stdin"))??;
        if !handshake.starts_with(HANDSHAKE_RQ) {
            bail!("expected v2 handshake, got: {handshake:?}");
        }
        writeln!(out, "{}", HANDSHAKE_RS)?;
        out.flush()?;

        // Dispatch commands to appropriate trait functions
        loop {
            let Some(line) = in_lines.next().transpose()? else {
                return Ok(());
            };
            let Some((cmd, args)) = line.split_once(' ') else {
                bail!("expected command with args, got: {line:?}");
            };
            let mut args = args.split(' ').map(urldecode_to_path);
            match cmd {
                "detect" => {
                    let Some(path) = args.next() else {
                        bail!("expected 1 arg to 'detect', got none");
                    };
                    let res = c.detect(&path?)?;
                    writeln!(out, "detected {}", if res { "present" } else { "absent" })?;
                }
                "gather" => {
                    let Some((path1, path2)) = args.next_tuple() else {
                        bail!("expected 2 args to 'gather', got less");
                    };
                    c.gather(&path1?, &path2?)?;
                    writeln!(out, "gathered")?;
                }
                "affect" => {
                    let Some((path1, path2)) = args.next_tuple() else {
                        bail!("expected 2 args to 'affect', got less");
                    };
                    c.affect(&path1?, &path2?)?;
                    writeln!(out, "affected")?;
                }
                _ => bail!("unknown command: {cmd:?}"),
            }
            out.flush()?;
        }
    }
}

fn urldecode_to_path(s: &str) -> Result<PathBuf> {
    use std::str::FromStr;
    let decoded = urlencoding::decode(s)?;
    let path = PathBuf::from_str(&decoded)?;
    Ok(path)
}

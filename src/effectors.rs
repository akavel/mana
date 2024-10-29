use anyhow::{anyhow, bail, Result};
use fn_error_context::context;
use path_slash::PathBufExt as _;
use phf::phf_set;

use std::collections::BTreeMap;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process;

use script::Effectors as Spec;

pub fn serve(mut args: std::env::Args) -> Result<()> {
    let Some(name) = args.next() else {
        bail!("subcommand 'effector' requires name of effector");
    };
    use effectors::Callee;
    match name.as_str() {
        "*lua" => f_lua::Effector::serve(args),
        "*scp" => f_scp::Effector::serve(args),
        "*zeroinstall" => f_zeroinstall::Effector::serve(args),
        _ => Err(anyhow!("unknown effector name: {name:?}")),
    }
}

static EFFECTORS: phf::Set<&'static str> = phf_set! {
    "*lua",
    "*scp",
    "*zeroinstall",
};

type ChildProcs = BTreeMap<String, ChildProc>;

pub struct ChildProc {
    pub proc: process::Child,
    pub buf_out: BufReader<process::ChildStdout>,
}

impl ChildProc {
    #[context("spawning effector {name}")]
    pub fn new_effector(name: &str, args: &[String]) -> Result<Self> {
        let arg0 = std::env::args().next().unwrap();
        let args = ["effector", name]
            .into_iter()
            .chain(args.iter().map(|s| s.as_ref()));
        let mut proc = process::Command::new(arg0)
            .args(args)
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::inherit())
            .spawn()?;
        let buf_out = BufReader::new(proc.stdout.take().unwrap());
        let mut child = ChildProc { proc, buf_out };
        {
            let mut child_in = child.proc.stdin.as_ref().unwrap();
            // TODO: print error details in case of error
            writeln!(child_in, "{}", effectors::HANDSHAKE_RQ)?;
            child_in.flush()?;
            let rs = child.read_line()?;
            if rs.as_str().trim_end() != effectors::HANDSHAKE_RS {
                bail!("expected v2 handshake from {name}, got: {rs:?}");
            }
        }
        Ok(child)
    }

    pub fn read_line(&mut self) -> Result<String> {
        use std::io::BufRead;
        let mut buf = String::new();
        self.buf_out.read_line(&mut buf)?;
        Ok(buf)
    }

    pub fn detect(&mut self, path: &Path) -> Result<bool> {
        let mut child_in = self.proc.stdin.as_ref().unwrap();
        writeln!(
            child_in,
            "detect {}",
            urlencoding::encode(path.to_str().unwrap())
        )?;
        let rs = self.read_line()?;
        match rs.trim_end() {
            "detected present" => Ok(true),
            "detected absent" => Ok(false),
            _ => bail!("unexpected 'detect' response: {:?}", rs),
        }
    }

    pub fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        use urlencoding::encode;
        let mut child_in = self.proc.stdin.as_ref().unwrap();
        writeln!(
            child_in,
            "gather {} {}",
            encode(path.to_str().unwrap()),
            encode(shadow_prefix.to_str().unwrap())
        )?;
        let rs = self.read_line()?;
        if !rs.starts_with("gathered") {
            bail!("unexpected 'gather' response: {:?}", rs);
        }
        Ok(())
    }

    pub fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        use urlencoding::encode;
        let mut child_in = self.proc.stdin.as_ref().unwrap();
        writeln!(
            child_in,
            "affect {} {}",
            encode(path.to_str().unwrap()),
            encode(shadow_prefix.to_str().unwrap())
        )?;
        let rs = self.read_line()?;
        if !rs.starts_with("affected") {
            bail!("unexpected 'affect' response: {:?}", rs);
        }
        Ok(())
    }
}

pub struct Effectors {
    child_procs: ChildProcs,
}

impl Effectors {
    pub fn init(spec: &Spec) -> Result<Effectors> {
        let mut child_procs = ChildProcs::new();
        for (root, cmd) in spec {
            match &cmd[..] {
                [s, args @ ..] if EFFECTORS.contains(s) => {
                    // TODO[LATER]: check no duplicates
                    child_procs.insert(root.clone(), ChildProc::new_effector(s, args)?);
                }
                _ => {
                    bail!("unknown effector command: {cmd:?}");
                }
            }
        }
        Ok(Self { child_procs })
    }

    #[context("detecting at {prefix}/{subpath}")]
    pub fn detect(&mut self, prefix: &str, subpath: &str) -> Result<bool> {
        self.for_prefix(prefix)?
            .detect(&PathBuf::from_slash(subpath))
    }

    #[context("gathering at {prefix}/{subpath}")]
    pub fn gather(&mut self, prefix: &str, subpath: &str, shadow_root: &Path) -> Result<()> {
        let subpath = &PathBuf::from_slash(subpath);
        self.for_prefix(prefix)?
            .gather(subpath, &shadow_root.join(prefix))
    }

    #[context("affecting at {prefix}/{subpath}")]
    pub fn affect(&mut self, prefix: &str, subpath: &str, shadow_root: &Path) -> Result<()> {
        let subpath = &PathBuf::from_slash(subpath);
        self.for_prefix(prefix)?
            .affect(subpath, &shadow_root.join(prefix))
    }

    fn for_prefix(&mut self, prefix: &str) -> Result<&mut ChildProc> {
        let Some(v) = self.child_procs.get_mut(prefix) else {
            bail!("effector not found for prefix {prefix:?}");
        };
        Ok(v)
    }
}

mod callee {
    use anyhow::{bail, Context, Result};
    use std::io::{BufRead, Write};

    pub trait Handler {
        fn detect(path: &Path) -> Result<bool>;
        fn gather(path: &Path, shadow_root: &Path) -> Result<()>;
        fn affect(path: &Path, shadow_root: &Path) -> Result<()>;
    }

    pub fn parse_and_dispatch(&mut instream: impl BufRead, &mut outstream: impl Write, handler: impl Handler) -> Result<()> {
        let mut buf = String::new();
        loop {
            buf.clear();
            if instream.read_line(&mut buf)? == 0 {
                return Ok(()) // EOF
            }
            if &buf == "com.akavel.mana.v1.rq" {
                outstream.write("com.akavel.mana.v1.rs")?;
                continue;
            }
            const DETECT: &str = "detect ";
            if let Some(raw_args) = buf.strip_prefix(DETECT) {
                // TODO: split on space, verify nothing after it
                // TODO: urlencoding lib looks not super stable, use better one
                let arg = urlencoding::decode(raw_args)?;
                let path = Path::new(&arg);
                let found = handler.detect(path)?;
                let answer = if found { "present" } else { "absent" };
                writeln!(outstream, "detected {answer}")?;
                continue
            }
        }
    }
}

#[cfg(test)]
mod test {
}

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
    struct TestHandler {
        lines: Vec<(String, String, String)>,
        last_detect: bool,
    }

    impl Default for TestHandler {
        fn default() -> Self {
            Self { lines: vec![], last_detect: false }
        }
    }

    impl super::Handler for TestHandler {
        fn detect(&mut self, path: &Path) -> Result<bool> {
            self.lines.append("detect".into(), path.into(), "".into());
            self.last_detect = !self.last_detect;
            Ok(self.last_detect)
        }

        fn gather(path: &Path, shadow_root: &Path) -> Result<()> {
            Ok(())
        }

        fn affect(path: &Path, shadow_root: &Path) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn detecting() {
        let script = r#"com.akavel.mana.v1.rq
detect foo/bar/baz
detect fee/fo/fum"#;
        let h = TestHandler::default();
        let buf = StringBuffer::new();
        parse_and_dispatch(&script, &mut buf, &mut h).unwrap();
        assert_eq!(h.lines, vec![
            ("detect", "foo/bar/baz", ""),
            ("detect", "fee/fo/fum", ""),
        ]);
        assert_eq!(buf, "detected present\ndetected absent");
    }
}

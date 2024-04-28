use anyhow::{bail, Context, Result};
use wmi;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::manaprotocol::callee;

pub struct Handler {
    feats: BTreeSet<PathBuf>,
}

impl Handler {
    pub fn new() -> Result<Handler> {
        query_wmi()
    }
}

impl callee::Handler for Handler {
    fn detect(&mut self, path: &Path) -> Result<bool> {
        Ok(self.feats.contains(path))
    }

    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        // let s = self.feats.get(path).unwrap();
        std::fs::write(shadow_prefix.join(path), "")?;
        Ok(())
    }

    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        bail!("winfeat::affect NIY");
/*
        //println!("MCDBG path={path:?}, spfx={shadow_prefix:?}");
        let shadow_path = shadow_prefix.join(path);

        // Convert path to URL
        // TODO[LATER]: is there better way than first parsing dummy url?
        let mut url = Url::parse("http://akavel.com").unwrap();
        let mut components = path.components();
        let Some((scheme, host)) = components.next_tuple() else {
            bail!("missing scheme or host in path: {path:?}");
        };
        let rel_path = components.collect::<PathBuf>();
        url.set_path(&rel_path.to_slash().unwrap());
        //let Ok(mut url) = Url::from_file_path(&rel_path) else {
        //    bail!("error converting path to url: {rel_path:?}");
        //};
        let Ok(_) = url.set_host(Some(host.as_os_str().to_str().unwrap())) else {
            bail!("error setting host as: {host:?}");
        };
        let Ok(_) = url.set_scheme(scheme.as_os_str().to_str().unwrap()) else {
            bail!("error setting scheme as: {scheme:?}");
        };

        // Try reading shadow file.
        let maybe_content = std::fs::read(&shadow_path);

        // Handle file-not-found scenario - remove app from 0install
        // TODO: merge two ifs once let-chains are stabilized
        if let Err(ref err) = maybe_content {
            if err.kind() == ErrorKind::NotFound {
                let out = Command::new("0install")
                    .args(["remove", &String::from(url)])
                    .output()?;
                if !out.status.success() {
                    bail!(
                        "0install remove failed; STDOUT: {:?}, STDERR: {:?}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr),
                    );
                }
                // TODO[LATER]: refresh query_0install - or mark dirty
                return Ok(());
            }
        }

        let content = maybe_content?;
        let s = std::str::from_utf8(&content)?;
        // TODO: use yaserde::de::from_reader
        // FIXME: don't unwrap
        let mut app = yaserde::de::from_str::<raw::App>(&s).unwrap();

        // Build XML with the app details for feeding into `0install`
        app.interface = Some(url.into());
        let list = raw::AppList {
            app: vec![app],
        };
        let list_file = tempfile::NamedTempFile::new()?;
        let rs = yaserde::ser::serialize_with_writer(&list, list_file, &Default::default());
        let Ok(list_file) = rs else {
            bail!("{}", rs.unwrap_err());
        };
        //println!("MCDBG XML {}", String::from_utf8_lossy(&std::fs::read(list_file.path()).unwrap()));

        // Feed the XML into `0install` to install the app.
        let xml_path = list_file.into_temp_path();
        println!("- 0install {path:?}...");
        let out = Command::new("0install")
            //.args(["import-apps", "--batch", "-o", &list_file.path().to_string_lossy()])
            .args(["import-apps", "--batch", &xml_path.to_string_lossy()])
            .output()?;
        if !out.status.success() {
            bail!(
                "0install failed; STDOUT: {:?}, STDERR: {:?}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr),
            );
        }

        // TODO[LATER]: refresh query_0install - or mark dirty

        Ok(())
*/
    }
}

fn query_wmi() -> Result<Handler> {
    bail!("NIY");
}

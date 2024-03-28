use anyhow::{bail, Context, Result};
use itertools::Itertools;
use url::Url;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::manaprotocol::callee;

pub struct Handler {
    apps: BTreeMap<PathBuf, Timestamp>,
}

impl Handler {
    pub fn new() -> Result<Handler> {
        query_0install()
    }
}

impl callee::Handler for Handler {
    fn detect(&mut self, path: &Path) -> Result<bool> {
        Ok(self.apps.contains_key(path))
    }

    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        let t = self.apps.get(path).unwrap();
        std::fs::write(shadow_prefix.join(path), format!("{t}"))?;
        Ok(())
    }

    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        let shadow_path = shadow_prefix.join(path);
        // FIXME: handle file-not-found error - delete app from 0install then
        let content = std::fs::read(&shadow_path)?;
        let timestamp: u64 = std::str::from_utf8(&content)?.parse()?;

        // Convert path to URL
        let mut components = path.components();
        let Some((scheme, host)) = components.next_tuple() else {
            bail!("missing scheme or host in path: {path:?}");
        };
        let rel_path = components.collect::<PathBuf>();
        let Ok(mut url) = Url::from_file_path(&rel_path) else {
            bail!("error converting path to url: {rel_path:?}");
        };
        let Ok(_) = url.set_host(Some(host.as_os_str().to_str().unwrap())) else {
            bail!("error setting host as: {host:?}");
        };
        let Ok(_) = url.set_scheme(scheme.as_os_str().to_str().unwrap()) else {
            bail!("error setting scheme as: {scheme:?}");
        };

        // Build XML with the app details for feeding into `0install`
        let list = raw::AppList {
            app: vec![
                raw::App {
                    interface: url.into(),
                    timestamp,
                },
            ],
        };
        let list_file = tempfile::NamedTempFile::new()?;
        let rs = yaserde::ser::serialize_with_writer_content(&list, list_file);
        let Ok(list_file) = rs else {
            bail!("{}", rs.unwrap_err());
        };

        // Feed the XML into `0install` to install the app.
        let out = Command::new("0install")
            .args(["import-apps", "--batch", "-o", &list_file.path().to_string_lossy()])
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
    }
}

type Timestamp = u64;

fn query_0install() -> Result<Handler> {
    // TODO[LATER]: streaming read & parse
    // TODO[LATER]: handle stderr & better handling of errors/failures
    let stdout = Command::new("0install")
        .args(["list-apps", "--xml"])
        .output()?
        //.expect("failed to exec 0install")
        .stdout;
    // FIXME: why std::str::from_utf8(&stdout).unwrap() panicked?
    let s = String::from_utf8_lossy(&stdout);
    //println!("{}", s); //.unwrap());
    let app_list = yaserde::de::from_str::<raw::AppList>(&s).unwrap();
    //println!("{:?}", app_list); //.unwrap());
    let map: Result<BTreeMap<_, _>> = app_list
        .app
        .into_iter()
        .map(|app| {
            let url = Url::parse(&app.interface)?;
            // FIXME: ensure/sanitize/encode safe scheme, no username/password, no ipv6, no port, etc.
            let scheme = url.scheme().to_string();
            let host = url.host_str().unwrap().to_string();
            let path: PathBuf = url
                .path_segments()
                .unwrap()
                .map(|s| Path::new(s).to_path_buf())
                .collect();
            let disk_path = Path::new(&scheme).join(host).join(path);
            Ok((disk_path, app.timestamp))
        })
        .collect();
    //println!("{map:?}");
    Ok(Handler { apps: map? })
}

mod raw {
    use yaserde::{YaDeserialize, YaSerialize};

    #[derive(YaDeserialize, YaSerialize, Debug)]
    #[yaserde(
        rename = "app-list",
        namespace = "http://0install.de/schema/desktop-integration/app-list"
    )]
    pub struct AppList {
        #[yaserde(child)]
        pub app: Vec<App>,
    }

    #[derive(YaDeserialize, YaSerialize, Debug)]
    #[yaserde(rename = "app")]
    pub struct App {
        #[yaserde(attribute)]
        pub interface: String,
        #[yaserde(attribute)]
        pub timestamp: u64,
    }
}

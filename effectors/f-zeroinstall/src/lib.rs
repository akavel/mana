pub mod xmlutil;

use anyhow::{bail, Result};
use itertools::Itertools;
use log::debug;
use path_slash::PathBufExt as _;
use url::Url;

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Effector {
    apps: BTreeMap<PathBuf, String>,
}

impl Effector {
    pub fn new() -> Result<Effector> {
        query_0install()
    }
}

impl effectors::Callee for Effector {
    fn start(_: std::env::Args) -> Result<Self> {
        Effector::new()
    }

    fn detect(&mut self, path: &Path) -> Result<bool> {
        Ok(self.apps.contains_key(path))
    }

    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
        let s = self.apps.get(path).unwrap();
        std::fs::write(shadow_prefix.join(path), s)?;
        Ok(())
    }

    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()> {
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
        let maybe_content = std::fs::read(shadow_path);

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
        let mut app = yaserde::de::from_str::<raw::App>(s).unwrap();

        // Build XML with the app details for feeding into `0install`
        app.interface = Some(url.into());
        let list = raw::AppList { app: vec![app] };
        let list_file = tempfile::NamedTempFile::new()?;
        let rs = yaserde::ser::serialize_with_writer(&list, list_file, &Default::default());
        let Ok(list_file) = rs else {
            bail!("{}", rs.unwrap_err());
        };
        //println!("MCDBG XML {}", String::from_utf8_lossy(&std::fs::read(list_file.path()).unwrap()));

        // Feed the XML into `0install` to install the app.
        let xml_path = list_file.into_temp_path();
        debug!("- 0install {path:?}...");
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
    }
}

fn query_0install() -> Result<Effector> {
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
    debug!("{:?}", app_list); //.unwrap());
    let map: Result<BTreeMap<_, _>> = app_list
        .app
        .into_iter()
        .map(|mut app| {
            let url = Url::parse(&app.interface.take().unwrap())?;
            // FIXME: ensure/sanitize/encode safe scheme, no username/password, no ipv6, no port, etc.
            let scheme = url.scheme().to_string();
            let host = url.host_str().unwrap().to_string();
            let path: PathBuf = url
                .path_segments()
                .unwrap()
                .map(|s| Path::new(s).to_path_buf())
                .collect();
            let disk_path = Path::new(&scheme).join(host).join(path);
            let cfg = yaserde::ser::Config {
                perform_indent: true,
                write_document_declaration: false,
                indent_string: None, // presumably, will use a default (?)
            };
            let xml = yaserde::ser::to_string_with_config(&app, &cfg).unwrap();
            Ok((disk_path, xml))
        })
        .collect();
    //println!("{map:?}");
    Ok(Effector { apps: map? })
}

mod raw {
    use crate::xmlutil;
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
        pub interface: Option<String>,
        #[yaserde(child)]
        pub capabilities: Option<xmlutil::OpaqueXml>,
        #[yaserde(child, rename = "access-points")]
        pub access_points: Option<xmlutil::OpaqueXml>,
    }
}

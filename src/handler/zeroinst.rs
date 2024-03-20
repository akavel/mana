use anyhow::{bail, Context, Result};

use std::path::Path;

use crate::manaprotocol::callee;

struct Handler {
}

impl callee::Handler for Handler {
    fn detect(&mut self, path: &Path) -> Result<bool> {
    }

    fn gather(&mut self, path: &Path, shadow_root: &Path) -> Result<()> {
    }

    fn affect(&mut self, path: &Path, shadow_root: &Path) -> Result<()> {
    }
}

mod raw {
    use yaserde::YaDeserialize;

    #[derive(YaDeserialize, Debug)]
    #[yaserde(
        rename = "app-list",
        namespace = "http://0install.de/schema/desktop-integration/app-list"
    )]
    pub struct AppList {
        #[yaserde(child)]
        pub app: Vec<App>,
    }

    #[derive(YaDeserialize, Debug)]
    #[yaserde(rename = "app")]
    pub struct App {
        #[yaserde(attribute)]
        pub interface: String,
        #[yaserde(attribute)]
        pub timestamp: u64,
    }
}

struct Apps {}

fn query_0install() -> Apps {
    use std::process::Command;
    // TODO[LATER]: streaming read & parse
    // TODO[LATER]: handle stderr & better handling of errors/failures
    let stdout = Command::new("0install")
        .args(["list-apps", "--xml"])
        .output()
        .expect("failed to exec 0install")
        .stdout;
    // FIXME: why std::str::from_utf8(&stdout).unwrap() panicked?
    let s = String::from_utf8_lossy(&stdout);
    println!("{}", s); //.unwrap());
    let result = yaserde::de::from_str::<raw::AppList>(&s).unwrap();
    println!("{:?}", result); //.unwrap());
    Apps {}
}

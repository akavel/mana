use anyhow::{bail, Context, Result};

use std::path::Path;

fn main() -> Result<()> {
    query_0install();
    Ok(())
}

fn exists(path: &Path) {}

fn query(path: &Path, shadowpath: &Path) {}

mod zeroinstall {
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
    let result = yaserde::de::from_str::<zeroinstall::AppList>(&s).unwrap();
    println!("{:?}", result); //.unwrap());
    Apps {}
}

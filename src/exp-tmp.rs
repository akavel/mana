use anyhow::{bail, Context, Result};

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let h = query_wmi()?;
    println!("{h:?}");
    Ok(())
}

#[derive(Debug)]
pub struct Handler {
    feats: BTreeSet<PathBuf>,
}

mod raw {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    #[serde(rename = "Win32_OptionalFeature")]
    #[serde(rename_all = "PascalCase")]
    pub struct Win32OptFeat {
        pub name: String,
    }
}

fn query_wmi() -> Result<Handler> {
    use wmi::*;
    let wmi_con = WMIConnection::new(COMLibrary::new()?)?;
    let results: Vec<raw::Win32OptFeat> = wmi_con.raw_query(
         "SELECT * FROM Win32_OptionalFeature WHERE InstallState = 1"
    )?;
    let feats = results.into_iter().map(|v| v.name.into()).collect();
    Ok(Handler { feats })
}

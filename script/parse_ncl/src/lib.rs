use anyhow::{bail, Context, Result};
use std::path::Path;
use toml::macros::Deserialize;

pub fn from_file(ncl_path: &Path) -> Result<toml::Table> {
    let username = whoami::username();
    let mut hostname = whoami::fallible::hostname()?;
    hostname.make_ascii_lowercase();
    let field_path_raw = format!("{username}@{hostname}");

    use nickel_lang_core::{
        error::report::ErrorFormat, eval::cache::lazy::CBNCache, identifier::LocIdent,
        pretty::ident_quoted, program::Program as Prog,
    };
    let field_path = ident_quoted(&LocIdent::new(field_path_raw));
    // println!("FIELD: {field_path:?}");
    use std::io::stderr;
    let mut prog = Prog::<CBNCache>::new_from_file(&ncl_path, stderr())?;
    let res_field = prog.parse_field_path(field_path.clone());
    let Ok(field) = res_field else {
        prog.report(res_field.unwrap_err(), ErrorFormat::Text);
        bail!("failed to parse {field_path:?} as Nickel path");
    };
    prog.field = field;
    let res_term = prog.eval_full_for_export();
    let Ok(term) = res_term else {
        prog.report(res_term.unwrap_err(), ErrorFormat::Text);
        bail!("script {ncl_path:?} failed");
    };
    let toml = toml::Table::deserialize(term).context("loading Nickel output to TOML")?;
    Ok(toml)
}

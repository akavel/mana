use anyhow::Result;
use std::path::Path;

pub trait Callee {
    fn detect(&mut self, path: &Path) -> Result<bool>;
    fn gather(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;
    fn affect(&mut self, path: &Path, shadow_prefix: &Path) -> Result<()>;
}

// stub - will be implemented next
use std::io;
use std::path::{Path, PathBuf};

pub fn discover_env_files(_dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
    Ok(vec![])
}

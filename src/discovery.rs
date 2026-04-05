use std::io;
use std::path::{Path, PathBuf};

/// Auto-discover all `.env*` files in `dir`, sorted by filename.
///
/// A file qualifies if its name starts with `.env` (e.g. `.env`, `.env.local`,
/// `.env.production`). Hidden subdirectories and symlinks that don't resolve
/// are skipped gracefully.
pub fn discover_env_files(dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let read_dir = std::fs::read_dir(dir)?;

    let mut files: Vec<PathBuf> = read_dir
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let file_type = entry.file_type().ok()?;

            // Follow symlinks to check if they point to a file
            let is_file = if file_type.is_symlink() {
                entry.path().metadata().map(|m| m.is_file()).unwrap_or(false)
            } else {
                file_type.is_file()
            };

            if !is_file {
                return None;
            }

            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with(".env") {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect();

    files.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .cmp(b.file_name().unwrap_or_default())
    });

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "").unwrap();
        path
    }

    #[test]
    fn test_discovers_env_files() {
        let tmp = tempfile::tempdir().unwrap();
        make_file(tmp.path(), ".env");
        make_file(tmp.path(), ".env.local");
        make_file(tmp.path(), ".env.production");
        make_file(tmp.path(), "Cargo.toml"); // should be ignored

        let found = discover_env_files(tmp.path()).unwrap();
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();

        assert_eq!(names, vec![".env", ".env.local", ".env.production"]);
    }

    #[test]
    fn test_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let found = discover_env_files(tmp.path()).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn test_no_env_files() {
        let tmp = tempfile::tempdir().unwrap();
        make_file(tmp.path(), "Cargo.toml");
        make_file(tmp.path(), "README.md");

        let found = discover_env_files(tmp.path()).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn test_sorted_output() {
        let tmp = tempfile::tempdir().unwrap();
        make_file(tmp.path(), ".env.staging");
        make_file(tmp.path(), ".env");
        make_file(tmp.path(), ".env.development");

        let found = discover_env_files(tmp.path()).unwrap();
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();

        assert_eq!(names, vec![".env", ".env.development", ".env.staging"]);
    }

    #[test]
    fn test_nonexistent_directory() {
        let result = discover_env_files(Path::new("/nonexistent/dir/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_directories_not_included() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a directory named .env.d — should be skipped
        fs::create_dir(tmp.path().join(".env.d")).unwrap();
        make_file(tmp.path(), ".env");

        let found = discover_env_files(tmp.path()).unwrap();
        assert_eq!(found.len(), 1);
    }
}

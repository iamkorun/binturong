use crate::parser::EnvFile;
use std::collections::BTreeSet;

/// Status of a single key in a single file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStatus {
    /// Key is present and has a non-empty value.
    Present(String),
    /// Key is present but has an empty value.
    Empty,
    /// Key is missing from this file entirely.
    Missing,
}

impl KeyStatus {
    #[allow(dead_code)]
    pub fn is_present(&self) -> bool {
        !matches!(self, KeyStatus::Missing)
    }
}

/// A row in the comparison report — one key across all files.
#[derive(Debug, Clone)]
pub struct KeyRow {
    pub key: String,
    /// Status for each file, in the same order as `DriftReport.files`.
    pub statuses: Vec<KeyStatus>,
    /// True if this key differs across at least two files.
    pub has_drift: bool,
}

/// The full comparison report.
#[derive(Debug)]
pub struct DriftReport {
    pub files: Vec<EnvFile>,
    /// All keys seen across all files, sorted alphabetically.
    #[allow(dead_code)]
    pub all_keys: Vec<String>,
    /// One row per key.
    pub rows: Vec<KeyRow>,
}

impl DriftReport {
    /// Returns true if any key has drift (missing in some file, or value differs).
    pub fn has_drift(&self) -> bool {
        self.rows.iter().any(|r| r.has_drift)
    }

    /// Returns only rows that have drift.
    pub fn drifted_rows(&self) -> impl Iterator<Item = &KeyRow> {
        self.rows.iter().filter(|r| r.has_drift)
    }
}

/// Compare N parsed env files and produce a drift report.
///
/// Drift is defined as any key that is:
/// - present in some files but not others (missing), or
/// - present in all files but with different values.
///
/// A key that is empty in all files is NOT considered drift.
pub fn compare_files(files: &[EnvFile]) -> DriftReport {
    // Collect all unique keys across all files (sorted)
    let all_keys: Vec<String> = files
        .iter()
        .flat_map(|f| f.keys.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let rows: Vec<KeyRow> = all_keys
        .iter()
        .map(|key| {
            let statuses: Vec<KeyStatus> = files
                .iter()
                .map(|f| match f.entries.get(key) {
                    None => KeyStatus::Missing,
                    Some(v) if v.is_empty() => KeyStatus::Empty,
                    Some(v) => KeyStatus::Present(v.clone()),
                })
                .collect();

            let has_drift = detect_drift(&statuses);

            KeyRow {
                key: key.clone(),
                statuses,
                has_drift,
            }
        })
        .collect();

    DriftReport {
        files: files.to_vec(),
        all_keys,
        rows,
    }
}

/// A key has drift if its statuses are not all equivalent across files.
///
/// Equivalence rules:
/// - All `Missing` → no drift (key simply doesn't exist anywhere).
/// - All `Empty` → no drift.
/// - All `Present` with the same value → no drift.
/// - Any combination of Missing/Empty/Present(different values) → drift.
fn detect_drift(statuses: &[KeyStatus]) -> bool {
    if statuses.len() <= 1 {
        return false;
    }

    let first = &statuses[0];

    for status in &statuses[1..] {
        if status != first {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_content;
    use std::path::PathBuf;

    fn make_env_file(path: &str, content: &str) -> EnvFile {
        let (keys, entries) = parse_content(content);
        EnvFile {
            path: PathBuf::from(path),
            keys,
            entries,
        }
    }

    #[test]
    fn test_no_drift_identical_files() {
        let a = make_env_file(".env", "FOO=bar\nBAZ=qux\n");
        let b = make_env_file(".env.local", "FOO=bar\nBAZ=qux\n");
        let report = compare_files(&[a, b]);
        assert!(!report.has_drift());
    }

    #[test]
    fn test_drift_missing_key() {
        let a = make_env_file(".env", "FOO=bar\nBAZ=qux\n");
        let b = make_env_file(".env.local", "FOO=bar\n");
        let report = compare_files(&[a, b]);
        assert!(report.has_drift());

        let baz_row = report.rows.iter().find(|r| r.key == "BAZ").unwrap();
        assert!(baz_row.has_drift);
        assert_eq!(baz_row.statuses[0], KeyStatus::Present("qux".to_string()));
        assert_eq!(baz_row.statuses[1], KeyStatus::Missing);
    }

    #[test]
    fn test_drift_different_values() {
        let a = make_env_file(".env", "FOO=local\n");
        let b = make_env_file(".env.production", "FOO=prod\n");
        let report = compare_files(&[a, b]);
        assert!(report.has_drift());
    }

    #[test]
    fn test_no_drift_empty_in_all() {
        let a = make_env_file(".env", "FOO=\n");
        let b = make_env_file(".env.local", "FOO=\n");
        let report = compare_files(&[a, b]);
        assert!(!report.has_drift());
    }

    #[test]
    fn test_drift_empty_vs_value() {
        let a = make_env_file(".env", "FOO=bar\n");
        let b = make_env_file(".env.local", "FOO=\n");
        let report = compare_files(&[a, b]);
        assert!(report.has_drift());
    }

    #[test]
    fn test_keys_sorted_alphabetically() {
        let a = make_env_file(".env", "ZEBRA=1\nAPPLE=2\nMIDDLE=3\n");
        let b = make_env_file(".env.local", "ZEBRA=1\nAPPLE=2\nMIDDLE=3\n");
        let report = compare_files(&[a, b]);
        assert_eq!(report.all_keys, vec!["APPLE", "MIDDLE", "ZEBRA"]);
    }

    #[test]
    fn test_three_files_drift() {
        let a = make_env_file(".env", "FOO=a\nBAR=x\n");
        let b = make_env_file(".env.staging", "FOO=a\nBAR=y\n");
        let c = make_env_file(".env.production", "FOO=a\n");
        let report = compare_files(&[a, b, c]);
        assert!(report.has_drift());

        let foo_row = report.rows.iter().find(|r| r.key == "FOO").unwrap();
        assert!(!foo_row.has_drift);

        let bar_row = report.rows.iter().find(|r| r.key == "BAR").unwrap();
        assert!(bar_row.has_drift);
    }

    #[test]
    fn test_drifted_rows_filter() {
        let a = make_env_file(".env", "FOO=same\nBAR=a\n");
        let b = make_env_file(".env.local", "FOO=same\nBAR=b\n");
        let report = compare_files(&[a, b]);
        let drifted: Vec<_> = report.drifted_rows().collect();
        assert_eq!(drifted.len(), 1);
        assert_eq!(drifted[0].key, "BAR");
    }

    #[test]
    fn test_single_file_no_drift() {
        let a = make_env_file(".env", "FOO=bar\n");
        let report = compare_files(&[a]);
        assert!(!report.has_drift());
    }

    #[test]
    fn test_all_empty_files_no_drift() {
        let a = make_env_file(".env", "");
        let b = make_env_file(".env.local", "");
        let report = compare_files(&[a, b]);
        assert!(!report.has_drift());
        assert!(report.rows.is_empty());
    }

    #[test]
    fn test_extra_key_in_one_file() {
        let a = make_env_file(".env.example", "FOO=x\nEXTRA=y\n");
        let b = make_env_file(".env", "FOO=x\n");
        let report = compare_files(&[a, b]);
        assert!(report.has_drift());

        let extra_row = report.rows.iter().find(|r| r.key == "EXTRA").unwrap();
        assert_eq!(extra_row.statuses[0], KeyStatus::Present("y".to_string()));
        assert_eq!(extra_row.statuses[1], KeyStatus::Missing);
    }

    #[test]
    fn test_key_status_is_present() {
        assert!(KeyStatus::Present("v".to_string()).is_present());
        assert!(KeyStatus::Empty.is_present());
        assert!(!KeyStatus::Missing.is_present());
    }

    #[test]
    fn test_all_keys_aggregates_across_files() {
        let a = make_env_file(".env", "FOO=1\nBAR=2\n");
        let b = make_env_file(".env.local", "BAR=2\nBAZ=3\n");
        let report = compare_files(&[a, b]);
        assert_eq!(report.all_keys, vec!["BAR", "BAZ", "FOO"]);
    }

    #[test]
    fn test_no_files_produces_empty_report() {
        let report = compare_files(&[]);
        assert!(!report.has_drift());
        assert!(report.rows.is_empty());
        assert!(report.all_keys.is_empty());
    }
}

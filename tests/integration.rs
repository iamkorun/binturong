use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

/// Path to the compiled binary (built by cargo test automatically).
fn bin_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    // When tests run from target/debug/deps, the binary is at target/debug/
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("binturong");
    path
}

fn create_env_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

// ─── Exit code tests ────────────────────────────────────────────────────────

#[test]
fn test_exit_0_when_in_sync() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "FOO=bar\nBAZ=qux\n");
    let b = create_env_file(&tmp, ".env.local", "FOO=bar\nBAZ=qux\n");

    let status = Command::new(bin_path())
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(0));
}

#[test]
fn test_exit_1_when_drift() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "FOO=bar\nBAZ=qux\n");
    let b = create_env_file(&tmp, ".env.local", "FOO=bar\n");

    let status = Command::new(bin_path())
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(1));
}

#[test]
fn test_exit_2_file_not_found() {
    let status = Command::new(bin_path())
        .args(["/nonexistent/.env", "/also/missing/.env.local"])
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(2));
}

#[test]
fn test_exit_2_only_one_file() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "FOO=bar\n");

    let status = Command::new(bin_path())
        .arg(a.to_str().unwrap())
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(2));
}

// ─── Output content tests ───────────────────────────────────────────────────

#[test]
fn test_output_shows_key_names() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "DATABASE_URL=postgres://localhost/db\n");
    let b = create_env_file(&tmp, ".env.local", "DATABASE_URL=postgres://localhost/db\n");

    let output = Command::new(bin_path())
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DATABASE_URL"));
}

#[test]
fn test_output_masks_values_by_default() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "SECRET=supersecretvalue\n");
    let b = create_env_file(&tmp, ".env.local", "SECRET=othersecret\n");

    let output = Command::new(bin_path())
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("supersecretvalue"), "value should be masked by default");
    assert!(!stdout.contains("othersecret"), "value should be masked by default");
    assert!(stdout.contains("****"), "should show mask symbol");
}

#[test]
fn test_values_flag_shows_actual_values() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "SECRET=actualvalue\n");
    let b = create_env_file(&tmp, ".env.local", "SECRET=othervalue\n");

    let output = Command::new(bin_path())
        .args(["--values", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("actualvalue"), "--values should show actual values");
}

#[test]
fn test_diff_flag_hides_in_sync_keys() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "STABLE=same\nDRIFTED=a\n");
    let b = create_env_file(&tmp, ".env.local", "STABLE=same\nDRIFTED=b\n");

    let output = Command::new(bin_path())
        .args(["--diff", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DRIFTED"), "--diff should show drifted keys");
    // STABLE should not appear in table rows (it is in sync)
    // Check that "STABLE" doesn't appear in any data rows
    let stable_in_data = stdout
        .lines()
        .filter(|l| l.contains("STABLE") && !l.contains("KEY"))
        .count();
    assert_eq!(stable_in_data, 0, "STABLE should not appear in --diff output: {stdout}");
}

#[test]
fn test_quiet_flag_no_output_on_drift() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "FOO=bar\n");
    let b = create_env_file(&tmp, ".env.local", "FOO=different\n");

    let output = Command::new(bin_path())
        .args(["-q", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.stdout.is_empty(), "quiet mode should produce no stdout");
    // exit code should still be 1
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_output_shows_missing_indicator() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "FOO=bar\nEXTRA=only_here\n");
    let b = create_env_file(&tmp, ".env.local", "FOO=bar\n");

    let output = Command::new(bin_path())
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("missing"), "should show 'missing' for absent key");
}

// ─── Auto-discovery tests ────────────────────────────────────────────────────

#[test]
fn test_auto_discovers_env_files_in_directory() {
    let tmp = TempDir::new().unwrap();
    create_env_file(&tmp, ".env", "FOO=bar\n");
    create_env_file(&tmp, ".env.local", "FOO=bar\n");

    let output = Command::new(bin_path())
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_auto_discovery_no_files_exits_2() {
    let tmp = TempDir::new().unwrap();
    // No .env files at all

    let output = Command::new(bin_path())
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
}

// ─── Edge case tests ─────────────────────────────────────────────────────────

#[test]
fn test_empty_file_handles_gracefully() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "");
    let b = create_env_file(&tmp, ".env.local", "");

    let output = Command::new(bin_path())
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_comments_and_blank_lines_skipped() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "# comment\n\nFOO=bar\n");
    let b = create_env_file(&tmp, ".env.local", "# another comment\n\nFOO=bar\n");

    let output = Command::new(bin_path())
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_export_prefix_handled() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "export FOO=bar\n");
    let b = create_env_file(&tmp, ".env.local", "FOO=bar\n");

    let output = Command::new(bin_path())
        .args([a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    // FOO=bar in both, just different formatting — should be in sync
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_three_files_comparison() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "FOO=a\nBAR=x\nBAZ=common\n");
    let b = create_env_file(&tmp, ".env.staging", "FOO=a\nBAR=y\nBAZ=common\n");
    let c = create_env_file(&tmp, ".env.production", "FOO=a\nBAR=z\nBAZ=common\n");

    let output = Command::new(bin_path())
        .args([
            a.to_str().unwrap(),
            b.to_str().unwrap(),
            c.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1)); // BAR drifts
}

#[test]
fn test_verbose_flag_lists_drifted_keys() {
    let tmp = TempDir::new().unwrap();
    let a = create_env_file(&tmp, ".env", "FOO=bar\nBAZ=a\n");
    let b = create_env_file(&tmp, ".env.local", "FOO=bar\nBAZ=b\n");

    let output = Command::new(bin_path())
        .args(["--verbose", a.to_str().unwrap(), b.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Drifted keys:"));
    assert!(stdout.contains("BAZ"));
}

#[test]
fn test_version_flag() {
    let output = Command::new(bin_path())
        .arg("--version")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("binturong"));
    assert_eq!(output.status.code(), Some(0));
}

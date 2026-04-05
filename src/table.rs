use crate::compare::{DriftReport, KeyStatus};
use colored::Colorize;
use comfy_table::{Attribute, Cell, CellAlignment, Color, ContentArrangement, Table};

const MASK: &str = "****";
const VALUE_MAX_LEN: usize = 32;

/// Render the drift report as a formatted table string.
pub fn render_table(report: &DriftReport, diff_only: bool, show_values: bool, verbose: bool) -> String {
    if report.files.is_empty() || report.rows.is_empty() {
        return render_empty(report, diff_only, verbose);
    }

    let rows: Vec<_> = if diff_only {
        report.drifted_rows().collect()
    } else {
        report.rows.iter().collect()
    };

    if rows.is_empty() {
        return format!(
            "{} All {} keys are in sync across {} files.",
            "✓".green().bold(),
            report.rows.len(),
            report.files.len()
        );
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    // Build header: KEY + one column per file
    let mut header: Vec<Cell> = vec![Cell::new("KEY")
        .add_attribute(Attribute::Bold)
        .set_alignment(CellAlignment::Left)];

    for file in &report.files {
        header.push(
            Cell::new(file.filename())
                .add_attribute(Attribute::Bold)
                .set_alignment(CellAlignment::Center),
        );
    }
    table.set_header(header);

    // Build rows
    for row in &rows {
        let key_cell = if row.has_drift {
            Cell::new(&row.key)
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow)
        } else {
            Cell::new(&row.key)
        };

        let mut cells: Vec<Cell> = vec![key_cell];

        for status in &row.statuses {
            let cell = match status {
                KeyStatus::Present(val) => {
                    let display = if show_values {
                        truncate(val, VALUE_MAX_LEN)
                    } else {
                        MASK.to_string()
                    };
                    let symbol = if row.has_drift {
                        format!("✓ {display}").yellow().to_string()
                    } else {
                        format!("✓ {display}").green().to_string()
                    };
                    Cell::new(symbol).set_alignment(CellAlignment::Center)
                }
                KeyStatus::Empty => {
                    let symbol = if row.has_drift {
                        "○ (empty)".yellow().to_string()
                    } else {
                        "○ (empty)".dimmed().to_string()
                    };
                    Cell::new(symbol).set_alignment(CellAlignment::Center)
                }
                KeyStatus::Missing => {
                    Cell::new("✗ missing".red().bold().to_string())
                        .set_alignment(CellAlignment::Center)
                }
            };
            cells.push(cell);
        }

        table.add_row(cells);
    }

    let mut output = table.to_string();

    // Summary footer
    let drift_count = report.rows.iter().filter(|r| r.has_drift).count();
    let total = report.rows.len();
    let file_count = report.files.len();

    output.push('\n');

    if diff_only && drift_count == 0 {
        output.push_str(&format!(
            "{} {} key{} in sync across {} file{}.",
            "✓".green().bold(),
            total,
            if total == 1 { "" } else { "s" },
            file_count,
            if file_count == 1 { "" } else { "s" },
        ));
    } else if drift_count > 0 {
        output.push_str(&format!(
            "{} {}/{} key{} drifted  ({} file{})",
            "✗".red().bold(),
            drift_count,
            total,
            if total == 1 { "" } else { "s" },
            file_count,
            if file_count == 1 { "" } else { "s" },
        ));
    } else {
        output.push_str(&format!(
            "{} All {} key{} are in sync across {} file{}.",
            "✓".green().bold(),
            total,
            if total == 1 { "" } else { "s" },
            file_count,
            if file_count == 1 { "" } else { "s" },
        ));
    }

    if verbose && drift_count > 0 {
        output.push_str("\n\nDrifted keys:\n");
        for row in report.drifted_rows() {
            output.push_str(&format!("  - {}\n", row.key));
        }
    }

    output
}

fn render_empty(report: &DriftReport, diff_only: bool, _verbose: bool) -> String {
    if report.files.is_empty() {
        return "No files to compare.".to_string();
    }
    if diff_only {
        return format!(
            "{} No drift detected across {} files (no keys defined).",
            "✓".green().bold(),
            report.files.len()
        );
    }
    format!(
        "{} No keys found across {} files.",
        "○".dimmed(),
        report.files.len()
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compare::compare_files;
    use crate::parser::parse_content;
    use std::path::PathBuf;
    use crate::parser::EnvFile;

    fn make_env_file(path: &str, content: &str) -> EnvFile {
        let (keys, entries) = parse_content(content);
        EnvFile {
            path: PathBuf::from(path),
            keys,
            entries,
        }
    }

    #[test]
    fn test_render_in_sync() {
        let a = make_env_file(".env", "FOO=bar\n");
        let b = make_env_file(".env.local", "FOO=bar\n");
        let report = compare_files(&[a, b]);
        let output = render_table(&report, false, false, false);
        assert!(output.contains("FOO"));
        assert!(output.contains("in sync"));
    }

    #[test]
    fn test_render_drift_shows_missing() {
        let a = make_env_file(".env", "FOO=bar\nBAZ=x\n");
        let b = make_env_file(".env.local", "FOO=bar\n");
        let report = compare_files(&[a, b]);
        let output = render_table(&report, false, false, false);
        assert!(output.contains("BAZ"));
        assert!(output.contains("missing"));
    }

    #[test]
    fn test_diff_only_hides_in_sync_keys() {
        let a = make_env_file(".env", "FOO=same\nBAR=a\n");
        let b = make_env_file(".env.local", "FOO=same\nBAR=b\n");
        let report = compare_files(&[a, b]);
        let output = render_table(&report, true, false, false);
        // BAR drifts, should appear; FOO is in sync, should be absent from rows
        assert!(output.contains("BAR"));
        // FOO should not appear in a row (it may appear in column headers — that's the filename, not the key)
        // The table rows won't have FOO since it's in sync
        let lines: Vec<&str> = output.lines().collect();
        let data_rows: Vec<&&str> = lines.iter().filter(|l| l.contains("FOO") && !l.contains("KEY")).collect();
        assert!(data_rows.is_empty(), "FOO should not appear in diff-only output: {output}");
    }

    #[test]
    fn test_values_mode_shows_value() {
        let a = make_env_file(".env", "FOO=secretvalue\n");
        let b = make_env_file(".env.local", "FOO=othervalue\n");
        let report = compare_files(&[a, b]);

        let masked = render_table(&report, false, false, false);
        assert!(masked.contains("****"));
        assert!(!masked.contains("secretvalue"));

        let revealed = render_table(&report, false, true, false);
        assert!(revealed.contains("secretvalue"));
    }

    #[test]
    fn test_empty_files_output() {
        let a = make_env_file(".env", "");
        let b = make_env_file(".env.local", "");
        let report = compare_files(&[a, b]);
        let output = render_table(&report, false, false, false);
        assert!(output.contains("No keys found"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 32), "short");
        let long = "a".repeat(40);
        let result = truncate(&long, 32);
        // '…' is 3 bytes; 31 ASCII chars + 3 bytes for ellipsis = 34 bytes max
        assert!(result.len() <= 34, "len was {}", result.len());
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_render_verbose_lists_drifted() {
        let a = make_env_file(".env", "FOO=a\nBAR=x\n");
        let b = make_env_file(".env.prod", "FOO=a\nBAR=y\n");
        let report = compare_files(&[a, b]);
        let output = render_table(&report, false, false, true);
        assert!(output.contains("Drifted keys:"));
        assert!(output.contains("BAR"));
    }
}

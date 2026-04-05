use clap::Parser;
use std::path::PathBuf;
use std::process;

mod compare;
mod discovery;
mod parser;
mod table;

/// binturong — compare .env files side-by-side to detect config drift
#[derive(Parser, Debug)]
#[command(
    name = "binturong",
    version,
    about = "Compare .env files side-by-side to detect config drift",
    long_about = "binturong compares multiple .env-style files and shows which keys are \
missing, extra, or have drifted across environments.\n\n\
Exit codes:\n  \
0 = all files are in sync\n  \
1 = drift detected\n  \
2 = error (file not found, permission denied, etc.)"
)]
struct Cli {
    /// Files to compare (auto-discovers .env* files in current dir if not specified)
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Show only keys that differ across files
    #[arg(long, short = 'd')]
    diff: bool,

    /// Show actual values in the table (masked by default)
    #[arg(long)]
    values: bool,

    /// Suppress output; exit code only
    #[arg(long, short = 'q')]
    quiet: bool,

    /// Show extra details
    #[arg(long, short = 'v')]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    let files: Vec<PathBuf> = if cli.files.is_empty() {
        match discovery::discover_env_files(&std::env::current_dir().unwrap_or_default()) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(2);
            }
        }
    } else {
        cli.files.clone()
    };

    if files.is_empty() {
        eprintln!("error: no .env files found. Pass file paths explicitly or run in a directory containing .env* files.");
        process::exit(2);
    }

    if files.len() < 2 {
        eprintln!(
            "error: at least 2 files are required for comparison (found: {})",
            files
                .first()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        );
        process::exit(2);
    }

    if cli.verbose && !cli.quiet {
        eprintln!("comparing {} files:", files.len());
        for f in &files {
            eprintln!("  {}", f.display());
        }
    }

    let parsed: Vec<parser::EnvFile> = files
        .iter()
        .map(|path| match parser::parse_env_file(path) {
            Ok(ef) => ef,
            Err(e) => {
                eprintln!("error reading {}: {e}", path.display());
                process::exit(2);
            }
        })
        .collect();

    let report = compare::compare_files(&parsed);

    let has_drift = report.has_drift();

    if !cli.quiet {
        let output = table::render_table(&report, cli.diff, cli.values, cli.verbose);
        println!("{output}");
    }

    process::exit(if has_drift { 1 } else { 0 });
}

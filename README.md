# 🐻 binturong

[![Crates.io](https://img.shields.io/crates/v/binturong)](https://crates.io/crates/binturong)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/iamkorun/binturong?style=social)](https://github.com/iamkorun/binturong)
  <a href="https://buymeacoffee.com/iamkorun"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?logo=buy-me-a-coffee&logoColor=black" alt="Buy Me a Coffee"></a>

**Spot config drift across your environments before it bites you.**

binturong compares multiple `.env` files side-by-side in a matrix table, so you can see at a glance which keys are present, missing, or empty across every environment.

---

## The Problem

You have `.env`, `.env.staging`, and `.env.production`. They've diverged. A key got added in one place but not the others. An empty value snuck into staging. Your new teammate deployed to prod and got a silent failure because `REDIS_URL` was missing.

Comparing two files manually is tedious. Comparing three or more is a guess. `diff` only shows two at a time.

## The Solution

binturong lays all your env files side-by-side in a matrix table and flags every key that drifted — across as many files as you have.

```
$ binturong .env .env.staging .env.production

┌─────────────────────┬───────────┬──────────────┬────────────────────┐
│ KEY                 │   .env    │ .env.staging │ .env.production    │
╞═════════════════════╪═══════════╪══════════════╪════════════════════╡
│ APP_ENV             │ ✓ ****    │ ✓ ****       │ ✓ ****             │
├─────────────────────┼───────────┼──────────────┼────────────────────┤
│ DATABASE_URL        │ ✓ ****    │ ✓ ****       │ ✓ ****             │
├─────────────────────┼───────────┼──────────────┼────────────────────┤
│ LOG_LEVEL           │ ✓ ****    │ ✓ ****       │ ✗ missing          │
├─────────────────────┼───────────┼──────────────┼────────────────────┤
│ REDIS_URL           │ ✓ ****    │ ✗ missing    │ ✗ missing          │
├─────────────────────┼───────────┼──────────────┼────────────────────┤
│ STRIPE_SECRET_KEY   │ ✓ ****    │ ○ (empty)    │ ✓ ****             │
└─────────────────────┴───────────┴──────────────┴────────────────────┘

✗ 3/5 keys drifted  (3 files)
```

Exit code 1 means drift detected. Pipe it into CI and sleep better.

---

## Quick Start

```sh
cargo install binturong
```

Then run it in any project directory:

```sh
binturong
```

binturong auto-discovers all `.env*` files in the current directory. No arguments needed.

---

## Installation

### From crates.io (recommended)

```sh
cargo install binturong
```

### From source

```sh
git clone https://github.com/iamkorun/binturong
cd binturong
cargo install --path .
```

---

## Usage

### Auto-discover (no arguments)

```sh
# Discovers all .env* files in the current directory
binturong
```

### Explicit files

```sh
binturong .env .env.staging .env.production
```

### Show only drifted keys (`--diff` / `-d`)

```sh
$ binturong --diff

┌───────────────────┬───────────┬──────────────┬────────────────────┐
│ KEY               │   .env    │ .env.staging │ .env.production    │
╞═══════════════════╪═══════════╪══════════════╪════════════════════╡
│ LOG_LEVEL         │ ✓ ****    │ ✓ ****       │ ✗ missing          │
├───────────────────┼───────────┼──────────────┼────────────────────┤
│ REDIS_URL         │ ✓ ****    │ ✗ missing    │ ✗ missing          │
├───────────────────┼───────────┼──────────────┼────────────────────┤
│ STRIPE_SECRET_KEY │ ✓ ****    │ ○ (empty)    │ ✓ ****             │
└───────────────────┴───────────┴──────────────┴────────────────────┘

✗ 3/5 keys drifted  (3 files)
```

### Reveal actual values (`--values`)

```sh
binturong --values
```

Values are masked (`****`) by default. Use `--values` to see them — useful for debugging mismatches.

### CI mode (`--quiet` / `-q`)

```sh
binturong --quiet
echo $?   # 0 = in sync, 1 = drift, 2 = error
```

No output; exits with a code you can act on in scripts or pipelines.

### Verbose mode (`--verbose` / `-v`)

```sh
binturong --verbose
# Shows each file being compared, and lists all drifted keys at the end
```

### All flags

| Flag            | Short | Description                                     |
|-----------------|-------|-------------------------------------------------|
| `--diff`        | `-d`  | Show only drifted keys                          |
| `--values`      |       | Reveal actual values (masked by default)        |
| `--quiet`       | `-q`  | No output; exit code only (CI-friendly)         |
| `--verbose`     | `-v`  | Show filenames being compared + drifted key list |
| `--help`        | `-h`  | Show help                                       |
| `--version`     | `-V`  | Show version                                    |

---

## Key Status Legend

| Symbol       | Meaning                           |
|--------------|-----------------------------------|
| `✓ ****`     | Key present (value masked)        |
| `✓ <value>`  | Key present (with `--values`)     |
| `○ (empty)`  | Key present but value is empty    |
| `✗ missing`  | Key not found in this file        |

---

## Exit Codes

| Code | Meaning                              |
|------|--------------------------------------|
| `0`  | All files are in sync                |
| `1`  | Drift detected                       |
| `2`  | Error (file not found, unreadable, etc.) |

---

## CI Integration

```yaml
# GitHub Actions example
- name: Check env drift
  run: binturong .env.example .env.staging .env.production --quiet
```

```sh
# Pre-commit hook
#!/bin/sh
binturong --quiet || { echo "Config drift detected. Run binturong to see details."; exit 1; }
```

---

## How It Compares to potto

binturong and [potto](https://github.com/iamkorun/potto) are complementary tools in the same toolkit:

| Tool          | Use case                                         |
|---------------|--------------------------------------------------|
| **potto**     | Keep `.env` and `.env.example` in sync (2 files) |
| **binturong** | Compare N env files across all environments      |

Use potto to validate your template. Use binturong when you need to audit staging, production, and beyond.

---

## Features

- **Matrix view** — all files side-by-side, all keys in one table
- **Auto-discovery** — finds all `.env*` files in the current directory automatically
- **N-file comparison** — not limited to two files; compare as many as you have
- **Drift detection** — flags keys that are missing, empty, or have different values
- **Value masking** — shows `****` by default to avoid leaking secrets in terminal output
- **`--diff` mode** — show only the keys that drifted, not the full matrix
- **CI-friendly** — `--quiet` flag + exit codes for scripting
- **Zero runtime** — single static binary, no dependencies to install

---

## Contributing

Issues and pull requests are welcome.

```sh
git clone https://github.com/iamkorun/binturong
cd binturong
cargo test
```

Please keep PRs focused. If you're adding a feature, open an issue first to discuss it.

---

## License

MIT — see [LICENSE](LICENSE).

---

<p align="center">
  <a href="https://buymeacoffee.com/iamkorun"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me a Coffee" width="200"></a>
</p>

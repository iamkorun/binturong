use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A parsed .env file: a filename plus an ordered list of key-value entries.
#[derive(Debug, Clone)]
pub struct EnvFile {
    pub path: PathBuf,
    /// Ordered list of keys as they appear in the file (deduped, last-wins kept).
    pub keys: Vec<String>,
    /// Key → value map. Empty string means the key has no value.
    pub entries: HashMap<String, String>,
}

impl EnvFile {
    pub fn filename(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }

    #[allow(dead_code)]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }
}

/// Parse a .env-style file from `path`.
///
/// Rules:
/// - Blank lines and comment lines (`#`) are skipped.
/// - `export KEY=VALUE` prefix is handled.
/// - Values may be single- or double-quoted; quotes are stripped.
/// - Inline comments (` # ...`) after unquoted values are stripped.
/// - Spaces around `=` are trimmed.
/// - Duplicate keys: last wins.
pub fn parse_env_file(path: &Path) -> Result<EnvFile, io::Error> {
    let content = fs::read_to_string(path)?;
    let (keys, entries) = parse_content(&content);
    Ok(EnvFile {
        path: path.to_path_buf(),
        keys,
        entries,
    })
}

/// Parse .env content from a string (useful in tests).
pub fn parse_content(content: &str) -> (Vec<String>, HashMap<String, String>) {
    let mut entries: HashMap<String, String> = HashMap::new();
    let mut key_order: Vec<String> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();

        // Skip blank lines and full-line comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Strip `export ` prefix
        let line = line.strip_prefix("export ").unwrap_or(line).trim_start();

        // Must contain `=`
        let Some(eq_pos) = line.find('=') else {
            continue;
        };

        let key = line[..eq_pos].trim().to_string();
        if key.is_empty() {
            continue;
        }

        let raw_value = line[eq_pos + 1..].trim();
        let value = parse_value(raw_value);

        if !entries.contains_key(&key) {
            key_order.push(key.clone());
        }
        entries.insert(key, value);
    }

    (key_order, entries)
}

/// Strip outer quotes and inline comments from a raw value string.
fn parse_value(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }

    // Double-quoted: consume until closing `"`, allow `\"` escapes
    if raw.starts_with('"') {
        return extract_quoted(raw, '"');
    }

    // Single-quoted: consume until closing `'` (no escapes inside single quotes)
    if raw.starts_with('\'') {
        return extract_quoted(raw, '\'');
    }

    // Unquoted: strip inline comment (` #` or `\t#` after whitespace)
    let without_comment = strip_inline_comment(raw);
    without_comment.trim_end().to_string()
}

/// Extract a quoted value (handles escape sequences for double-quoted strings).
fn extract_quoted(raw: &str, quote: char) -> String {
    let inner = &raw[1..]; // skip opening quote
    let mut result = String::new();
    let mut chars = inner.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' && quote == '"' {
            // Handle escape sequences inside double-quoted values
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some(c) => result.push(c),
                None => {}
            }
        } else if ch == quote {
            break;
        } else {
            result.push(ch);
        }
    }

    result
}

/// Strip an inline comment from an unquoted value.
/// A comment starts at the first ` #` or `\t#` sequence.
fn strip_inline_comment(s: &str) -> &str {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if (bytes[i] == b' ' || bytes[i] == b'\t') && i + 1 < len && bytes[i + 1] == b'#' {
            return &s[..i];
        }
        i += 1;
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_key_value() {
        let (keys, entries) = parse_content("FOO=bar\nBAZ=qux\n");
        assert_eq!(entries["FOO"], "bar");
        assert_eq!(entries["BAZ"], "qux");
        assert_eq!(keys, vec!["FOO", "BAZ"]);
    }

    #[test]
    fn test_skip_blank_and_comments() {
        let (keys, entries) = parse_content("# comment\n\nFOO=bar\n");
        assert_eq!(entries["FOO"], "bar");
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn test_export_prefix() {
        let (_, entries) = parse_content("export FOO=bar\n");
        assert_eq!(entries["FOO"], "bar");
    }

    #[test]
    fn test_spaces_around_equals() {
        let (_, entries) = parse_content("FOO = bar\n");
        assert_eq!(entries["FOO"], "bar");
    }

    #[test]
    fn test_double_quoted_value() {
        let (_, entries) = parse_content("FOO=\"hello world\"\n");
        assert_eq!(entries["FOO"], "hello world");
    }

    #[test]
    fn test_single_quoted_value() {
        let (_, entries) = parse_content("FOO='hello world'\n");
        assert_eq!(entries["FOO"], "hello world");
    }

    #[test]
    fn test_empty_value() {
        let (_, entries) = parse_content("FOO=\n");
        assert_eq!(entries["FOO"], "");
    }

    #[test]
    fn test_inline_comment_stripped() {
        let (_, entries) = parse_content("FOO=bar # this is a comment\n");
        assert_eq!(entries["FOO"], "bar");
    }

    #[test]
    fn test_inline_comment_not_stripped_in_quotes() {
        let (_, entries) = parse_content("FOO=\"bar # not a comment\"\n");
        assert_eq!(entries["FOO"], "bar # not a comment");
    }

    #[test]
    fn test_duplicate_keys_last_wins() {
        let (keys, entries) = parse_content("FOO=first\nFOO=second\n");
        assert_eq!(entries["FOO"], "second");
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn test_escape_sequences_in_double_quotes() {
        let (_, entries) = parse_content("FOO=\"hello\\nworld\"\n");
        assert_eq!(entries["FOO"], "hello\nworld");
    }

    #[test]
    fn test_no_equals_line_skipped() {
        let (keys, _) = parse_content("NOT_A_KEY\nFOO=bar\n");
        assert_eq!(keys, vec!["FOO"]);
    }

    #[test]
    fn test_empty_input() {
        let (keys, entries) = parse_content("");
        assert!(keys.is_empty());
        assert!(entries.is_empty());
    }

    #[test]
    fn test_windows_line_endings() {
        let (keys, entries) = parse_content("FOO=bar\r\nBAZ=qux\r\n");
        assert_eq!(entries["FOO"], "bar");
        assert_eq!(entries["BAZ"], "qux");
        assert_eq!(keys, vec!["FOO", "BAZ"]);
    }

    #[test]
    fn test_multibyte_utf8_values() {
        let (_, entries) = parse_content("GREETING=こんにちは\nEMOJI=🎉\n");
        assert_eq!(entries["GREETING"], "こんにちは");
        assert_eq!(entries["EMOJI"], "🎉");
    }

    #[test]
    fn test_value_with_equals_sign() {
        let (_, entries) = parse_content("URL=postgres://host/db?opt=val\n");
        assert_eq!(entries["URL"], "postgres://host/db?opt=val");
    }

    #[test]
    fn test_only_comments_and_blanks() {
        let (keys, entries) = parse_content("# just comments\n\n# more comments\n");
        assert!(keys.is_empty());
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_file_not_found() {
        let result = parse_env_file(Path::new("/nonexistent/.env.fake"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_real_file() {
        use std::io::Write;
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmpfile, "DATABASE_URL=postgres://localhost/db").unwrap();
        writeln!(tmpfile, "SECRET_KEY=abc123").unwrap();

        let ef = parse_env_file(tmpfile.path()).unwrap();
        assert_eq!(ef.get("DATABASE_URL"), Some("postgres://localhost/db"));
        assert_eq!(ef.get("SECRET_KEY"), Some("abc123"));
    }
}

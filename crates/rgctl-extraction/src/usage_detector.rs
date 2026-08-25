//! Config usage detector
//!
//! Task 1.5.1: Detect when code references configuration keys

use crate::graph_builder::ConfigUsageKind;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

static RUST_ENV_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"env::var(?:_os)?\("([^"]+)"\)"#).unwrap());
static RUST_CONFIG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\.get\("([^"]+)"\)"#).unwrap());
static PYTHON_ENV_BRACKET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"os\.environ\[['"]([^'"]+)['"]\]"#).unwrap());
static PYTHON_GETENV_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"os\.getenv\(['"]([^'"]+)['"]\)"#).unwrap());
static JS_DOT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"process\.env\.([A-Z0-9_]+)"#).unwrap());
static JS_BRACKET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"process\.env\[['"]([^'"]+)['"]\]"#).unwrap());
static GO_GETENV_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"os\.Getenv\("([^"]+)"\)"#).unwrap());

/// Confidence level for a detected config usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigConfidence {
    /// Directly extracted from source (e.g. string literal)
    Extracted,
    /// Inferred from context
    Inferred,
    /// Ambiguous match
    Ambiguous,
}

/// A detected configuration or environment variable usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigUsage {
    /// Config key or environment variable name
    pub key: String,
    /// Source file path
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Usage kind
    pub usage_type: ConfigUsageKind,
    /// Detection confidence
    pub confidence: ConfigConfidence,
}

/// Detects configuration and environment variable references in source code.
pub struct ConfigUsageDetector;

impl ConfigUsageDetector {
    /// Detect config usages for a supported language.
    pub fn detect(language_id: &str, source: &[u8], file_path: &Path) -> Vec<ConfigUsage> {
        match language_id {
            "rust" | "python" | "typescript" | "javascript" | "go" => {}
            _ => return Vec::new(),
        }

        let source = String::from_utf8_lossy(source);
        let file = file_path.to_string_lossy().to_string();

        match language_id {
            "rust" => Self::detect_rust(&source, &file),
            "python" => Self::detect_python(&source, &file),
            "typescript" | "javascript" => Self::detect_javascript(&source, &file),
            "go" => Self::detect_go(&source, &file),
            _ => Vec::new(),
        }
    }

    fn detect_rust(source: &str, file: &str) -> Vec<ConfigUsage> {
        let mut usages = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            for cap in RUST_ENV_RE.captures_iter(line) {
                usages.push(ConfigUsage {
                    key: cap[1].to_string(),
                    file: file.to_string(),
                    line: idx + 1,
                    usage_type: ConfigUsageKind::EnvVar,
                    confidence: ConfigConfidence::Extracted,
                });
            }
            for cap in RUST_CONFIG_RE.captures_iter(line) {
                usages.push(ConfigUsage {
                    key: cap[1].to_string(),
                    file: file.to_string(),
                    line: idx + 1,
                    usage_type: ConfigUsageKind::ConfigKey,
                    confidence: ConfigConfidence::Inferred,
                });
            }
        }
        usages
    }

    fn detect_python(source: &str, file: &str) -> Vec<ConfigUsage> {
        let mut usages = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            for cap in PYTHON_ENV_BRACKET_RE
                .captures_iter(line)
                .chain(PYTHON_GETENV_RE.captures_iter(line))
            {
                usages.push(ConfigUsage {
                    key: cap[1].to_string(),
                    file: file.to_string(),
                    line: idx + 1,
                    usage_type: ConfigUsageKind::EnvVar,
                    confidence: ConfigConfidence::Extracted,
                });
            }
        }
        usages
    }

    fn detect_javascript(source: &str, file: &str) -> Vec<ConfigUsage> {
        let mut usages = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            for cap in JS_DOT_RE
                .captures_iter(line)
                .chain(JS_BRACKET_RE.captures_iter(line))
            {
                usages.push(ConfigUsage {
                    key: cap[1].to_string(),
                    file: file.to_string(),
                    line: idx + 1,
                    usage_type: ConfigUsageKind::EnvVar,
                    confidence: ConfigConfidence::Extracted,
                });
            }
        }
        usages
    }

    fn detect_go(source: &str, file: &str) -> Vec<ConfigUsage> {
        let mut usages = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            for cap in GO_GETENV_RE.captures_iter(line) {
                usages.push(ConfigUsage {
                    key: cap[1].to_string(),
                    file: file.to_string(),
                    line: idx + 1,
                    usage_type: ConfigUsageKind::EnvVar,
                    confidence: ConfigConfidence::Extracted,
                });
            }
        }
        usages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_rust_config_detection() {
        let source = br#"
        fn main() {
            let host = env::var("DB_HOST").unwrap();
            let pool_size = config.get("database.pool_size").unwrap();
        }
        "#;

        let usages = ConfigUsageDetector::detect("rust", source, Path::new("main.rs"));
        assert!(
            usages
                .iter()
                .any(|u| u.key == "DB_HOST" && u.usage_type == ConfigUsageKind::EnvVar)
        );
        assert!(usages.iter().any(|u| u.key == "database.pool_size"));
    }

    #[test]
    fn test_python_config_detection() {
        let source = br#"
import os
host = os.environ['DB_HOST']
port = os.getenv('DB_PORT')
"#;

        let usages = ConfigUsageDetector::detect("python", source, Path::new("app.py"));
        assert!(usages.iter().any(|u| u.key == "DB_HOST"));
        assert!(usages.iter().any(|u| u.key == "DB_PORT"));
    }

    #[test]
    fn test_javascript_env_detection() {
        let source = br#"
const host = process.env.DB_HOST;
const port = process.env['DB_PORT'];
"#;

        let usages = ConfigUsageDetector::detect("javascript", source, Path::new("app.js"));
        assert!(usages.iter().any(|u| u.key == "DB_HOST"));
        assert!(usages.iter().any(|u| u.key == "DB_PORT"));
    }

    #[test]
    fn c_early_out_empty() {
        let src = b"int main(void) { return 0; }\n";
        let usages = ConfigUsageDetector::detect("c", src, Path::new("main.c"));
        assert!(usages.is_empty());
    }
}

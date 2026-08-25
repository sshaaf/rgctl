//! On-demand function body token extraction for semantic indexing.

use rgctl_error::{Error, Result};
use rgctl_graph::schema::Node;
pub use rgctl_graph::{
    MIN_TOKEN_LEN, TOKEN_BLOOM_BITS, TOKEN_BLOOM_WORDS, tokenize_string_into,
};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Body token sketch for one function (used by future fusion / keyword-AND stages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionTokenSketch {
    /// Graph node UUID.
    pub function_id: Uuid,
    /// Lowercased identifier tokens from the function body slice.
    pub tokens: HashSet<String>,
}

/// Resolve a repository-relative or absolute source path.
pub fn resolve_source_path(repo_root: &Path, file_path: &str) -> PathBuf {
    let path = Path::new(file_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

/// Extract body tokens for a graph function node when location metadata is present.
pub fn extract_body_tokens_for_node(repo_root: &Path, node: &Node) -> Result<HashSet<String>> {
    let file_path = node
        .file_path
        .as_deref()
        .ok_or_else(|| Error::NotFound("function has no file_path".into()))?;
    let start_line = node
        .start_line
        .ok_or_else(|| Error::NotFound("function has no start_line".into()))?;
    let end_line = node
        .end_line
        .ok_or_else(|| Error::NotFound("function has no end_line".into()))?;
    extract_body_tokens_from_slice(repo_root, file_path, start_line, end_line)
}

/// Read `start_line..=end_line` from a source file and tokenize identifier-like chunks.
pub fn extract_body_tokens_from_slice(
    repo_root: &Path,
    file_path: &str,
    start_line: usize,
    end_line: usize,
) -> Result<HashSet<String>> {
    if start_line == 0 || end_line == 0 || end_line < start_line {
        return Err(Error::ConfigError(format!(
            "invalid line range {start_line}..={end_line} for {file_path}"
        )));
    }

    let full_path = resolve_source_path(repo_root, file_path);
    if !full_path.is_file() {
        return Err(Error::NotFound(format!(
            "source file missing: {}",
            full_path.display()
        )));
    }

    let file = File::open(&full_path).map_err(Error::IoError)?;
    let reader = BufReader::new(file);
    let mut tokens = HashSet::new();

    for (idx, line_result) in reader.lines().enumerate() {
        let current_line = idx + 1;
        if current_line > end_line {
            break;
        }
        if current_line >= start_line {
            let line_text = line_result.map_err(Error::IoError)?;
            tokenize_string_into(&line_text, &mut tokens);
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgctl_graph::schema::{Node, NodeType};
    use std::io::Write;

    #[test]
    fn tokenize_splits_camel_case_and_snake_case() {
        let mut tokens = HashSet::new();
        tokenize_string_into("parseIncomingPacket sk_buff ntohs", &mut tokens);
        assert!(tokens.contains("parse"));
        assert!(tokens.contains("incoming"));
        assert!(tokens.contains("packet"));
        assert!(tokens.contains("buff"));
        assert!(tokens.contains("ntohs"));
        assert!(!tokens.contains("sk"));
    }

    #[test]
    fn extract_body_tokens_from_line_range() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("net.rs");
        let mut file = std::fs::File::create(&file_path).unwrap();
        writeln!(file, "// header").unwrap();
        writeln!(file, "fn helper() {{").unwrap();
        writeln!(file, "    let hdr: *const ethhdr = ptr;").unwrap();
        writeln!(file, "    let port = ntohs(raw);").unwrap();
        writeln!(file, "}}").unwrap();

        let tokens =
            extract_body_tokens_from_slice(dir.path(), file_path.to_str().unwrap(), 2, 4).unwrap();
        assert!(tokens.contains("helper"));
        assert!(tokens.contains("ethhdr"));
        assert!(tokens.contains("ntohs"));
    }

    #[test]
    fn extract_body_tokens_for_node_uses_graph_location() {
        let dir = tempfile::tempdir().unwrap();
        let rel = "src/packet.rs";
        let abs = dir.path().join(rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(&abs, "fn process_sk_buff() {\n    ntohs(value);\n}\n").unwrap();

        let node = Node::new(NodeType::Function, "process_sk_buff")
            .with_file_path(rel)
            .with_location(1, 2);

        let tokens = extract_body_tokens_for_node(dir.path(), &node).unwrap();
        assert!(tokens.contains("ntohs"));
        assert!(tokens.contains("process"));
        assert!(tokens.contains("buff"));
    }
}

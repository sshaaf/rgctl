//! `builtin.file` glob evaluation.

use crate::eval::{MatchSite, violation};
use crate::findings::KantraViolation;
use glob::Pattern;
use std::path::{Path, PathBuf};

/// Match file paths against a glob pattern.
pub fn eval_file(
    rule_id: &str,
    pattern: &str,
    repo_root: &Path,
    files: &[PathBuf],
) -> Result<Vec<KantraViolation>, glob::PatternError> {
    let glob = Pattern::new(pattern)?;
    let mut out = Vec::new();
    for file in files {
        let rel = file
            .strip_prefix(repo_root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        if glob.matches(&rel) || glob.matches(file.to_string_lossy().as_ref()) {
            out.push(violation(
                rule_id,
                "builtin.file",
                &MatchSite::new(&rel, 1),
            ));
        }
    }
    Ok(out)
}

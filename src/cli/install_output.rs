//! Structured `rgctl install` JSON response.

use serde::{Deserialize, Serialize};

/// Current install JSON schema version.
pub const INSTALL_SCHEMA_VERSION: u32 = 1;

/// Agent host that received a skill file write.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallWriteHost {
    /// Claude Code project skills directory.
    Claude,
    /// Cursor project skills directory.
    Cursor,
}

/// Outcome of one destination file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallWriteStatus {
    /// File did not exist and was written.
    Created,
    /// Existing file matched the bundle; left unchanged.
    Unchanged,
    /// Existing file was replaced (`--force`, or a symlink converted to a regular file).
    Overwritten,
    /// Existing file differed and `--force` was not set.
    SkippedExists,
}

/// One planned or completed file write.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallWrite {
    /// Agent host for this dest path.
    pub host: InstallWriteHost,
    /// Absolute destination path.
    pub path: String,
    /// Write outcome.
    pub status: InstallWriteStatus,
}

/// Top-level install JSON payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallJsonResponse {
    /// Schema version (1).
    pub schema_version: u32,
    /// Always `"install"`.
    pub command: String,
    /// Bundled skill id (`rgctl`).
    pub skill: String,
    /// Absolute repository root used as install prefix.
    pub repo: String,
    /// Whether `--force` was set.
    pub force: bool,
    /// Per-file results.
    pub writes: Vec<InstallWrite>,
}

/// Build the install response object.
pub fn build_install_response(
    repo: &str,
    force: bool,
    writes: Vec<InstallWrite>,
) -> InstallJsonResponse {
    InstallJsonResponse {
        schema_version: INSTALL_SCHEMA_VERSION,
        command: "install".into(),
        skill: "rgctl".into(),
        repo: repo.to_string(),
        force,
        writes,
    }
}

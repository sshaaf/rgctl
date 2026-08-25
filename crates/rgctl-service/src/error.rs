//! Errors from command execution.

use thiserror::Error;

/// Command service failure (not unreadiness — that is an `Ok` status JSON).
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Tool/command arguments are invalid.
    #[error("invalid params: {0}")]
    InvalidParams(String),
    /// Multiplexed `op` is not registered.
    #[error("unknown op '{op}'. allowed: {allowed}")]
    UnknownOp {
        /// Requested operation.
        op: String,
        /// Comma-separated allowed ops.
        allowed: String,
    },
    /// Execution failed (parse, I/O, analysis).
    #[error("{0}")]
    Failed(String),
}

impl ServiceError {
    /// Unknown CPG (or other family) op with a list of allowed names.
    #[must_use]
    pub fn unknown_op(op: impl Into<String>, allowed: &[&str]) -> Self {
        Self::UnknownOp {
            op: op.into(),
            allowed: allowed.join(", "),
        }
    }
}

impl From<anyhow::Error> for ServiceError {
    fn from(err: anyhow::Error) -> Self {
        Self::Failed(err.to_string())
    }
}

impl From<rgctl_error::Error> for ServiceError {
    fn from(err: rgctl_error::Error) -> Self {
        Self::Failed(err.to_string())
    }
}

impl From<serde_json::Error> for ServiceError {
    fn from(err: serde_json::Error) -> Self {
        Self::Failed(err.to_string())
    }
}

/// Result of a service command.
pub type Result<T> = std::result::Result<T, ServiceError>;

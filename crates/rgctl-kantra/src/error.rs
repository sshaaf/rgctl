//! Kantra evaluator errors.

use thiserror::Error;

/// Errors from ruleset loading or evaluation.
#[derive(Debug, Error)]
pub enum KantraError {
    /// I/O or parse failure.
    #[error("{0}")]
    Msg(String),
}

impl KantraError {
    /// Wrap a message.
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Msg(s.into())
    }
}

impl From<std::io::Error> for KantraError {
    fn from(e: std::io::Error) -> Self {
        Self::Msg(e.to_string())
    }
}

impl From<serde_yaml::Error> for KantraError {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Msg(e.to_string())
    }
}

impl From<regex::Error> for KantraError {
    fn from(e: regex::Error) -> Self {
        Self::Msg(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, KantraError>;

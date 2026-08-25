//! Code graph types and helpers (re-exported from `rgctl-graph`).

pub use rgctl_graph::CodeGraph;
pub use rgctl_graph::*;

use rgctl_error::Result;
use std::path::Path;

/// Build a code graph from a repository (workspace entry point).
pub fn from_repository(root: &Path) -> Result<CodeGraph> {
    crate::code_graph_from_repository(root)
}

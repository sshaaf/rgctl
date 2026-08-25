//! rgctl - AI-Powered Code Knowledge Graph

#![warn(missing_docs)]
#![warn(clippy::all)]

pub use rgctl_core::*;

pub mod analysis;
#[allow(missing_docs)]
pub mod cli;
pub mod graph;
pub mod languages;
pub mod security;

pub use rgctl_error::{Error, Result};
pub use rgctl_graph::CodeGraph;

/// Build information
pub const BUILD_INFO: &str = concat!(
    "rgctl v",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

/// Initialize workspace hooks (language registry builder).
pub fn init() {
    languages::ensure_registry_initialized();
}

/// Build a code graph from a repository using all built-in language plugins.
pub fn code_graph_from_repository(root: &std::path::Path) -> Result<CodeGraph> {
    use rgctl_pipeline::ProcessingPipeline;
    use std::sync::Arc;

    languages::ensure_registry_initialized();
    let pipeline =
        ProcessingPipeline::new(Arc::new(languages::LanguageRegistry::new().into_inner()));
    let (graph, _) = pipeline.process_repository(root)?;
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_build_info() {
        assert!(BUILD_INFO.contains("rgctl"));
    }
}

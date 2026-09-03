//! Parallel processing pipeline

pub mod parallel;
mod pipeline;
pub mod stream;

pub use parallel::{
    large_stack_thread_pool, par_filter_map, par_map, thread_pool, with_large_pool,
    with_large_stack, with_pool,
};
pub use pipeline::{PipelineConfig, PipelineStats, ProcessingPipeline};
pub use stream::{DEFAULT_STREAM_CHANNEL_CAPACITY, stream_into_graph};

use rgctl_error::Result;
use rgctl_graph::CodeGraph;
use std::path::Path;
use std::sync::Arc;

/// Build a code graph from a repository path using the default registry and pipeline.
pub fn code_graph_from_repository(root: &Path) -> Result<CodeGraph> {
    let pipeline = ProcessingPipeline::new(Arc::new(rgctl_registry::full_registry()));
    let (graph, _) = pipeline.process_repository(root)?;
    Ok(graph)
}

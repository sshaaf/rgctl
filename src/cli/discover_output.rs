//! Structured discover CLI JSON response.

use crate::pipeline::PipelineStats;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current discover JSON schema version.
pub const DISCOVER_SCHEMA_VERSION: u32 = 2;

/// Ingestion counters emitted after a successful discover pass.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoverMetrics {
    pub files_discovered: usize,
    pub files_indexed: usize,
    pub files_skipped: usize,
    pub nodes_generated: usize,
    pub edges_generated: usize,
    pub duration_ms: u64,
}

/// Top-level discover JSON payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoverJsonResponse {
    pub schema_version: u32,
    pub command: String,
    pub metrics: DiscoverMetrics,
    /// Present when `discover --full` ran the staged pipeline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full: Option<bool>,
    /// Stage plan when `full` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<Vec<super::pipeline_status::PipelineStagePlan>>,
}

/// Build the discover telemetry block from pipeline stats and wall-clock duration.
pub fn build_discover_response(stats: &PipelineStats, duration_ms: u64) -> DiscoverJsonResponse {
    DiscoverJsonResponse {
        schema_version: DISCOVER_SCHEMA_VERSION,
        command: "discover".into(),
        full: None,
        plan: None,
        metrics: DiscoverMetrics {
            files_discovered: stats.files_discovered,
            files_indexed: stats.files_processed,
            files_skipped: stats.files_failed,
            nodes_generated: stats.nodes_created,
            edges_generated: stats.edges_created,
            duration_ms,
        },
    }
}

pub fn discover_response_to_json(response: &DiscoverJsonResponse) -> Value {
    serde_json::to_value(response).expect("DiscoverJsonResponse serializes")
}

/// Fixture for schema sanity tests.
pub fn fixture_discover_response() -> DiscoverJsonResponse {
    build_discover_response(
        &PipelineStats {
            files_discovered: 10_921,
            files_processed: 10_784,
            files_failed: 137,
            nodes_created: 231_410,
            edges_created: 562_067,
            duration: std::time::Duration::from_millis(18_200),
            extract_duration: std::time::Duration::from_millis(12_000),
            graph_build_duration: std::time::Duration::from_millis(6_200),
        },
        18_200,
    )
}

pub fn fixture_discover_json() -> Value {
    discover_response_to_json(&fixture_discover_response())
}

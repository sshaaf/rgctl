//! Structured semantic search CLI JSON response.

use rgbuilder_analysis::{
    SEMANTIC_INDEX_SCHEMA_VERSION, SemanticBuildStats, SemanticEntry, SemanticExpansion,
    SemanticHit,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Current semantic CLI JSON schema version for index responses.
pub const SEMANTIC_INDEX_CLI_SCHEMA_VERSION: u32 = 2;

/// Current semantic CLI JSON schema version for query responses.
pub const SEMANTIC_QUERY_CLI_SCHEMA_VERSION: u32 = 3;

/// One ranked semantic search hit in JSON output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticHitJson {
    pub node_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    pub distance: u32,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fused_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking: Option<String>,
}

/// Index build counters for incremental runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticBuildStatsJson {
    pub total: usize,
    pub reused: usize,
    pub embedded: usize,
    pub removed: usize,
}

/// Payload for `rg-build semantic query`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticQueryJsonResponse {
    pub schema_version: u32,
    pub query: String,
    pub model_id: String,
    pub dimensions: usize,
    pub hits: Vec<SemanticHitJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<SemanticExpansion>,
}

/// Payload for `rg-build semantic index`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticIndexJsonResponse {
    pub schema_version: u32,
    pub model_id: String,
    pub dimensions: usize,
    pub functions_indexed: usize,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_stats: Option<SemanticBuildStatsJson>,
}

/// Convert Hamming distance to a simple similarity score in (0, 1].
pub fn distance_to_score(distance: u32, dimensions: usize) -> f64 {
    if dimensions == 0 {
        return 0.0;
    }
    1.0 - (distance as f64 / dimensions as f64)
}

pub fn hit_to_json(entry: &SemanticEntry, distance: u32, dimensions: usize) -> SemanticHitJson {
    hit_from_semantic(entry, distance, dimensions, None)
}

pub fn hit_from_semantic(
    entry: &SemanticEntry,
    distance: u32,
    dimensions: usize,
    hit: Option<&SemanticHit>,
) -> SemanticHitJson {
    let fused_score = hit.and_then(|h| h.fused_score);
    let score = fused_score.unwrap_or_else(|| distance_to_score(distance, dimensions));
    SemanticHitJson {
        node_id: entry.node_id.to_string(),
        name: entry.name.clone(),
        qualified_name: entry.qualified_name.clone(),
        file_path: entry.file_path.clone(),
        distance,
        score,
        fused_score,
        ranking: if fused_score.is_some() {
            Some("fusion".into())
        } else {
            None
        },
    }
}

pub fn build_stats_to_json(stats: SemanticBuildStats) -> SemanticBuildStatsJson {
    SemanticBuildStatsJson {
        total: stats.total,
        reused: stats.reused,
        embedded: stats.embedded,
        removed: stats.removed,
    }
}

pub fn build_query_response(
    query: &str,
    model_id: &str,
    dimensions: usize,
    hits: Vec<SemanticHitJson>,
    expansion: Option<SemanticExpansion>,
) -> SemanticQueryJsonResponse {
    SemanticQueryJsonResponse {
        schema_version: SEMANTIC_QUERY_CLI_SCHEMA_VERSION,
        query: query.to_string(),
        model_id: model_id.to_string(),
        dimensions,
        hits,
        expansion,
    }
}

pub fn build_index_response(
    model_id: &str,
    dimensions: usize,
    functions_indexed: usize,
    path: &str,
    graph_digest: Option<String>,
    build_stats: Option<SemanticBuildStats>,
) -> SemanticIndexJsonResponse {
    SemanticIndexJsonResponse {
        schema_version: SEMANTIC_INDEX_CLI_SCHEMA_VERSION,
        model_id: model_id.to_string(),
        dimensions,
        functions_indexed,
        path: path.to_string(),
        graph_digest,
        build_stats: build_stats.map(build_stats_to_json),
    }
}

pub fn query_response_to_json(response: &SemanticQueryJsonResponse) -> Value {
    let mut value = serde_json::to_value(response).expect("SemanticQueryJsonResponse serializes");
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "index_schema_version".into(),
            json!(SEMANTIC_INDEX_SCHEMA_VERSION),
        );
    }
    value
}

pub fn index_response_to_json(response: &SemanticIndexJsonResponse) -> Value {
    serde_json::to_value(response).expect("SemanticIndexJsonResponse serializes")
}

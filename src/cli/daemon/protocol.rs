//! Line-oriented JSON control protocol (Unix socket / Windows port).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ControlRequest {
    Ping,
    Shutdown,
    Status,
    List,
    Discover(DiscoverRequest),
    Gql {
        repo: String,
        query: String,
        #[serde(default)]
        explain: bool,
        #[serde(default)]
        macro_name: Option<String>,
    },
    Impact {
        repo: String,
        symbol: String,
        #[serde(default)]
        depth: Option<usize>,
        #[serde(default)]
        class: Option<String>,
        #[serde(default)]
        file: Option<String>,
    },
    Metrics {
        repo: String,
        #[serde(default)]
        pagerank: bool,
        #[serde(default)]
        betweenness: bool,
        #[serde(default)]
        communities: bool,
    },
    Check {
        repo: String,
        policy_file: String,
    },
    McpCall {
        repo: String,
        tool: String,
        arguments: serde_json::Value,
    },
    McpRpc {
        message: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverRequest {
    pub source: String,
    #[serde(default)]
    pub languages: Option<String>,
    #[serde(default)]
    pub exclude: Option<String>,
    #[serde(default)]
    pub with_security: bool,
    #[serde(default)]
    pub with_cfg: bool,
    #[serde(default)]
    pub with_taint: bool,
    #[serde(default)]
    pub with_dfg_loops: bool,
    #[serde(default)]
    pub with_ast_skeleton: bool,
    #[serde(default)]
    pub write_json_graph: bool,
    #[serde(default)]
    pub with_dashboard: bool,
    #[serde(default)]
    pub export_migration_hints: bool,
    #[serde(default)]
    pub with_harmonic: bool,
    #[serde(default)]
    pub full: bool,
    #[serde(default)]
    pub migration_preset: String,
    #[serde(default)]
    pub migration_order: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoListItem {
    pub name: String,
    pub source: String,
    pub status: String,
}

pub type RepoListEntry = RepoListItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "ty", rename_all = "snake_case")]
pub enum ControlResponse {
    Pong,
    Ok { message: String },
    Json { value: serde_json::Value },
    Err { message: String },
    Status {
        pid: u32,
        http: String,
        mcp: String,
        repos: usize,
    },
    List { repos: Vec<RepoListItem> },
}

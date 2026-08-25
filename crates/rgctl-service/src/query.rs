//! GQL / macro query command.

use crate::command::QueryArgs;
use crate::error::{Result, ServiceError};
use crate::gql_json::{GqlJsonResponse, gql_response_from_result};
use rgctl_analysis::{AnalysisResults, CommunityQueryContext};
use rgctl_gql::{
    QueryMacroRegistry, execute_explain_with_community, execute_macro_with_community,
    execute_with_community,
};
use rgctl_graph::CodeGraph;
use rgctl_graph::backend::GraphBackend;
use serde_json::Value;
use std::path::Path;

/// Validate exclusive `query` vs `macro` before touching the graph.
pub fn validate_query_args(args: &QueryArgs) -> Result<()> {
    let has_query = args.query.as_ref().is_some_and(|q| !q.trim().is_empty());
    let has_macro = args
        .macro_name
        .as_ref()
        .is_some_and(|m| !m.trim().is_empty());
    if has_query && has_macro {
        return Err(ServiceError::InvalidParams(
            "pass either `query` or `macro`, not both".into(),
        ));
    }
    if !has_query && !has_macro {
        return Err(ServiceError::InvalidParams(
            "request must include `query` or `macro`".into(),
        ));
    }
    Ok(())
}

/// Execute GQL on a loaded graph. `limit` `None` means no row cap.
pub fn run_query(graph: &CodeGraph, repo: &Path, args: &QueryArgs) -> Result<Value> {
    validate_query_args(args)?;

    let backend = graph.backend();
    let registry = QueryMacroRegistry::with_defaults();
    let community = load_community_context(repo, backend);
    let result = if let Some(name) = args.macro_name.as_deref().filter(|s| !s.is_empty()) {
        execute_macro_with_community(backend, &registry, name, community.as_ref())?
    } else if args.explain {
        let q = args.query.as_deref().unwrap_or("");
        execute_explain_with_community(backend, q, community.as_ref())?
    } else {
        let q = args.query.as_deref().unwrap_or("");
        execute_with_community(backend, q, community.as_ref())?
    };

    let mut response: GqlJsonResponse = gql_response_from_result(&result, args.explain);
    if let Some(limit) = args.limit {
        response.rows.truncate(limit);
        response.count = response.rows.len();
    }
    serde_json::to_value(&response).map_err(ServiceError::from)
}

fn load_community_context(
    repo: &Path,
    backend: &rgctl_graph::backend::MemoryBackend,
) -> Option<CommunityQueryContext> {
    let path = rgctl_graph::paths::artifact_path(repo, "analysis_results.bin");
    if !path.is_file() {
        return None;
    }
    let analysis = AnalysisResults::load(&path).ok()?;
    Some(CommunityQueryContext::from_analysis(&analysis, |uuid| {
        backend.get_node(uuid).ok().flatten().map(|n| {
            (
                n.name.to_string(),
                n.file_path.as_ref().map(|s| s.to_string()),
            )
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::QueryArgs;

    #[test]
    fn rejects_both_query_and_macro() {
        let err = validate_query_args(&QueryArgs {
            query: Some("MATCH (n:Function) RETURN n".into()),
            macro_name: Some("all_functions".into()),
            explain: false,
            limit: None,
        })
        .unwrap_err();
        assert!(
            matches!(err, ServiceError::InvalidParams(ref msg) if msg.contains("not both")),
            "{err}"
        );
    }

    #[test]
    fn rejects_neither_query_nor_macro() {
        let err = validate_query_args(&QueryArgs {
            query: None,
            macro_name: None,
            explain: false,
            limit: None,
        })
        .unwrap_err();
        assert!(matches!(err, ServiceError::InvalidParams(_)), "{err}");
    }

    #[test]
    fn accepts_macro_only() {
        validate_query_args(&QueryArgs {
            query: None,
            macro_name: Some("all_functions".into()),
            explain: false,
            limit: Some(20),
        })
        .unwrap();
    }
}

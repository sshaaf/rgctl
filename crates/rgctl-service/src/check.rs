//! Policy check command.

use crate::check_json::{build_check_response, check_response_to_json, violations_from_json_values};
use crate::command::CheckArgs;
use crate::error::{Result, ServiceError};
use crate::policy::PolicyFile;
use rgctl_analysis::{BlastRadiusEngine, CentralityAnalyzer, PetGraphView, PolicyViolation};
use rgctl_graph::CodeGraph;
use serde_json::{Value, json};
use std::path::Path;

/// Run CI policy check. Missing policy file is invalid-params.
pub fn run_check(graph: &CodeGraph, repo: &Path, args: &CheckArgs) -> Result<Value> {
    let path = Path::new(&args.policy_file);
    if !path.is_file() {
        return Err(ServiceError::InvalidParams(format!(
            "policy file not found: {}",
            args.policy_file
        )));
    }
    let policy = PolicyFile::load(path).map_err(|e| ServiceError::InvalidParams(e.to_string()))?;
    let strict = args.strict || policy.scope.strict_diff;
    let registry = policy.into_registry();
    let centrality_threshold = registry.centrality_alert_threshold;
    let backend = graph.backend();
    let view = PetGraphView::from_backend(backend).map_err(ServiceError::from)?;
    let centrality = CentralityAnalyzer::new()
        .analyze_with_view(&view)
        .map_err(ServiceError::from)?
        .scores;
    let engine = BlastRadiusEngine::build(backend).map_err(ServiceError::from)?;
    let symbols = rgctl_incremental::changed_function_symbols(
        repo,
        backend,
        &rgctl_incremental::SymbolScopeOptions {
            base_ref: args.base_ref.clone(),
            head_ref: args.head_ref.clone(),
            strict,
        },
    )
    .map_err(|e| ServiceError::InvalidParams(e.to_string()))?;
    let mut violation_rows = Vec::new();

    for symbol in symbols {
        let Ok((id, _)) = rgctl_analysis::resolve_unique_symbol(backend, &symbol) else {
            continue;
        };
        if let Err(err) =
            engine.analyze_with_policy(id, Some(backend), Some(&registry), Some(&centrality))
        {
            violation_rows.push(json!({
                "symbol": symbol,
                "error": err.to_string(),
            }));
            continue;
        }
        if let Ok(result) = engine.analyze(id) {
            for node_id in &result.impact_zone_ids {
                if let Some(score) = centrality.get(node_id) {
                    if score.betweenness > centrality_threshold {
                        violation_rows.push(json!({
                            "symbol": symbol,
                            "violation": format!("{}", PolicyViolation::CascadeHazard {
                                node: *node_id,
                                betweenness: score.betweenness,
                                threshold: centrality_threshold,
                            }),
                        }));
                    }
                }
            }
        }
    }

    let response = build_check_response(
        &args.policy_file,
        violations_from_json_values(&violation_rows),
    );
    Ok(check_response_to_json(&response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CheckArgs;
    use rgctl_graph::schema::{Node, NodeType};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_git_repo(dir: &Path) {
        for args in [
            ["init", "-b", "main"],
            ["config", "user.email", "test@example.com"],
            ["config", "user.name", "test"],
        ] {
            let out = Command::new("git")
                .args(args)
                .current_dir(dir)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .output()
                .unwrap();
            assert!(out.status.success(), "git {:?} failed", args);
        }
    }

    fn git_commit_all(dir: &Path, message: &str) {
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        let out = Command::new("git")
            .args(["-c", "commit.gpgsign=false", "commit", "-m", message])
            .current_dir(dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(out.status.success());
    }

    fn graph_with_function(name: &str, file: &str) -> CodeGraph {
        let mut graph = CodeGraph::new();
        graph
            .load(
                vec![Node::new(NodeType::Function, name).with_file_path(file)],
                vec![],
            )
            .unwrap();
        graph
    }

    #[test]
    fn strict_empty_commit_diff_returns_invalid_params() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        fs::write(tmp.path().join("f.rs"), "fn x() {}\n").unwrap();
        git_commit_all(tmp.path(), "only");

        let policy_path = tmp.path().join("policy.json");
        fs::write(&policy_path, r#"{"max_impact_nodes": 100}"#).unwrap();

        let graph = graph_with_function("x", "f.rs");
        let err = run_check(
            &graph,
            tmp.path(),
            &CheckArgs {
                policy_file: policy_path.to_string_lossy().into_owned(),
                base_ref: Some("HEAD".into()),
                head_ref: Some("HEAD".into()),
                strict: true,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("strict"));
    }

    #[test]
    fn non_strict_empty_diff_falls_back_to_all_functions() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        fs::write(tmp.path().join("f.rs"), "fn x() {}\n").unwrap();
        git_commit_all(tmp.path(), "only");

        let policy_path = tmp.path().join("policy.json");
        fs::write(
            &policy_path,
            r#"{"max_impact_nodes": 1000000, "centrality_alert_threshold": 1e12}"#,
        )
        .unwrap();

        let graph = graph_with_function("x", "f.rs");
        let value = run_check(
            &graph,
            tmp.path(),
            &CheckArgs {
                policy_file: policy_path.to_string_lossy().into_owned(),
                base_ref: Some("HEAD".into()),
                head_ref: Some("HEAD".into()),
                strict: false,
            },
        )
        .unwrap();
        assert_eq!(value["passed"].as_bool(), Some(true));
    }
}


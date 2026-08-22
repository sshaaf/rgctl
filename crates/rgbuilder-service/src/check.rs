//! Policy check command.

use crate::check_json::{build_check_response, check_response_to_json, violations_from_json_values};
use crate::command::CheckArgs;
use crate::error::{Result, ServiceError};
use crate::policy::PolicyFile;
use rgbuilder_analysis::{BlastRadiusEngine, CentralityAnalyzer, PetGraphView, PolicyViolation};
use rgbuilder_graph::CodeGraph;
use rgbuilder_graph::schema::NodeType;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

/// Run CI policy check. Missing policy file is invalid-params.
pub fn run_check(graph: &CodeGraph, repo: &Path, args: &CheckArgs) -> Result<Value> {
    let path = Path::new(&args.policy_file);
    if !path.is_file() {
        return Err(ServiceError::InvalidParams(format!(
            "policy file not found: {}",
            args.policy_file
        )));
    }
    let registry = PolicyFile::load(path)
        .map_err(|e| ServiceError::InvalidParams(e.to_string()))?
        .into_registry();
    let centrality_threshold = registry.centrality_alert_threshold;
    let backend = graph.backend();
    let view = PetGraphView::from_backend(backend).map_err(ServiceError::from)?;
    let centrality = CentralityAnalyzer::new()
        .analyze_with_view(&view)
        .map_err(ServiceError::from)?
        .scores;
    let engine = BlastRadiusEngine::build(backend).map_err(ServiceError::from)?;
    let symbols = changed_function_symbols(repo, backend)?;
    let mut violation_rows = Vec::new();

    for symbol in symbols {
        let Ok((id, _)) = rgbuilder_analysis::resolve_unique_symbol(backend, &symbol) else {
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

fn changed_function_symbols(
    repo: &Path,
    backend: &rgbuilder_graph::backend::MemoryBackend,
) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(repo)
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let files = String::from_utf8_lossy(&out.stdout);
            let paths: Vec<String> = files
                .lines()
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect();
            if !paths.is_empty() {
                let mut symbols = Vec::new();
                for node in backend.all_nodes().map_err(ServiceError::from)? {
                    if node.node_type != NodeType::Function {
                        continue;
                    }
                    if let Some(ref fp) = node.file_path {
                        if paths
                            .iter()
                            .any(|p| fp.ends_with(p) || p.ends_with(fp.as_str()))
                        {
                            symbols.push(node.name.to_string());
                        }
                    }
                }
                if !symbols.is_empty() {
                    return Ok(symbols);
                }
            }
        }
    }

    Ok(backend
        .collect_nodes_by_type(NodeType::Function)
        .map_err(ServiceError::from)?
        .into_iter()
        .map(|n| n.name.to_string())
        .collect())
}

//! MCP `tools/call` coverage on an indexed repo (happy path for each catalog tool).

mod rgctl_harness;

use rgctl_harness::{
    assert_ok, cli_json, materialize_fixture, mcp_connect_stdio,
    mcp_structured, run_no_daemon_in_repo,
};
use serde_json::Value;
use std::fs;
use std::path::Path;

fn discover_and_index(repo: &Path) {
    assert_ok(
        &run_no_daemon_in_repo(
            repo,
            &["discover", ".", "--languages", "java,rust"],
        ),
        "discover",
    );
    assert_ok(
        &run_no_daemon_in_repo(repo, &["semantic", "index"]),
        "semantic index",
    );
    let policy = Path::new(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/rgctl-policy.json");
    fs::copy(&policy, repo.join("policy.json")).expect("copy policy");
}

fn assert_not_status(doc: &Value, tool: &str) {
    assert_ne!(
        doc.get("command").and_then(Value::as_str),
        Some("pipeline_status"),
        "{tool} should execute, not return unreadiness: {doc}"
    );
}

#[test]
fn mcp_each_catalog_tool_returns_cli_shaped_json() {
    let (_tmp, repo) = materialize_fixture();
    discover_and_index(&repo);
    let mut mcp = mcp_connect_stdio(&repo);

    let status = mcp_structured(&mcp.call("rgctl_status", serde_json::json!({})));
    assert_eq!(status["schema_version"], 1);
    assert_eq!(status["command"], "pipeline_status");

    let match_q = "MATCH (n:Function) RETURN n LIMIT 5";
    let query = mcp_structured(&mcp.call(
        "rgctl_query",
        serde_json::json!({ "query": match_q }),
    ));
    assert_not_status(&query, "rgctl_query MATCH");
    assert_eq!(query["schema_version"], 1);
    assert!(query["rows"].is_array(), "{query}");
    assert!(query["count"].as_u64().unwrap_or(0) <= 5, "{query}");
    let cli_query = cli_json(&repo, &["gql", match_q]);
    assert_eq!(query["schema_version"], cli_query["schema_version"]);
    assert_eq!(query["count"], cli_query["count"]);

    let macro_q = mcp_structured(&mcp.call(
        "rgctl_query",
        serde_json::json!({ "macro": "all_functions", "limit": 5 }),
    ));
    assert_not_status(&macro_q, "rgctl_query macro");
    assert!(macro_q["count"].as_u64().unwrap_or(99) <= 5, "{macro_q}");
    assert_eq!(
        macro_q["rows"].as_array().map(Vec::len).unwrap_or(0) as u64,
        macro_q["count"].as_u64().unwrap_or(0)
    );

    let search = mcp_structured(&mcp.call(
        "rgctl_search",
        serde_json::json!({ "text": "order", "scope": "function", "limit": 5 }),
    ));
    assert_not_status(&search, "rgctl_search");
    assert_eq!(search["schema_version"], 3);
    assert_eq!(search["query"], "order");
    assert!(search["hits"].is_array(), "{search}");
    assert!(search["hits"].as_array().map(Vec::len).unwrap_or(99) <= 5);
    let cli_search = cli_json(
        &repo,
        &["semantic", "query", "order", "--limit", "5", "--scope", "function"],
    );
    assert_eq!(search["schema_version"], cli_search["schema_version"]);

    let impact = mcp_structured(&mcp.call(
        "rgctl_impact",
        serde_json::json!({ "symbol": "OrderService::process" }),
    ));
    assert_not_status(&impact, "rgctl_impact");
    assert_eq!(impact["schema_version"], 2);
    for key in ["score", "direct_callers_count", "impact_zone_size"] {
        assert!(impact["metrics"][key].is_number(), "missing metrics.{key}: {impact}");
    }
    let cli_impact = cli_json(&repo, &["blast-radius", "OrderService::process"]);
    assert_eq!(impact["schema_version"], cli_impact["schema_version"]);
    assert_eq!(impact["target"]["canonical_fqn"], cli_impact["target"]["canonical_fqn"]);

    let metrics = mcp_structured(&mcp.call(
        "rgctl_metrics",
        serde_json::json!({ "pagerank": true }),
    ));
    assert_not_status(&metrics, "rgctl_metrics");
    assert_eq!(metrics["schema_version"], 1);
    assert!(metrics["pagerank"]["top"].is_array(), "{metrics}");
    assert!(metrics.get("betweenness").is_none(), "{metrics}");
    let cli_metrics = cli_json(&repo, &["metrics", "--pagerank"]);
    assert_eq!(metrics["schema_version"], cli_metrics["schema_version"]);
    assert!(cli_metrics["pagerank"].is_object());

    let cpg_status = mcp_structured(&mcp.call(
        "rgctl_cpg",
        serde_json::json!({ "op": "status" }),
    ));
    assert_not_status(&cpg_status, "rgctl_cpg status");
    assert_eq!(cpg_status["schema_version"], 1);
    assert!(cpg_status["archive_present"].is_boolean(), "{cpg_status}");
    let cli_cpg = cli_json(&repo, &["cpg", "status"]);
    assert_eq!(cpg_status["schema_version"], cli_cpg["schema_version"]);
    assert_eq!(cpg_status["archive_present"], cli_cpg["archive_present"]);

    let cpg_fn = mcp_structured(&mcp.call(
        "rgctl_cpg",
        serde_json::json!({ "op": "function", "symbol": "unique_leaf" }),
    ));
    assert_not_status(&cpg_fn, "rgctl_cpg function");
    assert_eq!(cpg_fn["schema_version"], 1);
    assert_eq!(cpg_fn["name"], "unique_leaf");

    let cpg_calls = mcp_structured(&mcp.call(
        "rgctl_cpg",
        serde_json::json!({ "op": "calls", "symbol": "unique_leaf" }),
    ));
    assert_not_status(&cpg_calls, "rgctl_cpg calls");
    assert_eq!(cpg_calls["schema_version"], 1);
    assert!(cpg_calls["edges"].is_array(), "{cpg_calls}");

    let policy = repo.join("policy.json");
    let check = mcp_structured(&mcp.call(
        "rgctl_check",
        serde_json::json!({ "policy_file": policy.to_str().unwrap() }),
    ));
    assert_not_status(&check, "rgctl_check");
    assert_eq!(check["schema_version"], 1);
    assert!(check["passed"].is_boolean(), "{check}");
    assert!(check["violations"].is_array(), "{check}");
    let cli_check = cli_json(
        &repo,
        &["check", "--policy-file", policy.to_str().unwrap()],
    );
    assert_eq!(check["schema_version"], cli_check["schema_version"]);
    assert_eq!(check["passed"], cli_check["passed"]);
}

#[test]
fn mcp_tool_call_ids_and_invalid_metrics_flag() {
    let (_tmp, repo) = materialize_fixture();
    discover_and_index(&repo);
    let mut mcp = mcp_connect_stdio(&repo);

    let listed = mcp.rpc("tools/list", serde_json::json!({}));
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(names.len(), 7, "{listed}");

    let bad_metrics = mcp.call("rgctl_metrics", serde_json::json!({}));
    assert_eq!(bad_metrics["error"]["code"], -32602, "{bad_metrics}");

    let explain = mcp_structured(&mcp.call(
        "rgctl_query",
        serde_json::json!({
            "query": "MATCH (n:Function) RETURN n LIMIT 1",
            "explain": true
        }),
    ));
    assert_eq!(explain["explain"], true);
    assert!(explain["count"].as_u64().unwrap_or(0) <= 1);
}

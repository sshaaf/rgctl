//! MCP `tools/call` coverage on an indexed repo (happy path for each catalog tool).

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn rgbuilder_bin() -> PathBuf {
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(bin);
    }
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(bin);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rgctl")
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(from, to)?;
        } else {
            fs::copy(from, to)?;
        }
    }
    Ok(())
}

fn materialize() -> (tempfile::TempDir, PathBuf) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    copy_dir_all(&fixture, &repo).expect("copy fixture");
    let _ = fs::remove_dir_all(repo.join(".rgbuilder"));
    let _ = fs::remove_dir_all(repo.join(".rbuilder"));
    (tmp, repo)
}

fn run_in(repo: &Path, args: &[&str]) -> Output {
    Command::new(rgbuilder_bin())
        .args(args)
        .current_dir(repo)
        .output()
        .expect("spawn rgctl")
}

fn assert_ok(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed status={:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn cli_json(repo: &Path, args: &[&str]) -> Value {
    let mut full = vec!["-r", repo.to_str().unwrap(), "-f", "json"];
    full.extend_from_slice(args);
    let output = run_in(repo, &full);
    assert_ok(&output, &format!("cli {}", args.join(" ")));
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "CLI JSON parse failed ({err}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn discover_and_index(repo: &Path) {
    let repo_s = repo.to_str().unwrap();
    assert_ok(
        &run_in(
            repo,
            &[
                "-r",
                repo_s,
                "discover",
                ".",
                "--languages",
                "java,rust",
            ],
        ),
        "discover",
    );
    assert_ok(
        &run_in(repo, &["-r", repo_s, "semantic", "index"]),
        "semantic index",
    );
    let policy = Path::new(env!("CARGO_MANIFEST_DIR")).join("rgbuilder-tests/rgbuilder-policy.json");
    fs::copy(&policy, repo.join("policy.json")).expect("copy policy");
}

fn read_mcp_json(reader: &mut BufReader<impl Read>) -> Option<Value> {
    let mut header = String::new();
    let mut content_length: Option<usize> = None;
    loop {
        header.clear();
        if reader.read_line(&mut header).ok()? == 0 {
            return None;
        }
        let lower = header.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse().ok();
        }
        if header.trim().is_empty() {
            break;
        }
        if header.trim_start().starts_with('{') {
            return serde_json::from_str(header.trim()).ok();
        }
    }
    let len = content_length?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

struct McpProc {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i64,
}

impl Drop for McpProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl McpProc {
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        writeln!(self.stdin, "{msg}").expect("write rpc");
        let resp = read_mcp_json(&mut self.reader).expect("rpc response");
        assert_eq!(resp["id"], id, "{resp}");
        resp
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.rpc(
            "tools/call",
            serde_json::json!({ "name": name, "arguments": arguments }),
        )
    }
}

fn mcp_connect(repo: &Path) -> McpProc {
    let mut child = Command::new(rgbuilder_bin())
        .args([
            "-r",
            repo.to_str().unwrap(),
            "serve",
            "--mode",
            "mcp",
            "--no-pipeline",
        ])
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut proc = McpProc {
        child,
        stdin,
        reader: BufReader::new(stdout),
        next_id: 1,
    };
    let init = proc.rpc(
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" }
        }),
    );
    assert!(init.get("result").is_some(), "initialize failed: {init}");
    proc
}

fn mcp_structured(resp: &Value) -> Value {
    assert!(resp.get("error").is_none(), "tool error: {resp}");
    let result = resp.get("result").expect("result");
    let structured = result
        .get("structuredContent")
        .cloned()
        .unwrap_or_else(|| {
            let text = result["content"][0]["text"].as_str().unwrap_or("");
            serde_json::from_str(text).unwrap_or_else(|err| {
                panic!("expected structured JSON ({err}): {resp}")
            })
        });
    let text = result["content"][0]["text"].as_str().expect("content text");
    let parsed: Value = serde_json::from_str(text).expect("content text JSON");
    assert_eq!(parsed, structured, "content text must match structuredContent");
    structured
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
    let (_tmp, repo) = materialize();
    discover_and_index(&repo);
    let mut mcp = mcp_connect(&repo);

    let status = mcp_structured(&mcp.call("rgbuilder_status", serde_json::json!({})));
    assert_eq!(status["schema_version"], 1);
    assert_eq!(status["command"], "pipeline_status");

    let match_q = "MATCH (n:Function) RETURN n LIMIT 5";
    let query = mcp_structured(&mcp.call(
        "rgbuilder_query",
        serde_json::json!({ "query": match_q }),
    ));
    assert_not_status(&query, "rgbuilder_query MATCH");
    assert_eq!(query["schema_version"], 1);
    assert!(query["rows"].is_array(), "{query}");
    assert!(query["count"].as_u64().unwrap_or(0) <= 5, "{query}");
    let cli_query = cli_json(&repo, &["gql", match_q]);
    assert_eq!(query["schema_version"], cli_query["schema_version"]);
    assert_eq!(query["count"], cli_query["count"]);

    let macro_q = mcp_structured(&mcp.call(
        "rgbuilder_query",
        serde_json::json!({ "macro": "all_functions", "limit": 5 }),
    ));
    assert_not_status(&macro_q, "rgbuilder_query macro");
    assert!(macro_q["count"].as_u64().unwrap_or(99) <= 5, "{macro_q}");
    assert_eq!(
        macro_q["rows"].as_array().map(Vec::len).unwrap_or(0) as u64,
        macro_q["count"].as_u64().unwrap_or(0)
    );

    let search = mcp_structured(&mcp.call(
        "rgbuilder_search",
        serde_json::json!({ "text": "order", "scope": "function", "limit": 5 }),
    ));
    assert_not_status(&search, "rgbuilder_search");
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
        "rgbuilder_impact",
        serde_json::json!({ "symbol": "OrderService::process" }),
    ));
    assert_not_status(&impact, "rgbuilder_impact");
    assert_eq!(impact["schema_version"], 2);
    for key in ["score", "direct_callers_count", "impact_zone_size"] {
        assert!(impact["metrics"][key].is_number(), "missing metrics.{key}: {impact}");
    }
    let cli_impact = cli_json(&repo, &["blast-radius", "OrderService::process"]);
    assert_eq!(impact["schema_version"], cli_impact["schema_version"]);
    assert_eq!(impact["target"]["canonical_fqn"], cli_impact["target"]["canonical_fqn"]);

    let metrics = mcp_structured(&mcp.call(
        "rgbuilder_metrics",
        serde_json::json!({ "pagerank": true }),
    ));
    assert_not_status(&metrics, "rgbuilder_metrics");
    assert_eq!(metrics["schema_version"], 1);
    assert!(metrics["pagerank"]["top"].is_array(), "{metrics}");
    assert!(metrics.get("betweenness").is_none(), "{metrics}");
    let cli_metrics = cli_json(&repo, &["metrics", "--pagerank"]);
    assert_eq!(metrics["schema_version"], cli_metrics["schema_version"]);
    assert!(cli_metrics["pagerank"].is_object());

    let cpg_status = mcp_structured(&mcp.call(
        "rgbuilder_cpg",
        serde_json::json!({ "op": "status" }),
    ));
    assert_not_status(&cpg_status, "rgbuilder_cpg status");
    assert_eq!(cpg_status["schema_version"], 1);
    assert!(cpg_status["archive_present"].is_boolean(), "{cpg_status}");
    let cli_cpg = cli_json(&repo, &["cpg", "status"]);
    assert_eq!(cpg_status["schema_version"], cli_cpg["schema_version"]);
    assert_eq!(cpg_status["archive_present"], cli_cpg["archive_present"]);

    let cpg_fn = mcp_structured(&mcp.call(
        "rgbuilder_cpg",
        serde_json::json!({ "op": "function", "symbol": "unique_leaf" }),
    ));
    assert_not_status(&cpg_fn, "rgbuilder_cpg function");
    assert_eq!(cpg_fn["schema_version"], 1);
    assert_eq!(cpg_fn["name"], "unique_leaf");

    let cpg_calls = mcp_structured(&mcp.call(
        "rgbuilder_cpg",
        serde_json::json!({ "op": "calls", "symbol": "unique_leaf" }),
    ));
    assert_not_status(&cpg_calls, "rgbuilder_cpg calls");
    assert_eq!(cpg_calls["schema_version"], 1);
    assert!(cpg_calls["edges"].is_array(), "{cpg_calls}");

    let policy = repo.join("policy.json");
    let check = mcp_structured(&mcp.call(
        "rgbuilder_check",
        serde_json::json!({ "policy_file": policy.to_str().unwrap() }),
    ));
    assert_not_status(&check, "rgbuilder_check");
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
    let (_tmp, repo) = materialize();
    discover_and_index(&repo);
    let mut mcp = mcp_connect(&repo);

    let listed = mcp.rpc("tools/list", serde_json::json!({}));
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert_eq!(names.len(), 7, "{listed}");

    let bad_metrics = mcp.call("rgbuilder_metrics", serde_json::json!({}));
    assert_eq!(bad_metrics["error"]["code"], -32602, "{bad_metrics}");

    let explain = mcp_structured(&mcp.call(
        "rgbuilder_query",
        serde_json::json!({
            "query": "MATCH (n:Function) RETURN n LIMIT 1",
            "explain": true
        }),
    ));
    assert_eq!(explain["explain"], true);
    assert!(explain["count"].as_u64().unwrap_or(0) <= 1);
}

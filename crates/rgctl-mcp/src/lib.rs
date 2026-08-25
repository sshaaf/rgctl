//! MCP stdio server (`rgctl serve --mode mcp`).
//!
//! JSON-RPC only on stdout. Depends on `rgctl-service`, not the root `rgctl` package.
#![allow(missing_docs)]

use anyhow::{Context, Result};
use rgctl_service::command::{
    CheckArgs, Command, CpgArgs, CpgOp, DEFAULT_LIMIT, ImpactArgs, MetricsArgs, QueryArgs,
    SearchArgs, SearchScope,
};
use rgctl_service::error::ServiceError;
use rgctl_service::{Session, execute, pipeline_status_value};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;

/// Optional hook invoked once before the stdio loop (CLI starts the pipeline).
pub type OnStart = Box<dyn FnOnce() + Send>;

/// MCP serve options.
pub struct McpServeArgs {
    /// Session root.
    pub repo: PathBuf,
    /// Called once at start (e.g. spawn full pipeline). Must not write to stdout.
    pub on_start: Option<OnStart>,
}

/// Run MCP on stdio. Does not bind HTTP. Does not run discover.
pub fn serve(args: McpServeArgs) -> Result<()> {
    if let Some(hook) = args.on_start {
        hook();
    }
    let mut session = Session::new(args.repo);
    run_stdio_loop(&mut session)
}

fn run_stdio_loop(session: &mut Session) -> Result<()> {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout().lock();
    loop {
        let Some(msg) = read_rpc_message(&mut reader)? else {
            break;
        };
        if let Some(response) = handle_message(session, &msg) {
            write_rpc_message(&mut stdout, &response)?;
        }
    }
    Ok(())
}

fn read_rpc_message(reader: &mut BufReader<std::io::StdinLock<'_>>) -> Result<Option<Value>> {
    let mut first = String::new();
    let n = reader.read_line(&mut first)?;
    if n == 0 {
        return Ok(None);
    }
    let trimmed = first.trim_start();
    if trimmed.starts_with('{') {
        let parsed = serde_json::from_str(trimmed.trim()).context("parse JSON-RPC line")?;
        return Ok(Some(parsed));
    }
    let mut content_length: Option<usize> = None;
    let mut line = first;
    loop {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse().ok();
        }
        if line.trim().is_empty() {
            break;
        }
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
    }
    let len = content_length.context("MCP message missing Content-Length")?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).context("read MCP body")?;
    let parsed = serde_json::from_slice(&buf).context("parse MCP JSON-RPC")?;
    Ok(Some(parsed))
}

fn write_rpc_message(stdout: &mut std::io::StdoutLock<'_>, value: &Value) -> Result<()> {
    let body = serde_json::to_vec(value)?;
    write!(stdout, "Content-Length: {}\r\n\r\n", body.len())?;
    stdout.write_all(&body)?;
    stdout.flush()?;
    Ok(())
}

const TOOL_NAMES: &[&str] = &[
    "rgctl_status",
    "rgctl_query",
    "rgctl_search",
    "rgctl_impact",
    "rgctl_metrics",
    "rgctl_cpg",
    "rgctl_check",
];

/// Handle a single JSON-RPC message (stdio or HTTP).
pub fn handle_rpc(session: &mut Session, msg: &Value) -> Option<Value> {
    handle_message(session, msg)
}

/// Stdio MCP loop that forwards each message to `handler`.
pub fn serve_proxy<F>(mut handler: F) -> Result<()>
where
    F: FnMut(&Value) -> Option<Value>,
{
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout().lock();
    loop {
        let Some(msg) = read_rpc_message(&mut reader)? else {
            break;
        };
        if let Some(response) = handler(&msg) {
            write_rpc_message(&mut stdout, &response)?;
        }
    }
    Ok(())
}

const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

fn negotiate_protocol_version(client_version: Option<&str>) -> &'static str {
    match client_version {
        Some("2025-03-26") => "2025-03-26",
        Some("2024-11-05") => "2024-11-05",
        Some("2024-10-07") => "2024-10-07",
        _ => DEFAULT_PROTOCOL_VERSION,
    }
}

fn handle_message(session: &mut Session, msg: &Value) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id").cloned();
    match method {
        "initialize" => {
            let client_version = msg
                .pointer("/params/protocolVersion")
                .and_then(|v| v.as_str());
            let protocol_version = negotiate_protocol_version(client_version);
            Some(rpc_ok(
                id,
                json!({
                    "protocolVersion": protocol_version,
                    "capabilities": { "tools": {}, "resources": {} },
                    "serverInfo": { "name": "rgctl", "version": env!("CARGO_PKG_VERSION") }
                }),
            ))
        }
        "notifications/initialized" | "initialized" => None,
        "ping" => Some(rpc_ok(id, json!({}))),
        "tools/list" => Some(rpc_ok(id, json!({ "tools": tool_descriptors() }))),
        "tools/call" => {
            let name = msg
                .pointer("/params/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = msg
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match dispatch_tool(session, name, &arguments) {
                Ok(value) => Some(rpc_ok(id, tool_result(value))),
                Err(err) => Some(map_service_error(id, err)),
            }
        }
        "resources/list" => Some(rpc_ok(id, json!({ "resources": resource_list() }))),
        "resources/read" => {
            let uri = msg
                .pointer("/params/uri")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match read_resource(session, uri) {
                Ok(body) => Some(rpc_ok(
                    id,
                    json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": body
                        }]
                    }),
                )),
                Err(message) => Some(rpc_err(id, -32602, message)),
            }
        }
        _ => {
            if id.is_some() {
                Some(rpc_err(id, -32601, format!("method not found: {method}")))
            } else {
                None
            }
        }
    }
}

fn tool_descriptors() -> Vec<Value> {
    vec![
        json!({
            "name": "rgctl_status",
            "description": "Pipeline and artifact status (dashboard, CFG, semantic).",
            "inputSchema": { "type": "object", "properties": { "repo": { "type": "string" } } }
        }),
        json!({
            "name": "rgctl_query",
            "description": "GQL MATCH or named macro (all_functions, all_communities, direct_calls, call_chain). Default limit 20. Find-by-name/FQN/community/neighborhood are GQL, not extra tools.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "macro": { "type": "string" },
                    "explain": { "type": "boolean" },
                    "limit": { "type": "integer" },
                    "repo": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "rgctl_search",
            "description": "Natural-language semantic search. Default limit 20. Scope: function, community, docs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string" },
                    "scope": { "type": "string" },
                    "limit": { "type": "integer" },
                    "repo": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "rgctl_impact",
            "description": "Blast-radius (upstream impact) before editing a symbol.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "symbol": { "type": "string" },
                    "depth": { "type": "integer" },
                    "class": { "type": "string" },
                    "file": { "type": "string" },
                    "repo": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "rgctl_metrics",
            "description": "PageRank, betweenness, or community hotspots. At least one flag required.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pagerank": { "type": "boolean" },
                    "betweenness": { "type": "boolean" },
                    "communities": { "type": "boolean" },
                    "repo": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "rgctl_cpg",
            "description": "Hybrid CPG. op: status, function, calls, mutations, flows, slice, inspect, pdg, ast. Not export.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "op": { "type": "string" },
                    "symbol": { "type": "string" },
                    "type_name": { "type": "string" },
                    "file": { "type": "string" },
                    "line": { "type": "integer" },
                    "variable": { "type": "string" },
                    "function": { "type": "string" },
                    "exclude_ctors": { "type": "boolean" },
                    "member": { "type": "string" },
                    "include_unresolved": { "type": "boolean" },
                    "direction": { "type": "string" },
                    "with_alias": { "type": "boolean" },
                    "repo": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "rgctl_check",
            "description": "CI policy check against a policy JSON file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "policy_file": { "type": "string" },
                    "repo": { "type": "string" }
                }
            }
        }),
    ]
}

fn dispatch_tool(session: &mut Session, name: &str, args: &Value) -> Result<Value, ServiceError> {
    if !TOOL_NAMES.contains(&name) {
        return Err(ServiceError::InvalidParams(format!("unknown tool {name}")));
    }
    let command = match name {
        "rgctl_status" => Command::Status,
        "rgctl_query" => Command::Query(parse_query(args)?),
        "rgctl_search" => Command::Search(parse_search(args)?),
        "rgctl_impact" => Command::Impact(parse_impact(args)?),
        "rgctl_metrics" => Command::Metrics(parse_metrics(args)?),
        "rgctl_cpg" => Command::Cpg(parse_cpg(args)?),
        "rgctl_check" => Command::Check(parse_check(args)?),
        _ => unreachable!(),
    };
    execute(session, command)
}

fn parse_query(args: &Value) -> Result<QueryArgs, ServiceError> {
    Ok(QueryArgs {
        query: args.get("query").and_then(|v| v.as_str()).map(str::to_string),
        macro_name: args
            .get("macro")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        explain: args.get("explain").and_then(Value::as_bool).unwrap_or(false),
        limit: Some(
            args.get("limit")
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_LIMIT),
        ),
    })
}

fn parse_search(args: &Value) -> Result<SearchArgs, ServiceError> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(SearchArgs {
        text,
        scope: SearchScope::parse(args.get("scope").and_then(|v| v.as_str()))?,
        limit: Some(
            args.get("limit")
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_LIMIT),
        ),
    })
}

fn parse_impact(args: &Value) -> Result<ImpactArgs, ServiceError> {
    Ok(ImpactArgs {
        symbol: args
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        depth: args.get("depth").and_then(Value::as_u64).map(|n| n as usize),
        class: args.get("class").and_then(|v| v.as_str()).map(str::to_string),
        file: args.get("file").and_then(|v| v.as_str()).map(str::to_string),
    })
}

fn parse_metrics(args: &Value) -> Result<MetricsArgs, ServiceError> {
    Ok(MetricsArgs {
        pagerank: args.get("pagerank").and_then(Value::as_bool).unwrap_or(false),
        betweenness: args
            .get("betweenness")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        communities: args
            .get("communities")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn parse_cpg(args: &Value) -> Result<CpgArgs, ServiceError> {
    let op = CpgOp::parse(args.get("op").and_then(|v| v.as_str()).unwrap_or(""))?;
    Ok(CpgArgs {
        op,
        symbol: opt_str(args, "symbol"),
        type_name: opt_str(args, "type_name"),
        file: opt_str(args, "file"),
        line: args.get("line").and_then(Value::as_u64).map(|n| n as usize),
        variable: opt_str(args, "variable"),
        function: opt_str(args, "function"),
        exclude_ctors: args
            .get("exclude_ctors")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        member: opt_str(args, "member"),
        include_unresolved: args
            .get("include_unresolved")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        direction: opt_str(args, "direction"),
        with_alias: args
            .get("with_alias")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn parse_check(args: &Value) -> Result<CheckArgs, ServiceError> {
    let policy_file = args
        .get("policy_file")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if policy_file.is_empty() {
        return Err(ServiceError::InvalidParams("`policy_file` is required".into()));
    }
    Ok(CheckArgs { policy_file })
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn tool_result(value: Value) -> Value {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into());
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value
    })
}

fn map_service_error(id: Option<Value>, err: ServiceError) -> Value {
    match err {
        ServiceError::UnknownOp { op, allowed } => {
            rpc_err(id, -32602, format!("unknown op '{op}'. allowed: {allowed}"))
        }
        ServiceError::InvalidParams(msg) => rpc_err(id, -32602, msg),
        ServiceError::Failed(msg) => rpc_err(id, -32603, msg),
    }
}

fn resource_list() -> Vec<Value> {
    vec![
        json!({
            "uri": "rgctl://status",
            "name": "Pipeline status",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "rgctl://manifest",
            "name": "Dashboard manifest",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "rgctl://migration-plan",
            "name": "Migration plan",
            "mimeType": "application/json"
        }),
    ]
}

fn read_resource(session: &Session, uri: &str) -> Result<String, String> {
    match uri {
        "rgctl://status" => {
            let value = pipeline_status_value(session);
            serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
        }
        "rgctl://manifest" => {
            let manifest = session.repo().join(".rgctl/dashboard/manifest.json");
            if manifest.is_file() {
                std::fs::read_to_string(&manifest).map_err(|e| e.to_string())
            } else {
                serde_json::to_string_pretty(&pipeline_status_value(session)).map_err(|e| e.to_string())
            }
        }
        "rgctl://migration-plan" => {
            let candidates = [
                session.repo().join(".rgctl/migration_plan.json"),
                session
                    .repo()
                    .join(".rgctl/dashboard/migration_plan.json"),
            ];
            for path in candidates {
                if path.is_file() {
                    return std::fs::read_to_string(&path).map_err(|e| e.to_string());
                }
            }
            Ok(json!({
                "available": false,
                "message": "migration plan not available"
            })
            .to_string())
        }
        other => Err(format!("unknown resource URI: {other}")),
    }
}

fn rpc_ok(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result })
}

fn rpc_err(id: Option<Value>, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": { "code": code, "message": message }
    })
}

/// Tool names advertised in `tools/list` (for tests).
#[must_use]
pub fn advertised_tool_names() -> &'static [&'static str] {
    TOOL_NAMES
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Harness {
        _tmp: tempfile::TempDir,
        session: Session,
    }

    fn empty_repo() -> Harness {
        let tmp = tempfile::tempdir().expect("tempdir");
        let session = Session::new(tmp.path());
        Harness { _tmp: tmp, session }
    }

    fn rpc(session: &mut Session, id: i64, method: &str, params: Value) -> Value {
        handle_message(
            session,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }),
        )
        .unwrap_or_else(|| panic!("expected response for {method}"))
    }

    fn call_tool(session: &mut Session, id: i64, name: &str, arguments: Value) -> Value {
        rpc(
            session,
            id,
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }

    fn structured(resp: &Value) -> Value {
        if let Some(value) = resp.pointer("/result/structuredContent") {
            return value.clone();
        }
        panic!("missing structuredContent: {resp}");
    }

    fn assert_tool_envelope(resp: &Value, id: i64) {
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], id);
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        let result = resp.get("result").expect("result");
        assert_eq!(result["content"][0]["type"], "text");
        let text = result["content"][0]["text"].as_str().expect("text");
        let parsed: Value = serde_json::from_str(text).expect("content text is JSON");
        assert_eq!(
            &parsed,
            result.get("structuredContent").expect("structuredContent"),
            "content text must match structuredContent"
        );
    }

    fn assert_invalid_params(resp: &Value, id: i64, needle: &str) {
        assert_eq!(resp["id"], id);
        assert_eq!(resp["error"]["code"], -32602, "{resp}");
        let message = resp["error"]["message"].as_str().unwrap_or("");
        assert!(
            message.to_ascii_lowercase().contains(&needle.to_ascii_lowercase()),
            "expected '{needle}' in '{message}' ({resp})"
        );
    }

    #[test]
    fn initialize_echoes_supported_client_protocol_version() {
        let mut h = empty_repo();
        let resp = rpc(
            &mut h.session,
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "opencode", "version": "0" }
            }),
        );
        assert_eq!(resp["result"]["protocolVersion"], "2025-03-26");
    }

    #[test]
    fn initialize_advertises_tools_and_resources() {
        let mut h = empty_repo();
        let resp = rpc(
            &mut h.session,
            1,
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }),
        );
        assert_eq!(resp["result"]["serverInfo"]["name"], "rgctl");
        assert!(resp["result"]["capabilities"]["tools"].is_object());
        assert!(resp["result"]["capabilities"]["resources"].is_object());
    }

    #[test]
    fn tools_list_matches_catalog_and_schemas() {
        let mut h = empty_repo();
        let resp = rpc(&mut h.session, 1, "tools/list", json!({}));
        let tools = resp["result"]["tools"].as_array().expect("tools");
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert_eq!(names, advertised_tool_names());
        for tool in tools {
            assert!(tool["inputSchema"].is_object(), "{tool}");
            assert!(tool["description"].as_str().is_some_and(|d| !d.is_empty()));
        }
    }

    #[test]
    fn ping_and_unknown_method() {
        let mut h = empty_repo();
        let ping = rpc(&mut h.session, 1, "ping", json!({}));
        assert!(ping.get("result").is_some(), "{ping}");
        let unknown = rpc(&mut h.session, 2, "tools/invoke", json!({}));
        assert_eq!(unknown["error"]["code"], -32601);
    }

    #[test]
    fn initialized_notification_has_no_response() {
        let mut h = empty_repo();
        let none = handle_message(
            &mut h.session,
            &json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }),
        );
        assert!(none.is_none());
    }

    #[test]
    fn status_tool_returns_pipeline_status_envelope() {
        let mut h = empty_repo();
        let resp = call_tool(&mut h.session, 1, "rgctl_status", json!({}));
        assert_tool_envelope(&resp, 1);
        let doc = structured(&resp);
        assert_eq!(doc["schema_version"], 1);
        assert_eq!(doc["command"], "pipeline_status");
        assert_eq!(doc["semantic_ready"], false);
        assert_eq!(doc["cfg_ready"], false);
    }

    #[test]
    fn query_rejects_both_and_neither() {
        let mut h = empty_repo();
        let both = call_tool(
            &mut h.session,
            1,
            "rgctl_query",
            json!({
                "query": "MATCH (n:Function) RETURN n",
                "macro": "all_functions"
            }),
        );
        assert_invalid_params(&both, 1, "not both");

        let neither = call_tool(&mut h.session, 2, "rgctl_query", json!({}));
        assert_invalid_params(&neither, 2, "query");
    }

    #[test]
    fn query_without_graph_is_status_not_rpc_error() {
        let mut h = empty_repo();
        let resp = call_tool(
            &mut h.session,
            1,
            "rgctl_query",
            json!({ "query": "MATCH (n:Function) RETURN n" }),
        );
        assert_tool_envelope(&resp, 1);
        assert_eq!(structured(&resp)["command"], "pipeline_status");
    }

    #[test]
    fn parse_query_defaults_limit_to_twenty() {
        let args = parse_query(&json!({ "macro": "all_functions" })).expect("parse");
        assert_eq!(args.limit, Some(DEFAULT_LIMIT));
        let capped = parse_query(&json!({ "query": "MATCH (n) RETURN n", "limit": 5 })).expect("parse");
        assert_eq!(capped.limit, Some(5));
        assert!(!capped.explain);
    }

    #[test]
    fn search_empty_text_is_invalid_params() {
        let mut h = empty_repo();
        let resp = call_tool(&mut h.session, 1, "rgctl_search", json!({ "text": "  " }));
        assert_invalid_params(&resp, 1, "text");
    }

    #[test]
    fn search_unknown_scope_is_invalid_params() {
        let mut h = empty_repo();
        let resp = call_tool(
            &mut h.session,
            1,
            "rgctl_search",
            json!({ "text": "checkout", "scope": "packages" }),
        );
        assert_invalid_params(&resp, 1, "scope");
    }

    #[test]
    fn search_without_index_returns_status() {
        let mut h = empty_repo();
        let resp = call_tool(
            &mut h.session,
            1,
            "rgctl_search",
            json!({ "text": "checkout", "scope": "function" }),
        );
        assert_tool_envelope(&resp, 1);
        let doc = structured(&resp);
        assert_eq!(doc["command"], "pipeline_status");
        assert_eq!(doc["semantic_ready"], false);
    }

    #[test]
    fn parse_search_scopes() {
        let parsed = parse_search(&json!({ "text": "x", "scope": "docs" })).expect("docs");
        assert_eq!(parsed.scope, SearchScope::Docs);
        assert_eq!(parsed.limit, Some(DEFAULT_LIMIT));
        assert!(parse_search(&json!({ "text": "x", "scope": "nope" })).is_err());
    }

    #[test]
    fn impact_requires_symbol_even_without_graph() {
        let mut h = empty_repo();
        let missing = call_tool(&mut h.session, 1, "rgctl_impact", json!({}));
        assert_invalid_params(&missing, 1, "symbol");
        let unreadiness = call_tool(
            &mut h.session,
            2,
            "rgctl_impact",
            json!({ "symbol": "OrderService::process" }),
        );
        assert_tool_envelope(&unreadiness, 2);
        assert_eq!(structured(&unreadiness)["command"], "pipeline_status");
    }

    #[test]
    fn metrics_requires_a_flag_then_unreadiness() {
        let mut h = empty_repo();
        let none = call_tool(&mut h.session, 1, "rgctl_metrics", json!({}));
        assert_invalid_params(&none, 1, "pagerank");
        let unreadiness = call_tool(
            &mut h.session,
            2,
            "rgctl_metrics",
            json!({ "pagerank": true }),
        );
        assert_tool_envelope(&unreadiness, 2);
        assert_eq!(structured(&unreadiness)["command"], "pipeline_status");
    }

    #[test]
    fn cpg_status_works_without_graph() {
        let mut h = empty_repo();
        let resp = call_tool(
            &mut h.session,
            1,
            "rgctl_cpg",
            json!({ "op": "status" }),
        );
        assert_tool_envelope(&resp, 1);
        let doc = structured(&resp);
        assert_eq!(doc["schema_version"], 1);
        assert_eq!(doc["archive_present"], false);
        assert_ne!(doc["command"], "pipeline_status");
    }

    #[test]
    fn cpg_slice_without_cfg_is_status() {
        let mut h = empty_repo();
        let resp = call_tool(&mut h.session, 1, "rgctl_cpg", json!({ "op": "slice" }));
        assert_tool_envelope(&resp, 1);
        let doc = structured(&resp);
        assert_eq!(doc["command"], "pipeline_status");
        assert_eq!(doc["cfg_ready"], false);
    }

    #[test]
    fn cpg_export_is_unknown_op() {
        let mut h = empty_repo();
        let resp = call_tool(&mut h.session, 1, "rgctl_cpg", json!({ "op": "export" }));
        assert_invalid_params(&resp, 1, "unknown op");
        let message = resp["error"]["message"].as_str().unwrap_or("");
        assert!(message.contains("unknown op"), "{message}");
        let allowed = message.split("allowed:").nth(1).unwrap_or("");
        assert!(allowed.contains("slice"), "{message}");
        assert!(
            !allowed.contains("export"),
            "allowed ops must not include export: {message}"
        );
    }

    #[test]
    fn check_requires_policy_file() {
        let mut h = empty_repo();
        let resp = call_tool(&mut h.session, 1, "rgctl_check", json!({}));
        assert_invalid_params(&resp, 1, "policy_file");
    }

    #[test]
    fn unknown_tool_is_invalid_params() {
        let mut h = empty_repo();
        let resp = call_tool(&mut h.session, 1, "rgctl_discover", json!({}));
        assert_invalid_params(&resp, 1, "unknown tool");
        let export = call_tool(&mut h.session, 2, "rgctl_export", json!({}));
        assert_invalid_params(&export, 2, "unknown tool");
    }

    #[test]
    fn resources_list_and_read() {
        let mut h = empty_repo();
        let listed = rpc(&mut h.session, 1, "resources/list", json!({}));
        let uris: Vec<&str> = listed["result"]["resources"]
            .as_array()
            .expect("resources")
            .iter()
            .filter_map(|r| r["uri"].as_str())
            .collect();
        assert_eq!(
            uris,
            [
                "rgctl://status",
                "rgctl://manifest",
                "rgctl://migration-plan"
            ]
        );

        let status = rpc(
            &mut h.session,
            2,
            "resources/read",
            json!({ "uri": "rgctl://status" }),
        );
        let text = status["result"]["contents"][0]["text"]
            .as_str()
            .expect("text");
        let doc: Value = serde_json::from_str(text).expect("status json");
        assert_eq!(doc["command"], "pipeline_status");
        assert_eq!(doc["schema_version"], 1);

        let plan = rpc(
            &mut h.session,
            3,
            "resources/read",
            json!({ "uri": "rgctl://migration-plan" }),
        );
        let plan_text = plan["result"]["contents"][0]["text"]
            .as_str()
            .expect("plan text");
        let plan_doc: Value = serde_json::from_str(plan_text).expect("plan json");
        assert_eq!(plan_doc["available"], false);

        let unknown = rpc(
            &mut h.session,
            4,
            "resources/read",
            json!({ "uri": "rgctl://nope" }),
        );
        assert_eq!(unknown["error"]["code"], -32602);
    }
}

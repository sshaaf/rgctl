//! MCP stdio server (`rg-build serve --mode mcp`).
//!
//! JSON-RPC only on stdout. Depends on `rgbuilder-service`, not the root `rgbuilder` package.
#![allow(missing_docs)]

use anyhow::{Context, Result};
use rgbuilder_service::command::{
    CheckArgs, Command, CpgArgs, CpgOp, DEFAULT_LIMIT, ImpactArgs, MetricsArgs, QueryArgs,
    SearchArgs, SearchScope,
};
use rgbuilder_service::error::ServiceError;
use rgbuilder_service::{Session, execute, pipeline_status_value};
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
    "rgbuilder_status",
    "rgbuilder_query",
    "rgbuilder_search",
    "rgbuilder_impact",
    "rgbuilder_metrics",
    "rgbuilder_cpg",
    "rgbuilder_check",
];

fn handle_message(session: &mut Session, msg: &Value) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id").cloned();
    match method {
        "initialize" => Some(rpc_ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {}, "resources": {} },
                "serverInfo": { "name": "rgbuilder", "version": env!("CARGO_PKG_VERSION") }
            }),
        )),
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
            "name": "rgbuilder_status",
            "description": "Pipeline and artifact status (dashboard, CFG, semantic).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rgbuilder_query",
            "description": "GQL MATCH or named macro (all_functions, all_communities, direct_calls, call_chain). Default limit 20. Find-by-name/FQN/community/neighborhood are GQL, not extra tools.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "macro": { "type": "string" },
                    "explain": { "type": "boolean" },
                    "limit": { "type": "integer" }
                }
            }
        }),
        json!({
            "name": "rgbuilder_search",
            "description": "Natural-language semantic search. Default limit 20. Scope: function, community, docs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string" },
                    "scope": { "type": "string" },
                    "limit": { "type": "integer" }
                }
            }
        }),
        json!({
            "name": "rgbuilder_impact",
            "description": "Blast-radius (upstream impact) before editing a symbol.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "symbol": { "type": "string" },
                    "depth": { "type": "integer" },
                    "class": { "type": "string" },
                    "file": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "rgbuilder_metrics",
            "description": "PageRank, betweenness, or community hotspots. At least one flag required.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pagerank": { "type": "boolean" },
                    "betweenness": { "type": "boolean" },
                    "communities": { "type": "boolean" }
                }
            }
        }),
        json!({
            "name": "rgbuilder_cpg",
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
                    "with_alias": { "type": "boolean" }
                }
            }
        }),
        json!({
            "name": "rgbuilder_check",
            "description": "CI policy check against a policy JSON file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "policy_file": { "type": "string" }
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
        "rgbuilder_status" => Command::Status,
        "rgbuilder_query" => Command::Query(parse_query(args)?),
        "rgbuilder_search" => Command::Search(parse_search(args)?),
        "rgbuilder_impact" => Command::Impact(parse_impact(args)?),
        "rgbuilder_metrics" => Command::Metrics(parse_metrics(args)?),
        "rgbuilder_cpg" => Command::Cpg(parse_cpg(args)?),
        "rgbuilder_check" => Command::Check(parse_check(args)?),
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
            "uri": "rgbuilder://status",
            "name": "Pipeline status",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "rgbuilder://manifest",
            "name": "Dashboard manifest",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "rgbuilder://migration-plan",
            "name": "Migration plan",
            "mimeType": "application/json"
        }),
    ]
}

fn read_resource(session: &Session, uri: &str) -> Result<String, String> {
    match uri {
        "rgbuilder://status" => {
            let value = pipeline_status_value(session);
            serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
        }
        "rgbuilder://manifest" => {
            let manifest = session.repo().join(".rgbuilder/dashboard/manifest.json");
            if manifest.is_file() {
                std::fs::read_to_string(&manifest).map_err(|e| e.to_string())
            } else {
                serde_json::to_string_pretty(&pipeline_status_value(session)).map_err(|e| e.to_string())
            }
        }
        "rgbuilder://migration-plan" => {
            let candidates = [
                session.repo().join(".rgbuilder/migration_plan.json"),
                session
                    .repo()
                    .join(".rgbuilder/dashboard/migration_plan.json"),
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

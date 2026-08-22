//! MCP stdio server (`rg-build serve --mode mcp`).
//!
//! JSON-RPC only on stdout. Pipeline progress goes to stderr.
//! Hand-rolled initialize + `rgbuilder_status` (issue #60 tool catalog can move to `rmcp`).

use super::context::CliContext;
use super::discover::resolve_session_root;
use super::pipeline_session::spawn_full_pipeline;
use super::pipeline_status;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;

/// Options for MCP mode.
pub struct McpServeArgs {
    pub path: Option<String>,
    pub no_pipeline: bool,
}

/// Run MCP on stdio. Does not bind HTTP.
pub fn serve(ctx: &CliContext, args: McpServeArgs) -> Result<()> {
    let root = PathBuf::from(resolve_session_root(ctx, args.path.as_deref()));
    if !args.no_pipeline {
        let _pipeline = spawn_full_pipeline(root.clone(), ctx.verbose);
    }
    run_stdio_loop(root)
}

fn run_stdio_loop(repo: PathBuf) -> Result<()> {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout().lock();
    loop {
        let Some(msg) = read_rpc_message(&mut reader)? else {
            break;
        };
        if let Some(response) = handle_message(&repo, &msg) {
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
    // Content-Length framing
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

fn handle_message(repo: &PathBuf, msg: &Value) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id").cloned();
    match method {
        "initialize" => Some(rpc_ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "rgbuilder", "version": env!("CARGO_PKG_VERSION") }
            }),
        )),
        "notifications/initialized" | "initialized" => None,
        "ping" => Some(rpc_ok(id, json!({}))),
        "tools/list" => Some(rpc_ok(
            id,
            json!({
                "tools": [{
                    "name": "rgbuilder_status",
                    "description": "Pipeline and artifact status for this session (dashboard, CFG, semantic).",
                    "inputSchema": { "type": "object", "properties": {} }
                }]
            }),
        )),
        "tools/call" => {
            let name = msg
                .pointer("/params/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name != "rgbuilder_status" {
                return Some(rpc_err(id, -32601, format!("unknown tool {name}")));
            }
            let mut status = pipeline_status::read_status(repo);
            pipeline_status::refresh_ready_flags(&mut status, repo);
            if !status.dashboard_ready {
                status.message = Some("Dashboard is being prepared".into());
            }
            let text = serde_json::to_string_pretty(&status).unwrap_or_else(|_| "{}".into());
            Some(rpc_ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": status
                }),
            ))
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

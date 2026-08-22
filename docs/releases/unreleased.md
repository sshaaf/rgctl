# Unreleased (post v0.4.6)

## Breaking

- **`rg-build serve`** no longer exits 1 when `.rgbuilder/dashboard/index.html` is missing. It binds HTTP, starts the **full pipeline** (`discover --full` stages), and serves a preparing page until the dashboard exists. Restore the old fail-fast with **`serve --no-pipeline`**.

## Added

- `rg-build discover PATH --full` — staged pipeline: basic discover → CFG/dashboard/harmonic → semantic index. Plan printed up front; snapshot is queryable after stage 1. JSON includes `full` and `plan`.
- `rg-build serve --mode standard|mcp` (default `standard`). MCP mode is stdio JSON-RPC with tool `rgbuilder_status` (no HTTP bind). Remaining MCP tools: [issue #60](https://github.com/sshaaf/rgBuilder/issues/60).
- CoolStore MCP walkthrough: [MCP Server](../guides/mcp-server.md).
- `GET /api/status` and `.rgbuilder/pipeline_status.json` (`schema_version` 1).

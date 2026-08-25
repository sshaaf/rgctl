# Unreleased (post v0.4.6)

## Breaking

- **Product rename: rgBuilder → rgctl.** Workspace crates are `rgctl-*`; repo artifacts live under **`.rgctl/`** (migrated from `.rgbuilder/` and `.rbuilder/`). Daemon state defaults to **`~/.rgctl/`** (migrated from `~/.rgbuilder/`). Env vars use **`RGCTL_*`** (legacy `RGBUILDER_*` / `RBUILDER_*` still read for one release). MCP tools are **`rgctl_*`** (not `rgbuilder_*`). Agent skill installs to **`.claude/skills/rgctl/`** and **`.cursor/skills/rgctl/`**.
- **CLI binary is `rgctl`.** The `rg-build` / `rg_ctl` binaries are removed (no alias or shim). Use `rgctl` everywhere (PATH-first: `rgctl <command> <path> [--] [flags]`).
- **`rgctl serve --daemon` / `rgctl daemon start`** run a background **HTTP+MCP** daemon (default bind `0.0.0.0:8080`). The legacy blast-radius `query.sock` auto-connect path is retired.
- Default CLI commands **auto-start** a daemon (stderr: `no daemon found; starting`). Daemon state defaults to **`~/.rgctl/`** (pid, socket, `cache/{reponame}/`). Override with **`--daemon-home`** or **`RGCTL_HOME`**. Opt out with **`--no-daemon`** (in-process, source-tree `{repo}/.rgctl/`) or fail closed with **`--fail-if-no-daemon`**. Pinning `--daemon-home` does not auto-start a different home if that daemon is down.

- **`rgctl serve`** no longer exits 1 when `.rgctl/dashboard/index.html` is missing. It binds HTTP, starts the **full pipeline** (`discover --full` stages), and serves a preparing page until the dashboard exists. Restore the old fail-fast with **`serve --no-pipeline`**.

## Added

- `rgctl discover PATH --full` — staged pipeline: basic discover → CFG/dashboard/harmonic → semantic index. Plan printed up front; snapshot is queryable after stage 1. JSON includes `full` and `plan`.
- `rgctl serve --mode standard|mcp` (default `standard`). MCP mode is stdio JSON-RPC (no HTTP bind) with tools `rgctl_status`, `rgctl_query`, `rgctl_search`, `rgctl_impact`, `rgctl_metrics`, `rgctl_cpg`, `rgctl_check`. Shared command service in `rgctl-service`; MCP crate `rgctl-mcp` does not depend on package `rgctl`.
- CoolStore MCP walkthrough: [MCP Server](../guides/mcp-server.md).
- `GET /api/status` and `.rgctl/pipeline_status.json` (`schema_version` 1).

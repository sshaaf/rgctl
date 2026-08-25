# rgctl integration test matrix

How to verify daemon, no-daemon, MCP stdio, and OpenCode host integration before shipping CLI/MCP changes. Shared harness: `tests/rgctl_harness.rs`.

**Quick PR gate (Tier A):**

```bash
cargo test --test rgctl_no_daemon --test rgctl_daemon --test mcp_tools -- --test-threads=1
```

---

## Tier overview

| Tier | When | Corpus | Confidence |
|------|------|--------|------------|
| **A** | Every PR | `tests/fixtures/tiny_polyglot_repo` (temp copy) | Daemon layout, `--no-daemon`, MCP 7-tool contract |
| **B** | Manual / nightly | `example/linux`, metasfresh, … | Scale + cold perf baselines |
| **C** | Manual / optional CI | OpenCode + fixture or `rgbuilder-tests/ecommerce-java` | Real MCP host spawns `rgctl`; full tool-call matrix |

---

## Tier A — fast CI

### Test targets

| File | Tests | What it proves |
|------|-------|----------------|
| `tests/rgctl_no_daemon.rs` | 5 (+ 1 ignored) | Artifacts under `{repo}/.rgbuilder/`; gql/metrics; `-r` + `discover .` pitfall; absolute-path discover |
| `tests/rgctl_daemon.rs` | 13 | Daemon start/stop, auto-start discover → cache (not source tree), HTTP catalog, HTTP MCP, storage override, stdio bridge, **session roundtrip** |
| `tests/mcp_tools.rs` | 2 | All **7** MCP tools via stdio; JSON matches CLI `-f json` |
| `tests/opencode_mcp_smoke.rs` | 1 (+ 2 ignored) | Script skip when `opencode` missing |

### Commands

```bash
cargo test --test rgctl_no_daemon --test rgctl_daemon --test mcp_tools -- --test-threads=1
cargo test --test opencode_mcp_smoke opencode_smoke_script_skips -- --nocapture
```

Daemon tests must use **`--test-threads=1`** (ports + temp `RGCTL_HOME`).

### Harness conventions

| Pattern | Use |
|---------|-----|
| `cd repo && rgctl --no-daemon discover .` | Correct no-daemon indexing |
| `-r OTHER discover .` from another cwd | **Indexes cwd, not `-r`** — regression test documents this |
| `discover /abs/path/to/repo` | Works from any cwd |
| Temp `RGCTL_HOME` + `DaemonGuard` | Daemon tests; stop + cleanup on drop |
| `daemon_discover_auto_start()` | `RGCTL_HOME` only (no `--daemon-home` on discover) |
| `daemon_discover()` | After daemon already running |

### Tier A corpora

- **tiny_polyglot_repo** — copied to `tempfile`; no `.rgbuilder` in git fixture tree.

---

## Tier B — corpus / perf (ignored)

| File | Test | Corpus | Wall (reference M3 Pro) |
|------|------|--------|---------------------------|
| `tests/rgctl_no_daemon.rs` | `linux_no_daemon_discover_and_gql_smoke` | `example/linux` | ~145 s discover |
| `tests/cold_profile_gates.rs` | `linux_cold_discover_within_baseline` | `example/linux` | baseline 145 s |
| `tests/cold_profile_gates.rs` | `metasfresh_cold_discover_within_baseline` | `example/metasfresh-4.9.8b` | baseline 74 s (`--full`) |

```bash
cargo build --release --bin rgctl
cargo test --release --test rgctl_no_daemon linux_no_daemon -- --ignored --nocapture
cargo test --release --test cold_profile_gates -- --ignored --nocapture --test-threads=1
```

**Discover cwd:** cold gates use `.current_dir(repo)` + `discover .` — not `-r repo discover .` from project root.

Override paths: `RGBUILDER_LINUX_REPO`, `METASFRESH_REPO`.

---

## Tier C — OpenCode + MCP tool matrix

### C1 — OpenCode connect smoke (tiny fixture)

Script: `scripts/integration/opencode-mcp-smoke.sh`

| Mode | Env | Transport |
|------|-----|-----------|
| stdio (default) | — | Local `rgctl serve --mode mcp --no-pipeline` |
| daemon | `RGBUILDER_OPENCODE_MODE=daemon` | Remote `http://127.0.0.1:PORT/mcp` |

```bash
chmod +x scripts/integration/opencode-mcp-smoke.sh
cargo build --bin rgctl
./scripts/integration/opencode-mcp-smoke.sh
RGBUILDER_REQUIRE_OPENCODE=1 ./scripts/integration/opencode-mcp-smoke.sh
cargo test --release --test opencode_mcp_smoke -- --ignored --nocapture
```

**Pass:** `[opencode-smoke] OK — rgbuilder connected`  
**Skip:** `skip: opencode not on PATH` (exit 0 unless `RGBUILDER_REQUIRE_OPENCODE=1`)

See also [MCP server guide §11](../guides/mcp-server.md#11-opencode-smoke-test-host-integration).

### C2 — Full 7-tool matrix (`ecommerce-java`)

Golden ecommerce corpus: `rgbuilder-tests/ecommerce-java/` (~993 nodes, Java). Use when validating MCP tool payloads on a real indexed app (not just connect smoke).

**Prep (once per fresh checkout or after deleting `.rgbuilder`):**

```bash
export RGCTL=/path/to/rbuilder/target/debug/rgctl
export ECOMM=/path/to/rbuilder/rgbuilder-tests/ecommerce-java

cd "$ECOMM"
"$RGCTL" --no-daemon discover . --languages java -e target,data
"$RGCTL" --no-daemon semantic index
cp ../rgbuilder-policy.json ./policy.json
```

**OpenCode connect (optional):** scratch `opencode.json` in a temp dir with `cwd` = `$ECOMM` and the same `rgctl` command as C1. As of Aug 2026, **`opencode mcp list` may time out at 60s** on this repo while direct stdio MCP succeeds in ~1s — treat OpenCode timeout as a **host issue**, not proof that rgctl MCP is broken.

**Direct stdio tool calls (authoritative for tool matrix):** use `cargo test --test mcp_tools` pattern or a one-off client:

| # | Tool | Arguments | Expected highlights (ecommerce-java) |
|---|------|-----------|-------------------------------------|
| 1 | `rgbuilder_status` | `{}` | `command: pipeline_status`, `cfg_ready: true`, `semantic_ready: true` |
| 2 | `rgbuilder_query` | `query: "MATCH (n:Function) RETURN n LIMIT 5"` | `count: 5`, `schema_version: 1` |
| 3 | `rgbuilder_search` | `text: "order"`, `scope: "function"`, `limit: 5` | hits include `User.getOrders`, `OrderController.checkout` |
| 4 | `rgbuilder_impact` | `symbol: "findByEmail"` | resolves `UserRepository::findByEmail`, score ~41, 3 callers |
| 5 | `rgbuilder_metrics` | `pagerank: true` | `pagerank.top` array |
| 6 | `rgbuilder_cpg` | `op: "status"` | `archive_present: true`, ~308 functions |
| 7 | `rgbuilder_cpg` | `op: "function"`, `symbol: "findByEmail"` | FQN under `UserRepository` |
| 8 | `rgbuilder_check` | `policy_file: "<ECOMM>/policy.json"` | `passed: true`, `violations: []` |

Reference symbols from [ecommerce-java README](../../rgbuilder-tests/ecommerce-java/README.md).

**Recorded run (Aug 2026, M3 Pro):** all 8 calls above passed via direct stdio; OpenCode `mcp list` failed with 60s timeout on the same binary.

---

## MCP protocol note

OpenCode’s MCP SDK may send `protocolVersion: "2025-03-26"`. `rgbuilder-mcp` echoes supported client versions (`2025-03-26`, `2024-11-05`, `2024-10-07`). Unit test: `initialize_echoes_supported_client_protocol_version` in `crates/rgbuilder-mcp`.

---

## Related tests (not in matrix above)

| File | Role |
|------|------|
| `tests/discover_full_serve.rs` | `--full` pipeline + foreground serve + MCP unreadiness |
| `tests/cold_profile_gates.rs` | kafka, k8s-website markdown, obsidian export gates |
| Dashboard `tests/dashboard_ecommerce_*.rs` | Per-language dashboard bundles |

---

## Checklist before merging CLI/daemon/MCP changes

- [ ] Tier A green with `--test-threads=1`
- [ ] No new writes to source-tree `.rgbuilder` on daemon discover (see `auto_start_discover_writes_cache`)
- [ ] MCP tools still return CLI-shaped JSON (`mcp_tools.rs`)
- [ ] If touching OpenCode path: C1 smoke on tiny fixture
- [ ] If touching tool dispatch: C2 matrix on `ecommerce-java` (direct stdio)
- [ ] If touching discover perf: Tier B cold gate on reference machine (optional)

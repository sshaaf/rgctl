# Task plan: rename rgBuilder → rgctl

**Status:** ✅ implemented (Aug 2026)  
**Goal:** One product name (**rgctl**), one CLI binary (`rgctl`), one crate/workspace naming scheme, aligned docs and on-disk layout.

---

## Completion summary

| Phase | Status |
|-------|--------|
| 0 — Prep (audit script, rename scripts) | ✅ |
| 1 — Mechanical crate rename | ✅ |
| 2 — Runtime paths & daemon | ✅ |
| 3 — MCP & HTTP surface | ✅ |
| 4 — Agent skill bundle | ✅ |
| 5 — Docs & guides | ✅ |
| 6 — Dashboard, website, scripts, CI | ✅ |
| 7 — Test corpus & harnesses | ✅ |
| 8 — External (GitHub repo, site URLs) | ⏳ manual follow-up |

---

## What changed

| Layer | Before | After |
|-------|--------|-------|
| Root Cargo package | `rgbuilder` | **`rgctl`** |
| Workspace crates (33) | `rgbuilder-*` | **`rgctl-*`** |
| Proc-macros | `rgbuilder-macros` | **`rgctl-macros`** |
| Product / docs brand | rgBuilder | **rgctl** |
| Repo artifacts | `.rgbuilder/` | **`.rgctl/`** (+ migration from `.rgbuilder`, `.rbuilder`) |
| Daemon state | `~/.rgbuilder/` | **`~/.rgctl/`** (+ migration) |
| Env vars | `RGBUILDER_*`, `RBUILDER_*` | **`RGCTL_*`** (+ legacy read one release) |
| MCP tools | `rgbuilder_*` | **`rgctl_*`** |
| Agent skill | `skills/rgbuilder/` | **`skills/rgctl/`** |
| Test corpus | `rgbuilder-tests/` | **`rgctl-tests/`** |
| Project config type | `RgbuilderConfig` | **`RgctlConfig`** |

---

## Migration behavior

### Artifact dirs (`crates/rgctl-graph/src/paths.rs`)

```text
.rbuilder  →  .rgbuilder  →  .rgctl   (one-shot rename chain)
```

### Daemon home (`src/cli/daemon/config.rs`)

If `~/.rgctl/` missing and `~/.rgbuilder/` exists → rename to `~/.rgctl/`.

### Env vars

Canonical: `RGCTL_*`. Legacy read: `RGBUILDER_*`, then `RBUILDER_*`.

---

## Verification (run before merge)

```bash
./scripts/rename-audit.sh
cargo build --release --bin rgctl
cargo test --test rgctl_daemon --test rgctl_no_daemon --test mcp_tools --test install_skill -- --test-threads=1
```

All of the above pass as of implementation.

---

## Phase 8 — External (manual, post-merge)

- [ ] GitHub repo rename (`sshaaf/rgBuilder` → `sshaaf/rgctl`) + redirect
- [ ] Website deploy path updates
- [ ] Release notes announcement for MCP tool rename

---

## Scripts added

- `scripts/rename-to-rgctl.sh` — directory git mv (one-time)
- `scripts/rename-content.py` — bulk content replacement
- `scripts/rename-audit.sh` — CI guard against stale names

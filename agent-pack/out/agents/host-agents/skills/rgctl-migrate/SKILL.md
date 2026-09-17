---
name: rgctl-migrate
description: "Migration roadmap. Use for rgctl migrate workflow. Spawn rgctl -f json; parse schema_version from stdout."
rgctl-managed: true
metadata:
  generatedBy: "rgctl 0.4.13"
---

# Migrate workflow

**Primary output:** `.rgctl/migration_plan.json` (and dashboard migration view via `serve --open`).

**This workflow is not Kantra.** Do not treat `--with-kantra` or `kantra_findings.json` as the main deliverable here.

```bash
rgctl -r "$REPO" discover . --with-cfg --with-harmonic --export-migration-hints \
  --migration-preset hybrid_default --migration-order scheduled
rgctl -r "$REPO" -f json metrics --pagerank
```

Presets: `hybrid_default`, `foundational_first`, `dense_cluster`, `risk_mitigation`.  
Orders: `scheduled` (dependency-aware), `priority` (score rank).

Report plan path, preset/order, and top packages — not raw discover telemetry.


## Agent loop

1. Parse the user question (natural language).
2. Run `rgctl -f json <command> …` (or `rgctl serve` + HTTP for repeated queries).
3. Parse `schema_version` and payload from **stdout** only.
4. Summarize facts; do not dump raw JSON.
5. Re-query if the graph may be stale after edits.

**Never** redirect stderr to `/dev/null`. If `.rgctl/` exists and the question is structural, use rgctl before ripgrep or bulk file reads.

```bash
export REPO=/path/to/repo
rgctl -r "$REPO" -f json <command>
```


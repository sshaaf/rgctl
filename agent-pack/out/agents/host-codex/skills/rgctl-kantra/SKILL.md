---
name: rgctl-kantra
description: "Konveyor Kantra rules. Use for rgctl kantra workflow. Spawn rgctl -f json; parse schema_version from stdout."
rgctl-managed: true
metadata:
  generatedBy: "rgctl 0.4.13"
---

# Kantra workflow

**Primary output:** `.rgctl/kantra_findings.json` and `KantraRule` / `VIOLATES` in the graph.

**This workflow is not migration roadmap export.** Do not present `migration_plan.json` as the main Kantra deliverable.

```bash
rgctl -r "$REPO" discover . -l java --with-kantra
rgctl -r "$REPO" discover . -l java --with-kantra --kantra-target quarkus
rgctl -r "$REPO" -f json gql 'MATCH (r:KantraRule) RETURN r LIMIT 20'
```

Overrides: `--kantra-rules DIR`, `--kantra-catalog ROOT`, `--kantra-index-only` (index without eval).

For extraction ordering after violations, use the **migrate** workflow separately.


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


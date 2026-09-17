---
name: rgctl-impact
description: "Blast radius and impact. Use for rgctl impact workflow. Spawn rgctl -f json; parse schema_version from stdout."
rgctl-managed: true
metadata:
  generatedBy: "rgctl 0.4.13"
---

# Impact workflow

**When:** Before refactors, renames, or API changes.

```bash
rgctl -r "$REPO" -f json blast-radius SYMBOL [--depth N] [--class NAME] [--file PATH]
```

Disambiguate symbols with `--class` or `--file` when names collide. Report hop depth and top callers/callees from JSON payload.


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


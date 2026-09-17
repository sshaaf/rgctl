---
name: rgctl-gql
description: "Graph query language. Use for rgctl gql workflow. Spawn rgctl -f json; parse schema_version from stdout."
rgctl-managed: true
metadata:
  generatedBy: "rgctl 0.4.13"
---

# GQL workflow

**When:** Ad-hoc graph queries, inventories, call neighborhoods.

```bash
rgctl -r "$REPO" -f json gql 'MATCH (n:Function) WHERE n.name LIKE "*Service*" RETURN n LIMIT 20'
rgctl -r "$REPO" -f json gql --macro-name all_functions unused
```

Use **qualified_name** / FQN for classes, not bare `n.name` when disambiguating. Always use **LIMIT** on broad patterns. Explain macros before inventing raw GQL.


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


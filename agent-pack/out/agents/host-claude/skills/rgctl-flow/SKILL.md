---
name: rgctl-flow
description: "Data flow and slices. Use for rgctl flow workflow. Spawn rgctl -f json; parse schema_version from stdout."
rgctl-managed: true
metadata:
  generatedBy: "rgctl 0.4.13"
---

# Flow workflow

**When:** Slices, PDG, taint, CPG data flows. Requires `discover --with-cfg`.

```bash
rgctl -r "$REPO" -f json slice FILE --line N --variable V [--function F] [--direction backward|forward]
rgctl -r "$REPO" -f json cpg flows FILE --line N --variable V --function F
```

Check readiness: `rgctl -f json cpg status`.


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


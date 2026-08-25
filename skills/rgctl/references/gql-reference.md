# GQL Quick Reference

rgctl's Graph Query Language (GQL) is a Cypher subset for querying the code knowledge graph.

## Capabilities & Limitations

### Supported Cypher Features

- `MATCH` patterns
- `WHERE` filters
- `RETURN` projections
- `LIMIT` caps
- Variable-length paths (`[:CALLS*1..3]`)
- Node labels (`:Function`, `:Class`, `:Module`)
- Property filters (`WHERE n.name = 'foo'`)
- `LIKE` for prefix/suffix matching

### NOT Supported

- `COUNT`, `ORDER BY`, `GROUP BY`, `COLLECT` (no aggregation)
- `WHERE n.id = '<uuid>'` (node UUID is not queryable — use `cpg function` or `blast-radius`)
- Computed properties or expressions in RETURN
- Subqueries or UNION
- `WITH` clauses
- `CREATE`, `DELETE`, `SET` (read-only)

## Macros

Pre-defined queries accessed via `--macro-name`:

| Macro | Intent |
|-------|--------|
| `all_functions` | Inventory all functions |
| `all_communities` | List all communities |
| `direct_calls` | Direct CALLS edges from a symbol |
| `call_chain` | Multi-hop CALLS paths |

**Usage:** `rgctl -f json gql --macro-name all_functions unused`

Always pass a positional query string with `--macro-name` (use `unused` if the macro doesn't need it).

## Edge Types

Valid relationship types in MATCH patterns:

| Edge | Meaning |
|------|---------|
| `CALLS` | Function A calls function B |
| `CONTAINS` | Module contains function/class |
| `USES` | Uses a dependency |
| `IMPLEMENTS` | Implements interface |
| `EXTENDS` | Extends class |
| `REFERENCES` | References symbol |
| `INSTANTIATES` | Creates instance |
| `MODIFIES` | Writes to field |
| `USESCONFIG` | Uses configuration |
| `DEFINEDIN` | Defined in file |
| `DEPENDSON` | Module-level dependency |

**Invalid edge types:** `DEPENDS`, `IMPORTS` (these don't exist)

## LIKE Pattern Matching

**Only single-sided wildcards work:**
- `n.name LIKE 'Handle*'` — prefix (starts-with) ✓
- `n.name LIKE '*Handler'` — suffix (ends-with) ✓
- `n.name LIKE '*middle*'` — **silently returns 0** ✗

**Path matching:**
- `WHERE n.file LIKE ...` — **always returns 0** (file paths not filterable in WHERE)

**Alternative for substring/contains:**
- Use `semantic query "<concept>"`
- Use `communities list` + grep labels
- Use `blast-radius` with `--file` filter

## CALLS Edge Limitations

CALLS edges track **static call sites only**. Missing cases:

- Interface/trait implementations
- Virtual method dispatch
- Receiver methods (in some languages)
- Dynamic dispatch
- Reflection-based calls

**Symptom:** `CALLS*1..N` returns 0 edges for a method you know is called

**Workaround:** Fall back to `grep` for call sites in source

## Common Patterns

### Find Callers (Incoming)

```bash
rgctl -f json gql "MATCH (a:Function)-[:CALLS]->(b:Function) 
  WHERE b.name = 'checkout' RETURN a,b LIMIT 20"
```

### Find Callees (Outgoing)

```bash
rgctl -f json gql "MATCH (a:Function)-[:CALLS]->(b:Function) 
  WHERE a.name = 'checkout' RETURN a,b LIMIT 20"
```

### Multi-Hop Paths

```bash
rgctl -f json gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) 
  WHERE a.name = 'updateQuantity' RETURN a,b LIMIT 50"
```

### Filter by Community

```bash
rgctl -f json gql "MATCH (f:Function) 
  WHERE f.community_id = '12' RETURN f LIMIT 20"
```

### Name Pattern (Suffix)

```bash
rgctl -f json gql "MATCH (n:Function) 
  WHERE n.name LIKE '*Service' RETURN n LIMIT 20"
```

### Multiple Node Types

```bash
rgctl -f json gql "MATCH (n) 
  WHERE n.name LIKE 'Config*' RETURN n LIMIT 20"
```

## Troubleshooting

### Query Returns 0 Results

1. **LIKE contains pattern** — `*middle*` doesn't work; use `semantic query`
2. **Concept in package/directory name** — try `communities list`, `semantic query`, or broaden to non-Function types
3. **Dynamic dispatch** — CALLS won't show virtual/interface calls; use `grep`
4. **UUID in WHERE** — can't query by node ID; use `cpg function <uuid>` instead

### Relationship Queries

**User asks:** "What's the relationship between X and Y?"

1. Resolve symbols via `semantic query` or `gql`
2. Run bounded CALLS/DEPENDSON traversal
3. Report hops, shared neighbors, files
4. If no direct path, try `blast-radius` on each

## Field Reference

Common node properties (language-dependent):

- `name` — symbol name (may be bare method name, not FQN)
- `qualified_name` — fully-qualified name
- `file_path` — source file path
- `community_id` — cluster ID (if analysis ran)
- `language` — source language
- `line` — definition line number
- `id` — internal UUID (not queryable in WHERE)

## See Also

- [Graph Query Language Guide](../../docs/guides/graph-query-language.md) - In-depth GQL tutorial
- [Command Encyclopedia](command-encyclopedia.md) - Full gql command reference
- [JSON API](../../docs/json-api.md) - Response schemas

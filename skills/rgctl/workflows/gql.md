# GQL workflow

**When:** Ad-hoc graph queries, inventories, call neighborhoods.

Use **qualified_name** / FQN for classes, not bare `n.name` when disambiguating. Always use **LIMIT** on broad patterns. Explain macros before inventing raw GQL.

### Function inventory

**User intent:** *"Give me an inventory of functions … candidates to delete or shrink"*

```bash
rgctl -f json gql --macro-name all_functions unused
```

`all_functions` → full inventory (`count` + `rows`). `unused` is a **placeholder**. Cross-check with blast-radius / CALL queries before deletes.

### Named communities

**User intent:** *"What architectural communities / packages does the graph see?"*

```bash
rgctl -f json gql --macro-name all_communities unused
# prefer for labels + modularity: rgctl -f json communities list
```

Lists communities — **not** "orphaned modules." Inspect members and call edges before proposing a prune.

### Pattern search

**User intent:** *"Find all Service classes … naming consistency"*

```bash
rgctl -f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service' RETURN n LIMIT 20"
```

Suffix-only — `*middle*` silently returns 0. For contains-style search, use `semantic query "Service"` instead.

### Community members

**User intent:** *"List all the functions inside Community 12"*

```bash
rgctl -f json gql "MATCH (f:Function) WHERE f.community_id = '12' RETURN f LIMIT 20"
```

### Call neighborhood

**User intent:** *"Show me the call stack surrounding `updateQuantity` up to 3 hops"*

```bash
rgctl -f json gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function)
  WHERE a.name = 'updateQuantity' RETURN a,b LIMIT 50"
```

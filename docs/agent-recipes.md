# Agent recipes

Copy-paste workflows for LLM agents and automation. All commands assume:

```bash
export REPO=/path/to/repo   # contains .rgctl/ after discover
```

**JSON shapes / field tables:** [json-api.md](json-api.md)

> **jq field contract:** use the exact field names from [json-api.md](json-api.md) (e.g. `direct_callers_count`, not `direct_caller_count`). Smoke-test recipes after schema bumps.

---

## Recipe 1 — Orient in an unfamiliar repo

```bash
rgctl -r "$REPO" discover .
rgctl -r "$REPO" -f json discover . | jq '.metrics'
rgctl -r "$REPO" -f json gql --macro-name all_functions unused | jq '.count'
rgctl -r "$REPO" -f json gql --macro-name all_communities unused | jq '.rows[:5]'
rgctl -r "$REPO" -f json metrics --pagerank | jq '.rows[:10]'
```

**Use when:** first turn on a codebase; replaces reading directory trees.

---

## Recipe 1b — Named communities

```bash
rgctl -r "$REPO" communities list
rgctl -r "$REPO" -f json gql 'MATCH (c:Community) RETURN c' | jq '.rows[:10]'
# members of community 12 (id from list / communities.json):
rgctl -r "$REPO" -f json gql "MATCH (f:Function) WHERE f.community_id = '12' RETURN f LIMIT 20"
# optional: refresh heuristic labels into analysis_results.bin
rgctl -r "$REPO" communities label --write
```

**Use when:** mapping subsystems without reading `communities.json` by hand. Labels are heuristic (package / path / token); they are **not** written into the topology graph.

## Recipe 2 — Before editing a symbol

```bash
SYMBOL=ShoppingCartService
rgctl -r "$REPO" -f json blast-radius "$SYMBOL" | jq '{
  score: .metrics.score,
  direct_callers: .metrics.direct_callers_count,
  impact_zone: .metrics.impact_zone_size
}'
rgctl -r "$REPO" -f json blast-radius "$SYMBOL" --depth 3 | jq '.topology.direct_callers[:10]'
```

If the name is ambiguous, disambiguate:

```bash
rgctl -r "$REPO" blast-radius process --class ShoppingCartService
```

**Use when:** agent plans a refactor or bugfix; avoids missing upstream callers.

---

## Recipe 3 — Find entrypoints / APIs

```bash
rgctl -r "$REPO" -f json gql \
  "MATCH (n:Function) WHERE n.name LIKE '*Endpoint' RETURN n LIMIT 20" \
  | jq '.rows[].n.name'
```

**Use when:** tracing HTTP handlers or CLI entrypoints.

---

## Recipe 3b — Natural-language function discovery

```bash
rgctl -r "$REPO" semantic index
rgctl -r "$REPO" -f json semantic query "shopping cart checkout" --limit 10 \
  | jq '.hits[] | {name, file_path, score: .fused_score}'
# Fusion is on by default; add --keyword-and to require every query token to match
rgctl -r "$REPO" -f json semantic query "OrderService validate" --keyword-and \
  | jq '.hits[:5]'
```

**Use when:** the agent knows intent but not exact symbol names; complements GQL `LIKE` patterns.

---

## Recipe 4 — Call chain neighborhood

```bash
rgctl -r "$REPO" -f json gql \
  "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) RETURN a,b LIMIT 50"
```

**Use when:** understanding feature locality without opening every file.

---

## Recipe 5 — Data-flow check at a line (needs `discover --with-cfg`)

```bash
rgctl -r "$REPO" discover . --with-cfg
rgctl -r "$REPO" -f json slice \
  src/main/java/com/example/Service.java \
  --line 42 --variable request --function handleRequest \
  | jq '.lines'
```

Note: `--function` is the **method name**, not the class name.

**Use when:** verifying what affects a variable before changing logic.

---

## Recipe 6 — Taint sanity check

```bash
rgctl -r "$REPO" discover . --with-cfg
rgctl -r "$REPO" -f json slice src/.../Controller.java \
  --line 30 --variable param --function handle --taint | jq '.flows'
```

**Use when:** security-sensitive edits (user input → sink).

---

## Recipe 7 — Migration batch planning

```bash
rgctl discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints
# Prefer root plan from --export-migration-hints; dashboard copy exists when --with-dashboard ran
jq '.packages[:10]' "$REPO/.rgctl/migration_plan.json"
rgctl serve --open   # Migration tab for interactive tuning
```

**Use when:** monolith extraction ordering for humans or agents.

---

## Recipe 8 — CI policy on a branch

```bash
cp docs/examples/policy-strict.json policy.json
rgctl -r "$REPO" -f json check --policy-file policy.json
# exit 1 → violations in .violations[]
```

**Use when:** blocking PRs that touch high-impact symbols.

---

## Recipe 9 — HTTP session (many queries)

```bash
rgctl -r "$REPO" serve &
curl -sS -X POST http://127.0.0.1:8080/api/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"MATCH (n:Function) RETURN n LIMIT 5"}' | jq '.count'
```

See [http-api.md](http-api.md).

---

## Recipe 10 — Export subgraph for external tools

```bash
# Filter syntax (not GQL MATCH):
rgctl -r "$REPO" export --export-format graphml \
  --export-output service.graphml --query "name:ShoppingCartService"
rgctl -r "$REPO" export --export-format mermaid \
  --export-output all-calls.mmd --query all
```

**Use when:** handing a neighborhood to GraphML/Gephi or docs.

---

## Recipe 12 — Obsidian vault from markdown graph

```bash
export REPO=/path/to/repo
rgctl -r "$REPO" discover . -l markdown

rgctl -r "$REPO" export \
  --export-format obsidian \
  --export-output "$REPO/vault" \
  --query all
```

Open `$REPO/vault` in Obsidian. One note per heading section; wikilinks from doc cross-references; `qualified_name` in frontmatter for GQL correlation.

Optional NL search on sections (build doc index first; query has no `--embedder`):

```bash
rgctl -r "$REPO" semantic index --scope docs --embedder hash
rgctl -r "$REPO" -f json semantic query "checkout flow" --scope docs --limit 10
```

**Use when:** browsing or editing docs in Obsidian while keeping rgctl as the structural index. Large corpora: `./scripts/fetch-profile-repos.sh` + `example/k8s-website` (~17k Obsidian notes). Doc semantic index includes heading + `code_block` modules; re-run index after doc changes. See [markdown-context.md](markdown-context.md#semantic-search-doc-sections).

---

## Recipe 11 — DTO / cart mutation safety (hybrid CPG)

```bash
rgctl -r "$REPO" discover . --with-cfg
# Optional fidelity: --with-dfg-loops  --with-ast-skeleton

# CoolStore ShoppingCart (ecommerce-* fixtures) — non-constructor field writes:
rgctl -r "$REPO" -f json cpg mutations --type ShoppingCart --exclude-ctors

# Same pattern for a DTO / record candidate (substitute your type name):
# rgctl -r "$REPO" -f json cpg mutations --type OrderDTO --exclude-ctors

# After picking a hit at file:line, forward flows on the receiver:
rgctl -r "$REPO" -f json cpg flows \
  src/main/java/com/example/ecommerce/coolstore/service/ShoppingCartService.java \
  --line 75 --variable sc --function priceShoppingCart --direction forward --with-alias

# Optional: coarse syntax tree for the function
rgctl -r "$REPO" -f json cpg ast priceShoppingCart

# Optional: export L_repo (+ L_proc if archived) for Joern/Neo4j tooling
rgctl -r "$REPO" cpg export --format graphson --output cart-cpg.json --path-contains coolstore/
```

**Use when:** proving immutability before converting a mutable cart/DTO to a `record`, or locating pricing side effects on `ShoppingCart`. Empty mutations ⇒ no typed non-ctor field writes found (unresolved receivers excluded unless `--include-unresolved`). On C fixtures use the struct typedef (`shopping_cart_t`). Requires `--with-cfg`. `--with-alias` expands may-alias names (copies + field bases). See [User Guide §10](user-guide.md#10-hybrid-cpg-cpg) and [hybrid-cpg-plan.md](design/hybrid-cpg-plan.md).

---

## See also

- [AGENTS.md](../AGENTS.md)
- [User Guide](user-guide.md)

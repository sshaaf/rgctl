---
metadata:
  author: rgctl-fixture
  team: platform-docs
  scope: markdown-context-example
---

# Markdown context graph — example corpus

This directory is a **minimal but realistic** repo used to demo and test rgctl’s markdown context graph ([issue #56](https://github.com/sshaaf/rgctl/issues/56)).

It is not production software. It models how **docs, ADRs, and code** land in the same `graph.snapshot.bin` so agents can query structure instead of reading every file.

**Note:** This is isolated from the parent rgctl repo. When you set `REPO` to this folder, the root [AGENTS.md](https://github.com/sshaaf/rgctl/blob/main/AGENTS.md) of rgctl is **not** indexed — only files under this tree.

## Layout

```text
markdown-context/
  README.md           ← you are here (YAML frontmatter demo)
  docs/
    guide.md          ← product guide (headings + cross-links)
    adr.md            ← architecture decision records
    overview.mdx      ← `.mdx` sample (same markdown plugin)
  src/
    CheckoutService.java  ← code anchor for doc→class queries
```

## What gets indexed

| File | Graph content |
|------|----------------|
| Headings (`#`, `##`) | `:Module` nodes, `kind=heading`, QN `{path}#{slug}` |
| Internal `[links](...)` | `:Import` link nodes + `REFERENCES` edges |
| Fenced code blocks | `:Module`, `kind=code_block` |
| YAML frontmatter (this README) | `:Variable`, flattened keys (`metadata.author`) |
| `.java` (with `-l java`) | Classes/methods; `CONTAINS` from file nodes |

## Agent workflow (graph-first)

Before editing checkout code in this example, query the graph:

```bash
rgctl -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:CONTAINS*1..3]->(n) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' RETURN h, n LIMIT 20"
```

Then read [Checkout Flow](docs/guide.md#checkout-flow) and [Payments ADR](docs/adr.md#payments) only if you need prose detail.

## Try it

From the rgctl repo root (build the CLI first: `cargo build --bin rgctl`):

```bash
export REPO="$(pwd)/tests/fixtures/markdown-context"
RGB="$(pwd)/target/debug/rgctl"

# Docs only (Phase 2a)
"$RGB" -r "$REPO" discover . -l markdown

# Docs + code (Phase 2b)
"$RGB" -r "$REPO" discover . -l markdown,java

# Example: find checkout-related headings
"$RGB" -r "$REPO" -f json gql \
  "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n LIMIT 10"

# Example: guide links to the payments ADR section
"$RGB" -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:REFERENCES]->(t:Module) WHERE h.kind = 'heading' AND t.name = 'Payments' RETURN h, t"

# Example: doc → Java class (needs markdown,java discover)
"$RGB" -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' RETURN h, f, c"

# Obsidian vault (one note per heading; open vault/ in Obsidian)
"$RGB" -r "$REPO" export --export-format obsidian --export-output "$REPO/vault" --query all

# Doc semantic search (optional — index scope docs first; no --embedder on query)
"$RGB" -r "$REPO" semantic index --scope docs --embedder hash
"$RGB" -r "$REPO" -f json semantic query "checkout flow" --scope docs --limit 5
```

Artifacts appear under `$REPO/.rgctl/` after discover (`graph.snapshot.bin`, `content_store.bin` when bodies are large). Vault export writes `$REPO/vault/`.

## Narrative (what the graph encodes)

1. **Checkout Flow** (`docs/guide.md`) describes the user journey and links to:
   - the **Payments** section in `docs/adr.md` (heading link),
   - the whole ADR file (file link),
   - `CheckoutService.java` (code link).
2. **Cart** is a child heading under Checkout Flow (`CONTAINS` in the graph).
3. **README frontmatter** (`metadata.*`) appears as `:Variable` nodes in the graph.
4. With Java enabled, query 6 walks: heading → file → `CheckoutService` class.

Full query catalog: [docs/markdown-context.md](../../../docs/markdown-context.md) in the main repo.

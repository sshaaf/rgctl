# Markdown context graph

rgBuilder indexes `.md` and `.mdx` through the **custom markup plugin** `rgbuilder-lang-markdown` (not Tier 1, not generic Tier 2). It uses official `tree-sitter-md` (block + inline grammars) to build a documentation context graph alongside code.

## Discover

Markdown is registered in `default_registry()` — `discover` indexes `.md` / `.mdx` by default (same as other built-in languages). Filter with `-l markdown` when you only want docs:

```bash
export REPO=/path/to/repo
rgctl -r "$REPO" discover . -l markdown
rgctl -r "$REPO" discover . -l markdown,java   # doc + code (Phase 2b)
```

Fixture corpus: `tests/fixtures/markdown-context/` — start with its [README.md](../tests/fixtures/markdown-context/README.md) for layout, narrative, and copy-paste commands.

Automated integration gate: `cargo test --test markdown_context_cli` (CLI discover + GQL) and `cargo test -p rgbuilder-extraction markdown_spec_coverage` (in-memory spec matrix).

## Cold profile (kubernetes/website)

Large real-world markdown corpus: [kubernetes/website `content/en`](https://github.com/kubernetes/website/tree/main/content/en). Same **cold profile** pattern as `example/linux` — gitignored local checkout, deletes `.rgbuilder/` before discover, release `rgctl` only.

**Cold profile definition:** run a **fresh** release build right before profiling:

```bash
cargo build --release --bin rgctl
```

Use that newly built `target/release/rgctl`; do not use debug or stale release binaries for cold profile comparisons.

**Agent prompt (suggested):**

> Run **cold profile** on markdown: `cargo build --release --bin rgctl`, `./scripts/fetch-profile-repos.sh`, then `cargo test --release --test cold_profile_gates k8s_website_markdown_cold_discover_within_baseline -- --ignored --nocapture`. Report `[profile] discover summary` wall_secs, nodes, functions, and `index_graph_build` vs baseline 3s (+10%). Compare to last known good on this machine. Do not use an existing `.rgbuilder/` cache.

```bash
./scripts/fetch-profile-repos.sh
cargo build --release --bin rgctl
cargo test --release --test cold_profile_gates k8s_website_markdown_cold_discover_within_baseline -- --ignored --nocapture
```

- Discover root: `example/k8s-website` (override with `RGBUILDER_K8S_WEBSITE_REPO`)
- Command: cold `discover . -l markdown -v` (markdown plugin only; no CFG)
- Baseline: **3.0s** profile wall_secs (+10% tolerance); override with `RGBUILDER_K8S_WEBSITE_DISCOVER_BASELINE_SECS` after you establish a number on your machine
- Correctness: ≥500 heading modules, zero `:Function` nodes
- **Obsidian export gate** (warm index, does not re-run discover): `cargo test --release --test cold_profile_gates k8s_website_obsidian_export_to_vault -- --ignored --nocapture` — baseline **30s** wall (+10%); override with `RGBUILDER_K8S_WEBSITE_OBSIDIAN_EXPORT_BASELINE_SECS`. Expect ~17k notes, note count = heading count.

See [example/README.md](../example/README.md) for other large local corpora.

## Node model

| Source | GQL label | `kind` property | Notes |
|--------|-----------|-----------------|-------|
| ATX/setext headings | `:Module` | `heading` | Filter `n.kind = 'heading'` — do not use bare `:Module` |
| Markdown links | `:Import` | `markdown_link` | Every link is a node (node inflation on link-heavy docs) |
| Fenced/indented code | `:Module` | `code_block` | `language` property from info string |
| Frontmatter keys | `:Variable` | `frontmatter` | Flattened dotted keys (`metadata.author`); `value` holds scalar text |

Qualified names: `{file_path}#{slug}` (ASCII slugify; duplicates get `-2`, `-3`, …).

### Content payloads (v1)

Agents can read section prose from the graph instead of opening files:

| Property | On | Meaning |
|----------|-----|---------|
| `body_text` | `:Module` (`heading`, `code_block`), `:Variable` (`frontmatter`) | Inline UTF-8 payload when ≤ 32 KiB |
| `body_hash` | same | Blake3 hex digest of full body (even when truncated inline) |
| `body_ref` | same | Blake3 hex pointer into `content_store.bin` when truncated |
| `content_hash` | `:File` | Blake3 hex of full file bytes |
| `blob_ref` | `:File` | Points into `content_store.bin` for large files |
| `value` | `:Variable` (`frontmatter`) | Scalar frontmatter value as string |

**Heading sections:** `body_text` is prose from the heading through the next heading (any level), excluding nested headings. `end_line` on the node spans that same range so `code_hash` / code index align.

**Code fences:** `body_text` is fence inner content (not delimiter lines).

Large corpora: bodies beyond the inline cap get `body_truncated`, `body_ref` (same Blake3 hex as `body_hash`), and full UTF-8 in `.rgbuilder/content_store.bin`. `:File` nodes carry `content_hash` (Blake3 of raw file bytes) and `blob_ref` when the file exceeds the inline cap.

## Obsidian vault export

Turn the markdown context graph into an **Obsidian vault** — one note per heading section, folder layout mirroring doc paths, wikilinks from `REFERENCES` edges. Export reads the graph + `content_store.bin` (no re-parse of source files).

### 1. Index markdown

```bash
export REPO=/path/to/repo
cargo build --release --bin rgctl   # release is faster for large exports
export PATH="$PWD/target/release:$PATH"

rgctl -r "$REPO" discover . -l markdown
# or docs + code: rgctl -r "$REPO" discover .
```

### 2. Export vault

```bash
rgctl -r "$REPO" export \
  --export-format obsidian \
  --export-output "$REPO/vault" \
  --query all
```

```text
Exported 17244 notes (2971 wikilinks) -> /path/to/repo/vault
```

| Flag | Meaning |
|------|---------|
| `--export-format obsidian` | Write Obsidian-compatible markdown notes |
| `--export-output` | Vault root (any folder; use `"$REPO/vault"` to keep it inside the repo) |
| `--query all` | Export every heading module (Obsidian does not use filter queries yet) |

### 3. Open in Obsidian

Obsidian → **Open folder as vault** → select `$REPO/vault`.

### Vault layout

| Graph | Vault path |
|-------|------------|
| `docs/guide.md#checkout-flow` | `docs/guide/checkout-flow.md` |
| `blog/_posts/2024/release.md#feature-x` | `blog/_posts/2024/release/feature-x.md` |

Long heading slugs (e.g. k8s blog posts) are truncated with a stable hash suffix so filenames stay within OS limits.

### Note shape

```markdown
---
qualified_name: "docs/guide.md#checkout-flow"
level: "2"
---

Section prose (from `body_text` or `content_store.bin` via `body_ref`).

[[docs/adr/payments]]
[[docs/adr]]
```

- **Frontmatter** — `qualified_name` ties back to GQL (`WHERE n.qualified_name = '…'`).
- **Body** — heading section text; large bodies resolved from `content_store.bin`.
- **Wikilinks** — outgoing `REFERENCES` edges as `[[vault-relative/path]]` (no `.md` suffix).

### Re-export after doc edits

```bash
rgctl -r "$REPO" discover . -l markdown
rgctl -r "$REPO" export --export-format obsidian --export-output "$REPO/vault" --query all
```

Obsidian export is **read-only** — edits in Obsidian are not synced back to the graph.

### Quick examples

**Fixture** (~16 notes):

```bash
export REPO="$(pwd)/tests/fixtures/markdown-context"
rgctl -r "$REPO" discover . -l markdown
rgctl -r "$REPO" export --export-format obsidian --export-output "$REPO/vault" --query all
```

**kubernetes/website** (~17k notes, ~5–20s release export after fetch + discover):

```bash
./scripts/fetch-profile-repos.sh
export REPO="$(pwd)/example/k8s-website"
rgctl -r "$REPO" discover . -l markdown
rgctl -r "$REPO" export --export-format obsidian --export-output "$REPO/vault" --query all
```

### OKF JSON export

```bash
rgctl -r "$REPO" export --export-format okf --export-output "$REPO/okf.json" --query all
```

Entity bundle for Open Knowledge Foundation tooling (heading modules + bodies).

## Semantic search (doc sections)

Default `semantic index` embeds **`:Function` nodes only**. For documentation:

```bash
rgctl -r "$REPO" discover . -l markdown   # or full discover

# Index doc sections (offline embedder — no ONNX)
rgctl -r "$REPO" semantic index --scope docs --embedder hash

# Query (embedder comes from the saved index — no --embedder on query)
rgctl -r "$REPO" -f json semantic query "checkout flow" --scope docs --limit 10
```

### Index scope vs query scope

| Step | Flag | What it does |
|------|------|----------------|
| **`semantic index --scope`** | `function` (default) | Embeds `:Function` only |
| | `docs` | Embeds `:Module` with `kind=heading` **and** `kind=code_block` |
| | `all` | Functions + doc modules above |
| **`semantic query --scope`** | `community` | Pooled community search (needs `analysis_results.bin`) |
| | `docs` / `function` / `all` | **Does not filter hits today** — results come from whatever was built into `semantic_index.bin` |

**Rule:** build the index with the scope you need (`--scope docs` for NL doc search). Re-run `semantic index` when switching scope or after large doc edits. On a markdown-only repo, default function index is empty.

**Bodies:** embeddings use `body_text` inline, or full UTF-8 from `.rgbuilder/content_store.bin` when `body_ref` is set (same store as Obsidian export).

**CLI note:** success text still says `Indexed N functions` — the count is **index entries** (doc sections when `--scope docs`).

### Scope summary

| Index `--scope` | Nodes embedded |
|-----------------|----------------|
| `function` (default) | `:Function` |
| `docs` | `:Module` `kind=heading` + `kind=code_block` |
| `all` | Functions + doc modules above |

GQL remains the low-token default for structural navigation; semantic `--scope docs` helps natural-language section search after a doc-scoped index build.

## GQL body text (agents)

```bash
rgctl -r "$REPO" -f json gql \
  "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n.body_text LIMIT 1"
```

When truncated inline, query `body_ref` / read `content_store.bin`, or use Obsidian export for human browsing.

## Author linking guide

**File links** (no `#`): href resolves relative to the markdown file’s directory. Graph edge `REFERENCES` targets the **File** node (`to_type_hint = file`). If the file is not in the discover set, the edge is **dropped** (no Class stub).

**Heading links** (`#fragment`): fragment is **literal** (never slugified). Target is a `:Module` with `kind=heading` or a Module stub if the heading does not exist.

| Author writes | Resolves to | Good? |
|---------------|-------------|-------|
| `./adr.md` | File `docs/adr.md` | Yes (file link) |
| `./adr.md#payments` | Module `docs/adr.md#payments` | Yes (literal fragment) |
| `#checkout-flow` | Same-file heading slug | Yes |
| `#Checkout Flow` | Module stub (fragment not slugified) | Avoid — use slug |
| `../src/Foo.java` | File node ending in `Foo.java` | Yes (code link) |
| `https://…` | No edge | External — ignored |

## GQL queries (Phase 2)

`LIKE` uses prefix/suffix glob only (`Checkout*`, `*adr.md`). No infix `*Checkout*`.

**Phase 2a** (`-l markdown`):

1. `MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n`
2. `MATCH (a:Module)-[:CONTAINS]->(b:Module) WHERE a.kind = 'heading' AND b.kind = 'heading' RETURN a, b`
3. `MATCH (h:Module)-[:REFERENCES]->(f:File) WHERE h.kind = 'heading' AND f.name LIKE '*adr.md' RETURN h, f`
4. `MATCH (h:Module)-[:REFERENCES]->(t:Module) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND t.kind = 'heading' RETURN h, t`
5. `MATCH (h:Module)-[:CONTAINS*1..3]->(n:Module) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND n.kind = 'heading' RETURN h, n`

**Phase 2b** (`-l markdown,java`):

6. `MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' RETURN h, f, c`

Query 6 finds doc → Java **file → class** via existing `REFERENCES` and `CONTAINS`. It does **not** include `Calls`, method-level symbols, or `blast-radius` into markdown.

## Other properties

- `WHERE n.file_path = 'docs/guide.md'` — GQL resolves `file_path` from the node (not only the properties map).
- **Concept blast** for docs: use GQL `CONTAINS` / `REFERENCES` (queries 4–6). `blast-radius` CLI remains **Calls-only**.

## PageRank and communities

Doc `REFERENCES` edges participate in discover-time centrality ([`default_behavioral_edges`](../../crates/rgbuilder-analysis/src/centrality.rs)) and community detection (`default_community_edge_types` includes `References`).

**Communities at discover:** `detect_with_view_defaults` projects neighbors via `build_community_neighbor_lists`:

- Always: `Calls`, `Uses`, `References` (when present).
- **Markdown-only** graph (zero functions): all `Contains` edges (heading trees + file structure).
- **Mixed code + docs:** `Contains` only for **heading → heading** (nested doc sections), not file→class/code containment.

`rgctl -f json metrics --pagerank` uses the same behavioral edge set — markdown-only corpora converge with **non-zero** PageRank (fixture top ~0.04; k8s smaller per-node scores at ~17k headings).

For navigation, heading `CONTAINS` trees and targeted GQL are still usually clearer than global PageRank on mixed code+doc graphs.

## `.mdx`

Registered under language id `markdown` (extensions `md` + `mdx`). MDX/JSX in code fences is not executed; only tree-sitter-md structure is indexed.

## CFG, PDG, slice, inspect, CPG flows

Markdown has **no CFG grammar**. `discover --with-cfg` skips `.md` / `.mdx` files in the CFG batch. Commands that need a function CFG (`slice`, `inspect`, `cpg flows`) **reject** markup paths with an error pointing here.

## Dashboard

The graph view defaults to **Function + Class**. Enable **Module (incl. doc headings)** in the sidebar filter, or click **Code + doc headings**, to see documentation nodes after drill-down. Search tab remains function-only (semantic API).

## Demo video

Record a short CLI walkthrough: `docs/videos/record-markdown-context-cli.sh` (VHS tape: `docs/videos/markdown-context-cli.tape`).

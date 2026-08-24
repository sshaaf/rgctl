# Markdown Context Graph

## Introduction

rgBuilder indexes `.md` and `.mdx` files into a **documentation context graph** alongside your code. Headings become navigable sections, internal links become `REFERENCES` edges, code fences become searchable blocks, and frontmatter keys become queryable variables — all in the same `graph.snapshot.bin` that powers `gql`, `metrics`, and `export`.

This guide walks through `discover`, GQL, Obsidian/OKF export, and doc-scoped semantic search. The primary example is the English docs from [kubernetes/website](https://github.com/kubernetes/website) (`content/en`, on the order of **17k heading sections**). A small in-tree fixture covers every markdown construct and doc→code linking.

## Use Cases

- **Large documentation sites.** Index an entire docs tree as heading modules with `CONTAINS` hierarchy and `REFERENCES` cross-links.
- **Obsidian vault browsing.** Export one note per heading section, with folder layout mirroring doc paths and wikilinks from internal links.
- **Agent-first doc navigation.** Query heading structure, cross-links, and community membership with GQL instead of reading every file.
- **Natural-language section search.** Build a doc-scoped semantic index over heading bodies and code blocks (`semantic index --scope docs`).
- **Doc + code linking.** Discover markdown and Java (or other languages) together; walk doc → file → class in one query.
- **Architecture decision records.** Index ADRs with heading hierarchy, file links, and section-level `REFERENCES` edges.

## Example Projects

This guide uses **two checkouts** — each for a different part of the walkthrough, not as alternatives to the same task.

1. **[kubernetes/website](https://github.com/kubernetes/website)** (`example/kubernetes-website/`) — **Steps 1–4:** `discover`, Obsidian export, doc semantic search, communities, and OKF on a real documentation site. Clone the repo (sparse `content/en` is enough) before you start.

2. **markdown-context fixture** (`tests/fixtures/markdown-context/`) — **Step 5 and the construct showcase:** every supported markdown syntax, plus `CheckoutService.java` for doc→code GQL. Always in-tree; no clone.

```bash
# kubernetes/website — sparse clone of English docs only
mkdir -p example
git clone --depth 1 --filter=blob:none --sparse https://github.com/kubernetes/website.git example/kubernetes-website
(cd example/kubernetes-website && git sparse-checkout set content/en)

export REPO="$(pwd)/example/kubernetes-website/content/en"
export REPO_FIXTURE="$(pwd)/tests/fixtures/markdown-context"
```

**Prerequisites:** `rg-build` on your `PATH`. For large exports (17k+ Obsidian notes), use a **release** binary — [download the latest release](https://github.com/sshaaf/rgBuilder/releases) or [build from source](../user-guide.md#1-installation) (`cargo build --release`). See [Installation](../user-guide.md#1-installation) in the User Guide.

## What rgBuilder indexes

rgBuilder parses markdown with official `tree-sitter-md` (block + inline grammars). The table below maps author syntax to graph nodes and edges.

| Author writes | Graph | Properties / edges |
|---------------|-------|-------------------|
| `# Heading` / `## …` (ATX) | `:Module` | `kind=heading`, `level`, `slug`, QN `{path}#{slug}` |
| Setext heading (`===` / `---` underline) | `:Module` | Same as ATX (`kind=heading`) |
| Nested headings | `:Module` + `CONTAINS` | Parent heading → child heading |
| Section prose under a heading | on heading node | `body_text`, `body_hash`; large → `body_ref` |
| `` ```lang … ``` `` fenced code | `:Module` | `kind=code_block`, `language`, `body_text` |
| Indented code (4 spaces) | `:Module` | `kind=code_block` (no language) |
| `[text](./file.md)` file link | `:Import` + `REFERENCES` | Edge to `:File`; `to_type_hint=file` |
| `[text](./file.md#section)` heading link | `:Import` + `REFERENCES` | Edge to `:Module` heading; `to_type_hint=module` |
| `[jump](#slug)` same-file fragment | `:Import` + `REFERENCES` | Edge to heading in same file |
| Reference-style links (`[x][ref]` + `[ref]: url`) | `:Import` + `REFERENCES` | Full, collapsed, and shortcut forms |
| `https://…` / `mailto:` links | `:Import` only | Link symbol with `url`; **no** `REFERENCES` edge |
| `![alt](img.png)` images | — | Not indexed |
| `[[wikilink]]` | — | Not indexed |
| YAML frontmatter (`---`) | `:Variable` | `kind=frontmatter`, flattened keys (`metadata.author`), `value` |
| TOML frontmatter (`+++`) | `:Variable` | Same flattening as YAML |
| Links inside table cells | `:Import` + `REFERENCES` | Parsed from `pipe_table_cell` inline trees |
| `.mdx` files | same as `.md` | Structure only — JSX in fences is not executed |

**Qualified names:** `{file_path}#{slug}` where `slug` is ASCII-slugified heading text. Duplicate titles in one file get `-2`, `-3`, … suffixes.

**Fragments are literal:** `[link](./adr.md#payments)` targets `adr.md#payments`, not a slugified variant. Prefer slug fragments (`#checkout-flow`) over visible titles (`#Checkout Flow`).

For the full node model and GQL catalog, see [markdown-context.md](../markdown-context.md).

## Step-by-Step

### 1. Discover documentation (kubernetes/website)

If you built from source, ensure `target/release/rg-build` is on your `PATH` (see [Add rgBuilder to your PATH](../user-guide.md#2-add-rgbuilder-to-your-path)).

```bash
export REPO="$(pwd)/example/kubernetes-website/content/en"

rg-build -r "$REPO" discover . -l markdown
```

**Output:**

```
[>] rg-build discover
[✓] rg-build discover finished in 1.6s
```

**What happened:**

- rgBuilder parsed the sparse checkout of kubernetes/website `content/en` — thousands of `.md` files, **17,244 heading modules** (`:Module` with `kind=heading`), zero `:Function` nodes.
- The graph snapshot was written to `$REPO/.rgbuilder/graph.snapshot.bin`.
- Section bodies larger than 32 KiB inline were stored in `$REPO/.rgbuilder/content_store.bin` (Blake3-keyed).

Confirm the heading count (read `"count"` from the JSON envelope — property projection in `RETURN` is not supported):

```bash
rg-build -r "$REPO" -f json gql \
  "MATCH (n:Module) WHERE n.kind = 'heading' RETURN n LIMIT 1"
```

The `"count"` field reflects the full match set (17,244 at time of writing; drifts with upstream). Add `LIMIT` only when you want sample rows in `rows`.

**Fixture equivalent** (same command, tiny graph — useful before iterating on GQL):

```bash
export REPO="$REPO_FIXTURE"
rg-build -r "$REPO" discover . -l markdown
```

Markdown is included in default `discover`. Use `-l markdown` when you want **only** documentation.

### 2. Export to Obsidian

Turn every heading section into an interlinked vault. Export reads the graph and `content_store.bin` — it does not re-parse source files.

```bash
export REPO="$(pwd)/example/kubernetes-website/content/en"

rg-build -r "$REPO" export \
  --export-format obsidian \
  --export-output "$REPO/vault" \
  --query all
```

**Output:**

```
[>] rg-build export
Exported 17244 notes (2971 wikilinks) -> …/example/kubernetes-website/content/en/vault
[✓] rg-build export finished in 7.0s
```

**What happened:**

- rgBuilder exported **17,244 notes** — one per heading module. Note count equals heading module count.
- Folder layout mirrors doc paths (e.g. `docs/concepts/…/feature.md#overview` → nested vault paths). Long blog titles get truncated slugs with a stable hash suffix.
- Outgoing `REFERENCES` edges became **2,971 wikilinks** (`[[path]]`, no `.md` suffix).
- Each note's YAML frontmatter includes `qualified_name` and `level` for trace-back to GQL.

**Open the vault:** Obsidian → **Open folder as vault** → select `$REPO/vault`.

**Fixture equivalent:**

```bash
export REPO="$REPO_FIXTURE"
rg-build -r "$REPO" export --export-format obsidian --export-output "$REPO/vault" --query all
```

```
Exported 16 notes (7 wikilinks) -> …/markdown-context/vault
```

Re-export after doc edits: `discover`, then `export`. Obsidian export is read-only — vault edits are not synced back to the graph.

### 3. Query, communities, and semantic search

**Cross-links across the corpus:**

```bash
export REPO="$(pwd)/example/kubernetes-website/content/en"

rg-build -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:REFERENCES]->(t) WHERE h.kind = 'heading' RETURN h, t LIMIT 20"
```

**Communities (GQL)** — community assignment is computed at `discover` and exposed through GQL as a virtual overlay (not stored in `graph.snapshot.bin`). List communities, then filter heading modules by `community_id`:

```bash
rg-build -r "$REPO" -f json gql --macro-name all_communities unused

# Pick a community_id from the output (example below — yours will differ)
rg-build -r "$REPO" -f json gql \
  "MATCH (n:Module) WHERE n.kind = 'heading' AND n.community_id = '60994' RETURN n LIMIT 20"
```

On markdown-only graphs, community detection uses `REFERENCES` cross-links and heading `CONTAINS` trees. See [Graph Query Language](graph-query-language.md) and [Community Detection](community-detection.md).

**PageRank (`metrics`, not GQL)** — centrality scores are not GQL node properties. Use the metrics command after `discover`:

```bash
rg-build -r "$REPO" -f json metrics --pagerank
```

The `top` array lists node UUIDs and scores. For named hotspots, combine with GQL on structure (`REFERENCES`, `CONTAINS`) or communities above.

**Doc-scoped semantic index** (separate from `discover`; uses `semantic_index.bin`):

```bash
rg-build -r "$REPO" semantic index --scope docs --embedder hash
```

**Output:**

```
[>] rg-build semantic index
Indexed 24608 functions (sign-hash-v1, 256 dims) → …/kubernetes-website/content/en/.rgbuilder/semantic_index.bin
  incremental: 0 reused, 24608 embedded, 0 removed
[✓] rg-build semantic index finished in 2.0s
```

The CLI still prints `functions` — the count is **index entries** (heading + code-block modules when `--scope docs`).

```bash
rg-build -r "$REPO" -f json semantic query "pod scheduling" --scope docs --limit 10
```

**What happened:**

- `--scope docs` embedded `:Module` nodes with `kind=heading` and `kind=code_block`.
- Embeddings use inline `body_text` or full UTF-8 from `content_store.bin` when `body_ref` is set.
- Query `--scope docs` does not filter hits — build the index with the scope you need. Re-run `semantic index` after large doc edits.

### 4. Export OKF JSON

```bash
export REPO="$(pwd)/example/kubernetes-website/content/en"

rg-build -r "$REPO" export \
  --export-format okf \
  --export-output "$REPO/okf.json" \
  --query all
```

**Output:**

```
[>] rg-build export
Exported 17244 OKF entities -> …/example/kubernetes-website/content/en/okf.json
[✓] rg-build export finished in 466ms
```

Use `--query all` for doc exports (filter queries target code-graph subsets). The fixture run is identical with `REPO="$REPO_FIXTURE"` (16 entities).

### 5. Feature corpus (markdown-context fixture)

Switch to the in-tree fixture for **every markdown construct** and **doc→code** queries. Layout:

```text
markdown-context/
  README.md              ← YAML frontmatter
  docs/
    guide.md             ← headings, links, fenced code
    adr.md               ← tables, cross-links
    overview.mdx         ← `.mdx`
  src/
    CheckoutService.java ← doc→class anchor
```

```bash
export REPO="$REPO_FIXTURE"
rg-build -r "$REPO" discover . -l markdown,java   # docs + code
```

**GQL on the fixture** (`LIKE` = prefix/suffix globs only):

```bash
# Checkout-related headings
rg-build -r "$REPO" -f json gql \
  "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n LIMIT 10"

# Heading tree
rg-build -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:CONTAINS*1..3]->(n:Module) \
   WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND n.kind = 'heading' \
   RETURN h, n"

# Cross-doc link (guide → payments ADR)
rg-build -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:REFERENCES]->(t:Module) \
   WHERE h.kind = 'heading' AND t.name = 'Payments' RETURN h, t"

# Doc → Java class (needs markdown,java discover)
rg-build -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) \
   WHERE h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' \
   RETURN h, f, c"

# Section prose — compact GQL returns bindings; use Obsidian export or semantic query for full text
rg-build -r "$REPO" -f json gql \
  "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name = 'Checkout Flow' RETURN n LIMIT 1"
```

## Markdown showcase (fixture)

After `discover` on `$REPO_FIXTURE`, these checks confirm each supported construct.

### Headings and hierarchy

`docs/guide.md` — ATX headings (`# Checkout Flow`, `## Cart`, `### Validation rules`). Nested `CONTAINS`: Checkout Flow → Cart → Validation rules.

```bash
rg-build -r "$REPO_FIXTURE" -f json gql \
  "MATCH (a:Module)-[:CONTAINS]->(b:Module) \
   WHERE a.kind = 'heading' AND b.kind = 'heading' RETURN a, b LIMIT 20"
```

### Internal links

`docs/guide.md#checkout-flow` links to `./adr.md#payments`, `./adr.md`, and `../src/CheckoutService.java`. External URLs (Stripe API) get link symbols but no `REFERENCES` edge.

```bash
rg-build -r "$REPO_FIXTURE" -f json gql \
  "MATCH (h:Module)-[:REFERENCES]->(t) \
   WHERE h.qualified_name LIKE '*#checkout-flow' RETURN h, t"
```

### Fenced and indented code

````markdown
```java
cart.validate();
```
````

```bash
rg-build -r "$REPO_FIXTURE" -f json gql \
  "MATCH (n:Module) WHERE n.kind = 'code_block' RETURN n LIMIT 5"
```

### Frontmatter, tables, MDX

- `README.md` — YAML frontmatter → `:Variable` with flattened keys (`metadata.author`)
- `docs/adr.md` — pipe table with internal link in a cell
- `docs/overview.mdx` — same plugin as `.md`; JSX in fences is not executed

```bash
rg-build -r "$REPO_FIXTURE" -f json gql \
  "MATCH (v:Variable) WHERE v.kind = 'frontmatter' RETURN v LIMIT 10"
```

## Author linking cheat sheet

| Author writes | Resolves to | Indexed? |
|---------------|-------------|----------|
| `./adr.md` | File `docs/adr.md` | Yes — `REFERENCES` → `:File` |
| `./adr.md#payments` | Module `docs/adr.md#payments` | Yes — `REFERENCES` → `:Module` |
| `#checkout-flow` | Same-file heading slug | Yes |
| `#Checkout Flow` | Literal fragment (often a stub) | Avoid — use slug |
| `../src/Foo.java` | File node | Yes (if file is in discover set) |
| `https://…` | — | Link symbol only; no edge |
| `![alt](img.png)` | — | Ignored |
| `[[Wiki]]` | — | Ignored |

If a linked file is not in the discover set, the `REFERENCES` edge is dropped.

## What is not supported

| Feature | Notes |
|---------|-------|
| CFG / PDG / `slice` / `inspect` / `cpg flows` on `.md` | No CFG grammar; commands reject markup paths |
| MDX component execution | Structure only |
| Images and wikilinks | Skipped |
| External URL graph edges | `https://` not `REFERENCES` |
| `blast-radius` on doc nodes | Calls-only; use GQL for docs |
| Obsidian → source sync | Export is one-way |

## Profile gates (maintainers)

Cold discover and warm Obsidian export on kubernetes/website are gated in ignored tests (`example/k8s-website` in the repo's internal profile layout). These use **ceiling** baselines (not typical laptop timings):

```bash
cargo build --release --bin rg-build
cargo test --release --test cold_profile_gates k8s_website_markdown_cold_discover_within_baseline -- --ignored --nocapture
cargo test --release --test cold_profile_gates k8s_website_obsidian_export_to_vault -- --ignored --nocapture
```

Ceilings: cold discover 3.0s wall, warm Obsidian export 30.0s wall (+10% tolerance). Details: [markdown-context.md § Cold profile](../markdown-context.md#cold-profile-kuberneteswebsite).

## Export formats for documentation

| Format | Output | Best for |
|--------|--------|----------|
| `obsidian` | Directory of `.md` notes | Human browsing in Obsidian |
| `okf` | Single JSON entity bundle | OKF / knowledge-platform tooling |

Code-graph formats (`json`, `graphml`, `graphviz`, `mermaid`) also include doc nodes when present. See [Exporting Graphs](exporting-graphs.md).

## Benefits

- **Real-site corpus.** kubernetes/website `content/en` is the profile fixture — 17k+ heading modules with real cross-links.
- **One graph for docs and code.** ADRs, guides, and services share `graph.snapshot.bin` when discovered together.
- **Low-token agent queries.** GQL returns compact bindings (name, qualified_name, file) without opening whole files.
- **Human-friendly export.** Obsidian vaults mirror heading hierarchy with wikilinks.
- **Large-corpus bodies.** `content_store.bin` holds section text beyond the 32 KiB inline cap.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) — `discover` flags and artifacts
- [Graph Query Language](graph-query-language.md) — GQL syntax and macros
- [Exporting Graphs](exporting-graphs.md) — JSON, GraphML, Mermaid, and filter queries
- [Semantic Search](semantic-search.md) — function and doc-scoped NL search
- [Community Detection](community-detection.md) — how communities are detected and named
- [Markdown context reference](../markdown-context.md) — node model, cold profiles, dashboard filter
- [Agent recipes](../agent-recipes.md) — copy-paste doc queries for LLM agents

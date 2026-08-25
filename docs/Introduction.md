# Introduction to rgctl

**What rgctl is** and how a **code knowledge graph** works — before you run commands.

**Hands-on:** [User Guide](user-guide.md) (ecommerce-java). **Agents:** [AGENTS.md](../AGENTS.md). **JSON:** [json-api.md](json-api.md).

---

## What problem does rgctl solve?

Modern codebases are too large to hold in your head. Changing a function raises reachability questions: who calls it, what depends on it, are security-sensitive paths involved, where is complexity concentrated?

**rgctl turns the repository into a structured graph** — functions, types, calls, imports, and more — so you ask structural questions and get deterministic answers instead of grepping and guessing. Built in **Rust** for speed and predictable memory on large repos.

---

## What is a code knowledge graph?

| Everyday idea | In rgctl |
|---------------|--------------|
| Places on the map | **Nodes** — functions, classes, files, modules, … |
| Roads | **Edges** — typed relations (`CALLS`, `CONTAINS`, `IMPORTS`, …) |
| The map file | Artifacts under **`.rgctl/`** after `discover` |

**Reachability** (who can reach whom along call paths) is pre-computed and stored compactly — that is why **blast-radius** stays fast on large graphs.

You do not need graph theory to use the CLI: **indexing builds the map; commands query the map.**

---

## How the pieces fit

```text
  Your repo
      │
      ▼
  discover          →  .rgctl/  (snapshot, indexes, optional archives)
      │
      ├── gql / blast-radius / metrics / cpg / slice / check   (−f json for agents)
      ├── semantic index + query   (opt-in)
      └── serve                    (optional HTTP UI + /api/query)
```

1. **Once** (or after large changes): `discover`.  
2. **Many times:** query commands against `.rgctl/`.  
3. **Agents:** always prefer `-f json` ([AGENTS.md](../AGENTS.md)).  
4. **Dashboard:** optional visual UI after `--with-dashboard` — not required for structural answers.

Capability designs for contributors: [design/](design/README.md).

---

## Capability map (concepts only)

Commands and sample output live in the **[User Guide](user-guide.md)**. Short intent:

| Capability | Intent |
|------------|--------|
| **discover** | Index repo → graph + analytics caches |
| **gql** | Exact inventory and relation queries |
| **blast-radius** | Upstream impact / reachability for a symbol |
| **slice / taint** | Statement-level data/control dependence; source→sink |
| **inspect** | CFG / PDG / dominance for one function |
| **cpg** | Hybrid CALL + CFG/PDG façade (mutations, flows) |
| **metrics / communities** | PageRank, betweenness, label-propagation clusters |
| **semantic** | Opt-in natural-language / keyword search over functions |
| **export / check** | Subgraph export; CI policy on blast-radius |
| **migration hints** | Package roadmap JSON (`--export-migration-hints`) |
| **serve** | Optional HTTP API (+ dashboard if assets present) |

**Markdown / docs:** `discover` indexes `.md` and `.mdx` by default (headings, links, frontmatter). GQL on `:Module` (`kind=heading`) and `REFERENCES`; semantic search stays function-only. See [markdown-context.md](markdown-context.md).

Languages: [languages.md](languages.md). Research: [further-reading.md](further-reading.md).

---

## Where to go next

| You want… | Go to |
|-----------|--------|
| Install and run every CLI command | [User Guide](user-guide.md) |
| Agent recipes | [AGENTS.md](../AGENTS.md) · [agent-recipes.md](agent-recipes.md) |
| JSON fields | [json-api.md](json-api.md) |
| Markdown / doc graph | [markdown-context.md](markdown-context.md) |
| Contribute / internals | [docs hub — For contributors](README.md#for-contributors) |

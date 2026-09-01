# rgctl for AI agents

rgctl is designed so agents answer **structural questions** from a pre-built graph instead of reading whole files into context.

**Installation:** [docs/installation.md](docs/installation.md) (prerequisites, setup)  
**Full JSON reference:** [docs/json-api.md](docs/json-api.md) (also on the site: [sshaaf.github.io/rgctl/docs/json-api/](https://sshaaf.github.io/rgctl/docs/json-api/))  
**Copy-paste recipes:** [docs/agent-recipes.md](docs/agent-recipes.md)  
**Human walkthrough:** [docs/user-guide.md](docs/user-guide.md)  
**Docs hub:** [docs/README.md](docs/README.md) · [site docs](https://sshaaf.github.io/rgctl/docs/)

Default for agents: spawn **`rgctl -f json`** subprocesses (or use foreground **`rgctl serve`** for repeated HTTP queries). Do **not** open the browser dashboard unless the user asks for a visual UI.

Install the project skill once (Claude Code + Cursor dirs under the repo):

```bash
rgctl -r "$REPO" install --skill
```

---

## Agent workflow

```text
1. cd "$REPO" && rgctl discover .     # or rgctl -r PATH discover (no trailing . with -r)
2. rgctl -f json <command>            # compact facts on stdout
3. Parse schema_version + payload     # never scrape stderr for JSON
```

Artifacts live at **`{repo}/.rgctl/`**. Set `REPO` to the repository root:

```bash
export REPO=/path/to/repo
rgctl -r "$REPO" -f json gql 'MATCH (n:Function) RETURN n LIMIT 20'
```

Upgrading from an old daemon install: `rgctl migrate-cache` copies `~/.rgctl/cache/{name}/.rgctl/` into the repo (see [installation.md](docs/installation.md)).

---

## High-value commands (low token cost)

| Intent | Command |
|--------|---------|
| Full session (graph + CFG + dashboard + semantic) | `rgctl discover PATH --full` (queryable after stage 1; status in `.rgctl/pipeline_status.json`) |
| HTTP session (auto-pipeline) | `rgctl serve` — `GET /api/status`; `--no-pipeline` restores fail-fast |
| Inventory functions | `rgctl -f json gql --macro-name all_functions unused` |
| List communities | `rgctl -f json gql --macro-name all_communities unused` |
| Find symbol by pattern | `rgctl -f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service*' RETURN n LIMIT 20"` |
| Find by FQN (not `n.name`) | `rgctl -f json gql "MATCH (n:Class) WHERE n.qualified_name = 'com.example.Foo' RETURN n"` |
| Community members | `rgctl -f json gql "MATCH (f:Function) WHERE f.community_id = '12' RETURN f LIMIT 20"` |
| Natural-language function search | `rgctl semantic index` then `rgctl -f json semantic query "checkout flow" --limit 10` |
| Community semantic search | `rgctl -f json semantic query "checkout" --scope community --limit 10` |
| Impact before editing | `rgctl -f json blast-radius <Symbol> [--depth N]` |
| Architectural hotspots | `rgctl -f json metrics --pagerank` |
| Call neighborhood | `rgctl -f json gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) RETURN a,b LIMIT 50"` |
| Doc headings / cross-links | `discover` indexes `.md` / `.mdx` by default; GQL on `:Module` with `kind=heading` and `REFERENCES` — see [markdown-context.md](docs/markdown-context.md) |
| Obsidian vault from docs | `rgctl -r "$REPO" discover -l markdown` then `export --export-format obsidian --export-output "$REPO/vault" --query all` — see [markdown-context.md](docs/markdown-context.md#obsidian-vault-export) |
| Doc section semantic search | `rgctl semantic index --scope docs --embedder hash` then `rgctl -f json semantic query "checkout flow" --scope docs --limit 10` (query scope does not filter — index must be doc-scoped) |
| Hybrid CPG status / CALL / PDG / slice | `rgctl -f json cpg status` then `cpg function\|calls\|pdg\|slice` (needs `discover --with-cfg` for PDG/slice) |
| Field mutations (cart / DTO safety) | `rgctl -f json cpg mutations --type ShoppingCart --exclude-ctors` (ecommerce CoolStore; or any type name; needs `--with-cfg`) |
| Data flows / slice (CPG) | `rgctl -f json cpg flows FILE --line N --variable V --function F [--direction forward\|backward] [--with-alias]` |
| Loop-carried DFG tags | `rgctl discover . --with-cfg --with-dfg-loops` (tags `DataDependency.loop_carried` in PDG) |
| AST skeleton | `rgctl discover --with-ast-skeleton` then `rgctl -f json cpg ast <Symbol>` |
| CPG export | `rgctl cpg export --format graphson --output cpg.json [--path-contains src/]` |
| Migration plan | `rgctl discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints` then read `.rgctl/migration_plan.json` (or dashboard copy) |
| CI gate on changes | `rgctl -f json check --policy-file policy.json` (exit 1 = violations) |
| Kantra migration rules | `rgctl discover . --with-kantra` (embedded Konveyor catalog; `.rgctl/kantra_findings.json`) |
| Kantra target filter | `rgctl discover . --with-kantra --kantra-target quarkus` |
| Kantra rules inventory (GQL) | `rgctl -f json gql "MATCH (r:KantraRule) RETURN r LIMIT 20"` (after `--with-kantra` index) |
| Kantra violations → code nodes | `rgctl -f json gql "MATCH (r:KantraRule)-[:VIOLATES]->(n) RETURN r, n LIMIT 20"` (after full eval, not `--kantra-index-only`) |
| Kantra rules by Konveyor target | `rgctl -f json gql` on `:KantraRule` with `` r.`konveyor.io/target` `` property filter — [user guide](docs/user-guide.md#kantra-migration-rules---with-kantra) |
| Kantra fixture override (CI) | `rgctl discover . --with-kantra --kantra-rules tests/fixtures/kantra-rules` |

---

## Repeated queries in one session

**Option A — CLI subprocess (default for agents):**

```bash
export REPO=/path/to/repo
rgctl -r "$REPO" -f json gql 'MATCH (n:Function) RETURN n LIMIT 5'
rgctl -r "$REPO" -f json blast-radius ShoppingCartService
```

**Option B — HTTP (one long-lived process):**

```bash
rgctl -r "$REPO" serve --open
# POST http://127.0.0.1:8080/api/query  {"query":"MATCH (n:Function) RETURN n LIMIT 5"}
```

See [docs/http-api.md](docs/http-api.md).

---

## Rules of thumb

0. **Artifacts** — always `{repo}/.rgctl/` after `discover`. Add `.rgctl/` to `.gitignore`.
1. **Index first** — `gql`, `blast-radius`, `metrics` fail without `discover`.
2. **Discover target** — `cd repo && rgctl discover .` or `rgctl -r PATH discover` (no trailing `.` with `-r`; `discover .` uses cwd, not `-r`).
3. **Use `-f json`** — stable `schema_version` fields; see [json-api.md](docs/json-api.md).
4. **`inspect` takes a symbol only** — no `--class` (use `blast-radius` for disambiguation).
5. **`slice --function`** is the **method/function name**, not the class name.
6. **`export --query`** uses filter syntax (`name:Foo`, `type:Function`, `all`) — not full GQL `MATCH`. Obsidian/OKF export use `--query all` (full heading set).
7. **Deep analysis** needs `discover --with-cfg` (and `--with-taint` for discover-time taint) (slice, inspect, taint).
8. **Semantic search** needs `semantic index` (separate from discover). Default is **vocab** (compiled token table, no ONNX). Optional **code-daemon** (`--embedder code-daemon`, Git LFS weights) or `--embedder hash`. `--embed-bodies` re-reads function source (off by default). Optional `semantic distill --matrix PATH` writes an RBVK matrix from **our** token list through a teacher (not `vocab`); copy to `assets/vocab_matrix.bin` and rebuild for `vocab-accumulate-v2`. Doc sections: `semantic index --scope docs` (embeds headings + code blocks); query `--scope docs` does not filter hits — only index scope matters (`community` is the exception). Fusion is on by default (`--no-fusion` to disable).
9. **Profile discover** — `discover -v` with `RUST_LOG=profile=info` for `[profile] stage` and centrality sub-phase timings (see [analysis-architecture.md](docs/analysis-architecture.md)). **Cold profile** (accurate perf): delete `.rgctl/`, build release `rgctl`, then run ignored gates — warm/partial caches skew timings. `cargo build --release --bin rgctl` then `cargo test --release --test cold_profile_gates -- --ignored --nocapture`. Linux: `linux_cold_discover_within_baseline` on `example/linux` (baseline **~145 s**). metasfresh: `metasfresh_cold_discover_within_baseline` with `--full` (baseline **~74 s**). Markdown: `./scripts/fetch-profile-repos.sh` then `k8s_website_markdown_cold_discover_within_baseline` on `example/k8s-website` (baseline ~3s, `-l markdown`). See [docs/internal/profile.md](docs/internal/profile.md) and `example/README.md`.
10. **Dashboard is optional** — only with `--with-dashboard` / `serve` when a human wants a UI; never required for structural answers.
11. **Markdown docs** — `.md` / `.mdx` are indexed on `discover` (headings, links, frontmatter). Use GQL for doc navigation; `semantic index --scope docs` for NL section search; Obsidian export for human vault browsing; `slice` / `inspect` / `cpg flows` reject markup paths. See [markdown-context.md](docs/markdown-context.md).
---

## On-disk artifacts for agents

After `discover`, artifacts live under **`{repo}/.rgctl/`**:

| Path | Content |
|------|---------|
| `graph.snapshot.bin` | Graph snapshot |
| `content_store.bin` | Large markdown bodies / files (Blake3-keyed; used by Obsidian export + doc semantic index) |
| `dashboard/manifest.json` | Counts, feature flags |
| `dashboard/migration_plan.json` | Migration export (with `--with-dashboard` and/or `--export-migration-hints`) |
| `dashboard/graph_payload.bin` | Columnar graph for dashboard WASM |
| `semantic_index.bin` | Opt-in semantic search index (`semantic index`) |

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Policy violation (`check`, `blast-radius --policy-file`) or command error |

---

## See also

- [Introduction](docs/Introduction.md) — concepts
- [User Guide](docs/user-guide.md) — full CLI
- [Integration test matrix](docs/internal/integration-tests.md) — CI harness
- [Markdown context graph](docs/markdown-context.md) — `.md` / `.mdx` indexing and GQL
- [Further reading](docs/further-reading.md) — research map and contribution ideas

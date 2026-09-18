# Agent instructions for rgctl

## Summary

`rgctl` is a high-performance Rust code knowledge graph for LLM agents: tree-sitter extraction, typed relations, mmap snapshots, blast-radius / communities / CPG, and JSON-first CLI (`-f json`).

**Your goal when contributing here:** preserve ingest scale, query correctness, memory discipline, and deterministic artifacts under `.rgctl/` — not add convenience at the cost of Tokio blocking, whole-repo clones, or ungated cold regressions.

> **Looking for how to *use* rgctl on another codebase?** Install skills (`rgctl install --skill --with-commands`) or copy [docs/agents/USER_AGENTS_TEMPLATE.md](docs/agents/USER_AGENTS_TEMPLATE.md) into *that* repo’s `AGENTS.md`. See [docs/guides/agent-commands.md](docs/guides/agent-commands.md).

---

## Must-follow rules

- **Async vs CPU:** Discover/serve use `tokio`. Do **not** run heavy CPU (parse, graph analytics, CFG) on the async executor — use `spawn_blocking` / Rayon where the pipeline already does.
- **Parallel ingest:** Per-file plugin extraction runs on the discover worker pool. Do not replace with a serial whole-repo walk when parallel ingest exists.
- **Streaming commits:** Emit symbols/relations file-by-file; avoid unbounded `Vec<Relation>` / whole-repo ASTs before commit (`rgctl-extraction` spill patterns).
- **Clone hygiene:** Prefer `&[u8]` / `Cow` / borrows in tree-sitter walkers; `Vec::with_capacity` when sizes are known; no `unwrap()` in library paths.
- **Typed graph:** Respect `EdgeType` / node kinds; do not invent ad-hoc string edges for hot paths.
- **Artifacts:** Session data lives in `{repo}/.rgctl/`. Warm caches invalidate wall-time claims.
- **Features:** Default semantic embedder is compiled **vocab**. Do not require ONNX / Python ML unless behind an explicit feature (e.g. `semantic-onnx` / code-daemon + Git LFS).
- **OpenSpec language work:** Still cite [openspec/changes/_shared/starting-context.md](openspec/changes/_shared/starting-context.md) (pointer here); follow the sections below.

---

## Context & architecture

- **Discover** walks the tree, runs language plugins (tree-sitter), builds the graph, writes compact caches to `.rgctl/`.
- **Query** paths are read-oriented and return versioned JSON (`schema_version` on stdout — never scrape stderr).
- **Analysis** (`rgctl-analysis`) projects CSR / callgraph / centrality / blast-radius / CFG–PDG; see [docs/analysis-architecture.md](docs/analysis-architecture.md).
- **Languages:** `crates/rgctl-lang-*` + `rgctl-plugin-api`; register in `languages.toml`.

---

## Starting context & performance policy

Applies to all extraction / language / discover hot-path work (and OpenSpec `*-extraction-depth` / `add-*-language-support` changes).

### Implementation model

1. **Async** — existing `tokio` orchestration; offload CPU-heavy work.
2. **Parallel** — discover file pool (`rayon` / workers).
3. **Streaming** — incremental graph commit; match extraction spill/channel patterns.
4. **Idiomatic Rust** — `Result` + `thiserror`; follow `rgctl-lang-java` / `rgctl-extraction` conventions.

### Cold profile (mandatory for scale / perf claims)

1. **Release binary only:** `cargo build --release --bin rgctl`
2. **Delete artifacts:** `rm -rf <corpus>/.rgctl/`
3. **Run from inside the corpus** (`cd example/<corpus> && rgctl discover . -v`) — positional `.` sets session root; `-r` is ignored when `.` is passed.
4. **Logging:** `RUST_LOG=info,profile=info`

Deep stage timings and reference machine notes: [docs/internal/profile.md](docs/internal/profile.md) · corpora: [example/README.md](example/README.md).

### Gate A — cross-language regression

| Corpus | Test gate | Discover | Baseline (ref M3 Pro, +10%) |
|--------|-----------|----------|------------------------------|
| Linux kernel | `linux_cold_discover_within_baseline` | default | **145 s** wall |

```bash
cargo build --release --bin rgctl
cargo test --release --test cold_profile_gates linux_cold_discover_within_baseline -- --ignored --nocapture
```

### Gate B — language-scale (~10k source files)

Language changes add (or document) a language-filtered cold discover on a ~10k-file corpus. Record `wall_secs`, `nodes`, `functions`, `index_graph_build` from `[profile] discover summary`; add a gate in `tests/cold_profile_gates.rs` once baselined (+10%).

Fetch: `./scripts/fetch-profile-repos.sh`

| Language | Corpus | Path | Discover | Env override |
|----------|--------|------|----------|--------------|
| **C** | Linux | `example/linux` | default | `RGCTL_LINUX_REPO` |
| **C++** | LLVM | `example/llvm-project` | `-l cpp` on `clang/` | `RGCTL_LLVM_REPO` |
| **C#** | Roslyn | `example/roslyn` | `-l csharp` on `src/` | `RGCTL_ROSLYN_REPO` |
| **Go** | Kubernetes | `example/kubernetes` | `-l go` on `pkg/` `cmd/` | — |
| **Java** | metasfresh | `example/metasfresh-4.9.8b` | `--full` | `METASFRESH_REPO` |
| **JavaScript** | Node.js | `example/node` | `-l javascript` on `test/` | `RGCTL_NODE_REPO` |
| **PHP** | Magento 2 | `example/magento2` | `-l php` | `RGCTL_MAGENTO2_REPO` |
| **Python** | Home Assistant | `example/home-assistant` | `-l python` | `RGCTL_HOME_ASSISTANT_REPO` |
| **Ruby** | Discourse | `example/discourse` | `-l ruby` | — |
| **Rust** | rustc | `example/rust` | `-l rust` | `RGCTL_RUST_REPO` |
| **TypeScript** | VS Code | `example/vscode` | `-l typescript` on `src/` | `RGCTL_VSCODE_REPO` |

File counts are approximate (goal **O(10⁴)** sources). Exclude `vendor/`, `node_modules/`, `target/`, `third_party/`.

---

## Profiles, tests, and benches

### Cargo profiles

| Profile | When |
|---------|------|
| default / `dev` | Iterate, unit tests |
| `--release` | Discover wall times, cold gates, any published timing |
| `cargo bench` (`[profile.bench]`) | Criterion microbenchmarks |

### Tests (run what you touched)

| Kind | Command | Practice |
|------|---------|----------|
| Workspace | `cargo test` | Default before merge for touched crates |
| Release CLI goldens | `cargo test --release --test subprocess_golden_path` (and related) | CLI surface changes |
| Cold profile gates | `cargo test --release --test cold_profile_gates -- --ignored --nocapture --test-threads=1` | Perf / extraction / ingest; **Gate A** for scale-sensitive work |
| Dashboard / lang | `dashboard_*`, langfeature / ecommerce fixture tests | When that path changes |
| Corpora | `./scripts/fetch-profile-repos.sh` | Before ignored gates needing `example/` |

Warm or partial `.rgctl/` **invalidates** cold timings.

### Benches

| Target | Command |
|--------|---------|
| Workspace | `cargo bench` — `parsing`, `graph`, `graph_benchmarks`, `analysis_benchmarks`, `centrality_benchmarks`, `community_benchmarks`, `blast_radius_benchmarks` |
| Snapshot diff | `cargo bench -p rgctl-graph --bench snapshot_diff` |

Baselines and notes: [docs/internal/profile.md](docs/internal/profile.md#snapshot-diff-micro-benchmarks).

---

## Must-read documents

| Doc | Why |
|-----|-----|
| [docs/analysis-architecture.md](docs/analysis-architecture.md) | Graph tiers, spill, CSR |
| [docs/design/blast-radius-design.md](docs/design/blast-radius-design.md) | Reachability / SCC |
| [docs/internal/profile.md](docs/internal/profile.md) | Cold profile deep dive |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Setup, tests, PR norms |
| [docs/contributor-checklist.md](docs/contributor-checklist.md) | Language / feature checklist |
| [docs/guides/semantic-search.md](docs/guides/semantic-search.md) | Embedders (if touching semantic) |
| [openspec/changes/_shared/starting-context.md](openspec/changes/_shared/starting-context.md) | OpenSpec pointer (canonical policy is this file) |

---

## Build and day-to-day commands

```bash
cargo build --release --bin rgctl
./target/release/rgctl --version
cargo test
```

Dashboard UI changes:

```bash
./scripts/build-dashboard.sh   # or dashboard/ npm ci && npm run build
cargo build --release
```

Code-daemon / ONNX weights: `git lfs pull` when using that embedder feature.

Dogfood fixtures: `rgctl-tests/` (e.g. ecommerce-*). Consumer agent pack: `rgctl install --skill --with-commands --tools cursor`.

---

## See also

- [docs/README.md](docs/README.md) — docs hub
- [docs/agents/USER_AGENTS_TEMPLATE.md](docs/agents/USER_AGENTS_TEMPLATE.md) — paste into *other* repos
- [docs/json-api.md](docs/json-api.md) — JSON schemas for CLI output

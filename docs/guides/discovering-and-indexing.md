# Discovering and Indexing a Codebase

## Introduction

The `discover` command is the foundation of every rgctl workflow. It parses your source code, builds a **code knowledge graph** of functions, classes, modules, and their relationships (calls, contains, imports), and runs configurable analytics on the result. Every other rgctl command — `gql`, `blast-radius`, `metrics`, and the rest — reads from the graph that `discover` produces.

Think of `discover` as the indexing step: you run it once (or after significant code changes), and then query the graph as many times as you like without re-parsing.

## Use Cases

- **Onboarding to an unfamiliar codebase.** Run `discover` to build an inventory of every function, class, and call relationship so you can query the structure instead of reading files one by one.
- **Pre-migration analysis.** Enable `--with-cfg`, `--with-harmonic`, and `--export-migration-hints` to generate a dependency-aware migration roadmap before refactoring a monolith.
- **CI integration.** Run `discover` in CI so downstream commands like `check` can enforce architectural policies on every pull request.
- **Deep program analysis.** Enable `--with-cfg` to build control-flow graphs, program dependence graphs, and dominator trees for every function — required by `slice`, `inspect`, and `cpg` commands.

## Example Project

This guide uses the **CoolStore** — a Java EE e-commerce application. It lives in `example/coolstore` and contains Java services, REST endpoints, JPA entities, and an AngularJS frontend.

## Choosing what to index

`discover` takes an optional `PATH` argument. How it combines with `-r` / `--repo` matters:

| Pattern | Indexes | When to use |
|---------|---------|-------------|
| `cd example/coolstore && rgctl discover .` | Current directory | **Recommended** in tutorials and day-to-day use |
| `rgctl -r example/coolstore discover` | The `-r` path (no `PATH` arg) | Scripts, agents (`export REPO=…`) |
| `rgctl discover /abs/path/to/coolstore` | Absolute path | One-shot from any cwd |

**Pitfall:** `rgctl -r example/coolstore discover` does **not** index `example/coolstore`. The positional `.` becomes the session root (usually your **shell cwd**), so `-r` is ignored. That can scan the wrong tree and fail on large parent directories.

**Artifacts:** `discover` writes snapshots under **`{repo}/.rgctl/`**. Add `.rgctl/` to `.gitignore`. Legacy daemon caches under `~/.rgctl/cache/` can be copied with `rgctl migrate-cache`.

## Step-by-Step

### 1. Basic Discovery

From the repository root:

```bash
cd example/coolstore
rgctl discover .
```

Or from anywhere, without a `PATH` argument:

```bash
rgctl -r example/coolstore discover
```

### Full pipeline

```bash
rgctl -r example/coolstore discover --full
```

This prints a plan, finishes a basic (queryable) index, then runs CFG + dashboard + harmonic centrality and a vocab semantic index. `--full` does not enable taint or secret scanning.

**Output:**

```
[>] rgctl discover
[!] Found 186 circular dependencies
[✓] rgctl discover finished in 598ms
```

**What happened:**

- rgctl scanned every supported source file (Java, JavaScript, and others) under the repository root.
- It detected 186 circular dependency cycles in the codebase.
- The graph snapshot was written to `example/coolstore/.rgctl/graph.snapshot.bin`.
- The entire process completed in under one second.

### 2. Discovery with Deep Analysis

To enable control-flow graphs, PDGs, and dominance analysis for every function, add `--with-cfg`:

```bash
rgctl -r example/coolstore discover --with-cfg
```

**Output:**

```
[>] rgctl discover
[!] Deep analysis enabled (--with-cfg / --with-taint).
   CFG/PDG on large codebases (>50K functions) may take several minutes.
Skipped files due to errors failed=1
[!] Found 186 circular dependencies

✓ Control flow analysis:
  Field writes indexed: 3299
  CFG/PDG/Dominance: 6585 functions analyzed
  Skipped: 941 functions (unsupported language or parse error)
[✓] rgctl discover finished in 20.7s
```

**What happened:**

- In addition to the basic graph, rgctl built a CFG (control-flow graph), PDG (program dependence graph), and dominator tree for each of the 6,585 parseable functions.
- It indexed 3,299 field-write sites, enabling the `cpg mutations` command.
- The analysis archive was written to `.rgctl/analysis/cfg_pdg.archive.bin`.
- 941 functions were skipped because they were in unsupported languages or had parse errors.

### 3. Full Analysis with Dashboard and Migration

For the most complete analysis, combine all the deep-analysis flags:

```bash
rgctl -r example/coolstore discover \
  --with-cfg \
  --with-dashboard \
  --with-harmonic \
  --export-migration-hints
```

This enables:

| Flag | Purpose |
|------|---------|
| `--with-cfg` | Build CFG, PDG, and dominator trees for every function |
| `--with-dashboard` | Export a static dashboard bundle to `.rgctl/dashboard/` |
| `--with-harmonic` | Compute harmonic centrality (needed for migration ranking) |
| `--export-migration-hints` | Write a migration roadmap to `.rgctl/migration_plan.json` |

### 4. Filtering by Language

If you only want to index Java files, use the `--languages` flag:

```bash
rgctl -r example/coolstore discover --languages java
```

This skips all JavaScript, TypeScript, and other files, producing a smaller, faster index focused on the backend code.

### 5. Excluding Directories

To skip vendor or generated code:

```bash
rgctl -r example/coolstore discover --exclude bower_components
```

### 6. Inspecting Artifacts

After discovery, artifacts live under **`{repo}/.rgctl/`**:

| Path | Content |
|------|---------|
| `.rgctl/graph.snapshot.bin` | Columnar graph snapshot |
| `.rgctl/analysis/cfg_pdg.archive.bin` | CFG/PDG/dominance archive (with `--with-cfg`) |
| `.rgctl/dashboard/manifest.json` | Dashboard metadata (with `--with-dashboard`) |
| `.rgctl/migration_plan.json` | Migration roadmap (with `--export-migration-hints`) |

## Discover Options Reference

| Option | Description |
|--------|-------------|
| `--with-cfg` | Per-function CFG, PDG, and dominator trees |
| `--with-taint` | Discover-time taint analysis (implies `--with-cfg`) |
| `--with-dfg-loops` | Classify loop-carried data dependencies on PDGs |
| `--with-ast-skeleton` | Write coarse AST skeleton archive |
| `--with-security` | Secret scanning |
| `--with-dashboard` | Export static dashboard bundle |
| `--with-harmonic` | Compute harmonic centrality for migration ranking |
| `--export-migration-hints` | Write migration roadmap JSON |
| `--write-json-graph` | Write legacy JSON graph files |
| `-l, --languages` | Restrict to specific languages |
| `-e, --exclude` | Exclude directories by name |
| `-v, --verbose` | Debug logging with stage profiling |
| `--full` | Staged pipeline: basic → CFG/dashboard/harmonic → semantic index |

## Benefits

- **Single command to index an entire codebase.** No build system integration, no compilation required — rgctl works directly on source files.
- **Incremental depth.** Start with basic discovery in under a second, then opt into deeper analysis (`--with-cfg`, `--with-taint`) only when you need it.
- **Multi-language support.** Nine Tier 1 languages (Java, Go, Rust, Python, JavaScript, TypeScript, C#, C, C++) plus markdown and config formats.
- **Foundation for all other commands.** Every query, metric, and analysis reads from the graph `discover` produces.

## Related Guides

- [Graph Query Language](graph-query-language.md) — query the graph that `discover` builds
- [Graph Metrics](graph-metrics.md) — run PageRank, betweenness, and community detection
- [Migration Planning](migration-planning.md) — use the `--export-migration-hints` output
- [HTTP Server and Dashboard](http-server-and-dashboard.md) — serve the `--with-dashboard` output in a browser

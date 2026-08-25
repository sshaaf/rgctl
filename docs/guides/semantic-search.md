# Semantic Search

## Introduction

The `semantic` command provides **natural-language search** over function symbols in your codebase. Instead of matching exact names or patterns, you describe what you are looking for in plain English (e.g., "shopping cart checkout") and rgctl returns the most semantically similar functions.

Semantic search is built on a separate opt-in index (`semantic_index.bin`) that embeds every function symbol into a vector space. Queries are then answered via Hamming nearest-neighbor search, returning results ranked by a **fusion score** that combines vector similarity with keyword matching.

## Use Cases

- **Exploring an unfamiliar codebase.** Ask "where is the payment processing logic?" without knowing function names.
- **Finding related functionality.** Search for "authentication" to discover all auth-related functions across the codebase.
- **Agent-assisted development.** LLM agents use semantic search to locate relevant code without reading every file.
- **Migration discovery.** Find all functions related to a concept (e.g., "database connection") when planning to extract a module.
- **Documentation cross-referencing.** Search across code and documentation sections with `--scope docs`.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` first:

```bash
rgctl -r example/coolstore discover .
```

## Step-by-Step

### 1. Build the Semantic Index

Semantic search requires a separate indexing step. Run `semantic index` after `discover`:

```bash
rgctl -r example/coolstore semantic index
```

**Output:**

```
Indexed 7526 functions (vocab-accumulate-v2, 256 dims) → example/coolstore/.rgctl/semantic_index.bin
  incremental: 0 reused, 7526 embedded, 0 removed
```

**What happened:**

- rgctl embedded all 7,526 function symbols into 256-dimensional vectors using the **vocab-accumulate-v2** model (a compiled token-table embedder that requires no external model or ONNX runtime).
- The index was written to `.rgctl/semantic_index.bin`.
- On subsequent runs, unchanged functions are reused (incremental indexing).

### 2. Query with Natural Language

Search for functions related to "shopping cart checkout":

```bash
rgctl -r example/coolstore -f json semantic query "shopping cart checkout" --limit 5
```

**Output:**

```json
{
  "dimensions": 256,
  "hits": [
    {
      "distance": 63,
      "file_path": "example/coolstore/./src/main/webapp/bower_components/angular-animate/angular-animate.js",
      "fused_score": 0.4749,
      "name": "close",
      "node_id": "5bda7d09-ac18-40c7-a90a-3a8876894947",
      "ranking": "fusion",
      "score": 0.4749
    }
  ],
  "index_schema_version": 2,
  "model_id": "vocab-accumulate-v2",
  "query": "shopping cart checkout",
  "schema_version": 3
}
```

**What this tells you:**

- **`hits`** -- the top 5 most semantically similar functions, ranked by `fused_score`.
- **`distance`** -- the Hamming distance in the vector space (lower is more similar).
- **`fused_score`** -- a combined score from vector similarity and keyword matching (fusion ranking).
- **`ranking: "fusion"`** -- indicates the result was ranked using both semantic and keyword signals.

### 3. Keyword-Only Search

Disable fusion to use pure keyword matching:

```bash
rgctl -r example/coolstore -f json semantic query "checkout" \
  --limit 5 --no-fusion
```

### 4. Increase Candidate Pool

For more thorough searches, increase the candidate pool that the fusion step considers:

```bash
rgctl -r example/coolstore -f json semantic query "order processing" \
  --limit 10 --candidate-pool 200
```

### 5. Community-Scoped Search

Search within a specific scope. Community-scoped search finds entire communities relevant to your query:

```bash
rgctl -r example/coolstore -f json semantic query "checkout" \
  --scope community --limit 10
```

### 6. Document-Scoped Search

To search documentation sections instead of code functions, first index with the `docs` scope:

```bash
rgctl -r example/coolstore semantic index --scope docs --embedder hash
```

Then query:

```bash
rgctl -r example/coolstore -f json semantic query "deployment instructions" \
  --scope docs --limit 5
```

Note: the `--scope` flag on the `index` command determines what gets embedded. The `--scope` on `query` does not filter -- it is the index scope that matters.

### 7. Choosing an Embedder

rgctl supports three embedding backends:

| Embedder | Command | Requirements | Best For |
|----------|---------|-------------|----------|
| **vocab** (default) | `semantic index` | None | General use, no external dependencies |
| **hash** | `semantic index --embedder hash` | None | Fast indexing, CI environments |
| **code-daemon** | `semantic index --embedder code-daemon` | ONNX weights (Git LFS) | Highest quality embeddings |

For most users, the default `vocab` embedder provides a good balance of quality and speed.

## Understanding Results

| Field | Meaning |
|-------|---------|
| `fused_score` | Combined semantic + keyword similarity (0.0 = no match, 1.0 = perfect match) |
| `distance` | Raw Hamming distance in vector space (lower = more similar) |
| `ranking` | Ranking method used: `fusion` (default) or `vector` |
| `name` | Function name |
| `file_path` | Source file containing the function |
| `node_id` | Unique graph node ID for cross-referencing with other commands |

## Benefits

- **No exact name required.** Find functions by describing what they do, not what they are called.
- **Zero external dependencies.** The default `vocab` embedder is compiled into the binary -- no model downloads, no ONNX runtime.
- **Incremental indexing.** Only re-embeds changed functions on subsequent runs.
- **Fusion ranking.** Combines vector similarity with keyword matching for more relevant results.
- **Multi-scope.** Search code functions, documentation sections, or communities.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- must run `discover` before semantic index
- [Graph Query Language](graph-query-language.md) -- use GQL for exact structural queries
- [Community Detection](community-detection.md) -- understand community-scoped semantic search
- [HTTP Server and Dashboard](http-server-and-dashboard.md) -- run semantic queries via the HTTP API

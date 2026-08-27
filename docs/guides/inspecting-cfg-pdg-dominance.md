# Inspecting CFG, PDG, and Dominance

## Introduction

The `inspect` command lets you examine the raw **Control-Flow Graph (CFG)**, **Program Dependence Graph (PDG)**, and **Dominator Tree** for any function in your codebase. These are the foundational data structures of program analysis -- the CFG shows how execution flows through a function, the PDG shows data and control dependencies between statements, and the dominator tree shows which basic blocks dominate others.

While higher-level commands like `slice` and `cpg flows` build on these structures, `inspect` gives you direct access to the raw graphs when you need to understand the internal structure of a function at the deepest level.

## Use Cases

- **Understand complex control flow.** Visualize how branches, loops, and exceptions create execution paths through a function.
- **Debug program analysis.** When a slice or flow result is unexpected, inspect the underlying CFG/PDG to understand why.
- **Academic and research use.** Access standard program analysis data structures for experimentation.
- **Security analysis.** Examine control-flow and data-dependency structures for potential vulnerabilities.
- **Compiler-style optimization analysis.** Use dominator trees and dominance frontiers for SSA-related analysis.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` with `--with-cfg`:

```bash
rgctl -r example/coolstore discover --with-cfg
```

## Step-by-Step

### 1. Control-Flow Graph (CFG)

Inspect the CFG for `priceShoppingCart`:

```bash
rgctl -r example/coolstore -f json inspect priceShoppingCart cfg
```

**Output (truncated):**

```json
{
  "edges": [
    {"kind": "next", "source": "block_5", "target": "block_7"},
    {"kind": "iftrue", "source": "block_7", "target": "block_8"},
    {"kind": "iffalse", "source": "block_7", "target": "block_1"},
    {"kind": "next", "source": "block_8", "target": "block_9"},
    {"kind": "iftrue", "source": "block_9", "target": "block_10"},
    {"kind": "iffalse", "source": "block_9", "target": "block_3"},
    {"kind": "iftrue", "source": "block_12", "target": "block_13"},
    {"kind": "iffalse", "source": "block_12", "target": "block_14"},
    {"kind": "jump", "source": "block_13", "target": "block_12"},
    {"kind": "iftrue", "source": "block_15", "target": "block_16"},
    {"kind": "iffalse", "source": "block_15", "target": "block_0"}
  ],
  "nodes": [
    {"block_index": 0, "label": "...", "lines": [83]},
    {"block_index": 5, "label": "ENTRY", "lines": [54]}
  ],
  "schema_version": 1
}
```

**What this tells you:**

- **`edges`** -- directed edges between basic blocks with edge kinds:
  - `next` -- unconditional fall-through to the next block
  - `iftrue` / `iffalse` -- conditional branch (true/false arm)
  - `jump` -- unconditional jump (e.g., loop back-edge)
- **`nodes`** -- basic blocks with their source line numbers and code labels.
- The `jump` edge from `block_13` to `block_12` is a loop back-edge (the `for` loop over shopping cart items).
- Multiple `iftrue`/`iffalse` pairs show the nested `if` statements in the function (null check, list size check, price threshold).

### 2. Program Dependence Graph (PDG)

The PDG combines control and data dependencies into a single graph:

```bash
rgctl -r example/coolstore -f json inspect priceShoppingCart pdg
```

**Output (truncated):**

```json
{
  "control_deps": 18,
  "data_deps": 5,
  "edges": [
    {"kind": "data", "source": "node_7", "target": "node_8", "variable": "sci"},
    {"kind": "data", "source": "node_7", "target": "node_8", "variable": "getQuantity"},
    {"kind": "data", "source": "node_7", "target": "node_8", "variable": "sc"},
    {"kind": "data", "source": "node_4", "target": "node_6", "variable": "sc"},
    {"kind": "data", "source": "node_12", "target": "node_13", "variable": "sc"},
    {"kind": "control", "source": "node_1", "target": "node_5"},
    {"kind": "control", "source": "node_1", "target": "node_12"},
    {"kind": "control", "source": "node_1", "target": "node_13"},
    {"kind": "control", "source": "node_8", "target": "node_5"},
    {"kind": "control", "source": "node_9", "target": "node_12"}
  ],
  "schema_version": 1
}
```

**What this tells you:**

- **18 control dependencies** -- statements that are conditionally executed based on branch conditions.
- **5 data dependencies** -- variables that flow from one statement to another.
- **`kind: "data"`** edges show where a variable defined at the source node is used at the target node. For example, `sc` flows from node_4 to node_6, and `sci` flows from node_7 to node_8.
- **`kind: "control"`** edges show that a statement's execution depends on a branch condition.

### 3. Dominator Tree

The dominator tree shows which basic blocks dominate (must execute before) others:

```bash
rgctl -r example/coolstore -f json inspect priceShoppingCart dom
```

**Output (truncated):**

```json
{
  "frontiers": [],
  "idom": [
    {"block": 11, "immediate_dominator": 10},
    {"block": 2, "immediate_dominator": 5},
    {"block": 8, "immediate_dominator": 7},
    {"block": 12, "immediate_dominator": 5},
    {"block": 3, "immediate_dominator": 15},
    {"block": 10, "immediate_dominator": 9},
    {"block": 9, "immediate_dominator": 8},
    {"block": 7, "immediate_dominator": 5},
    {"block": 5, "immediate_dominator": 5}
  ],
  "layer": "dom",
  "schema_version": 1
}
```

**What this tells you:**

- **`idom`** -- the immediate dominator for each basic block. Block 5 (the entry block) dominates most other blocks, as expected.
- **`frontiers`** -- dominance frontiers, used for SSA construction. In this example, no explicit frontiers are shown (use `--frontiers` for detailed output).
- The dominator tree is a tree structure rooted at the entry block; each block's immediate dominator is the closest block that must execute before it on every path.

### 4. Dominance Frontiers

Request dominance frontiers explicitly:

```bash
rgctl -r example/coolstore -f json inspect priceShoppingCart dom --frontiers
```

Dominance frontiers identify the join points in the CFG where phi functions would be needed in SSA form.

### 5. Pruned CFG

For a cleaner view, prune unreachable blocks:

```bash
rgctl -r example/coolstore -f json inspect priceShoppingCart cfg --prune
```

### 6. PDG with Edge Layer Filtering

Filter PDG edges by layer:

```bash
rgctl -r example/coolstore -f json inspect priceShoppingCart pdg --edge-layer data
```

This shows only data-dependency edges, filtering out control dependencies for a cleaner view.

```bash
rgctl -r example/coolstore -f json inspect priceShoppingCart pdg --def-use
```

The `--def-use` flag adds def-use chain information to the edges.

## Inspect Subcommands

| Subcommand | Description | Key Options |
|------------|-------------|-------------|
| `cfg` | Control-flow graph | `--prune` (remove unreachable blocks) |
| `pdg` | Program dependence graph | `--edge-layer` (filter by data/control), `--def-use` |
| `dom` | Dominator tree | `--frontiers` (show dominance frontiers) |

## Benefits

- **Full transparency.** See exactly the data structures that power slicing, flow analysis, and the CPG.
- **Standard representations.** CFG, PDG, and dominator trees are well-understood program analysis structures with extensive academic literature.
- **Debugging aid.** When higher-level commands produce unexpected results, inspect the underlying graphs to understand why.
- **Multi-format output.** Use `-f json` for programmatic access or `-f graphviz` for visualization.
- **Function-level precision.** Each graph is scoped to a single function, keeping the output manageable.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- `discover --with-cfg` is required
- [Program Slicing](program-slicing.md) -- slicing operates on the PDG that `inspect` exposes
- [Hybrid CPG](hybrid-cpg.md) -- the CPG facade that combines these graphs with the call graph
- [Blast Radius Analysis](blast-radius-analysis.md) -- function-level analysis that complements statement-level inspection

# Hybrid CPG

## Introduction

The `cpg` command provides a **Hybrid Code Property Graph (CPG)** -- a unified facade that bridges the repository-level call graph (L_repo) with per-function CFG/PDG analysis (L_proc). It gives you a single interface for function lookups, call neighborhoods, field mutation tracking, data flow analysis, PDG inspection, program slicing, and CPG export.

The "hybrid" in the name refers to the two-resolution architecture: the coarse-grained repository graph provides call and containment relationships across the entire codebase, while the fine-grained per-function analysis provides control flow, data dependencies, and statement-level detail within each function.

## Use Cases

- **DTO safety analysis.** Use `cpg mutations` to find every place a data transfer object is modified, ensuring encapsulation.
- **Data flow tracing.** Use `cpg flows` to trace a variable through a function without needing to understand PDG internals.
- **Call neighborhood exploration.** Use `cpg calls` to see all incoming and outgoing calls for a function.
- **Function resolution.** Use `cpg function` to look up a function by name and check if deep analysis data is available.
- **CPG export.** Export the hybrid graph in GraphSON or GraphML format for external tools.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` with `--with-cfg`:

```bash
rgctl -r example/coolstore discover . --with-cfg
```

## Step-by-Step

### 1. Check CPG Status

First, verify that the CPG archive is available:

```bash
rgctl -r example/coolstore -f json cpg status
```

**Output:**

```json
{
  "archive_path": "example/coolstore/.rgbuilder/analysis/cfg_pdg.archive.bin",
  "archive_present": true,
  "ast_skeleton_count": 0,
  "ast_skeleton_present": false,
  "field_write_count": 3299,
  "field_write_index_present": true,
  "function_count": 6585,
  "graph_digest": "98d77fdb4c9ecf4778c10ea2f2d78cc62ed0f21711f595f08127c071dabf76d3",
  "schema_version": 1
}
```

**What this tells you:**

- **`archive_present: true`** -- the CFG/PDG archive exists (from `discover --with-cfg`).
- **`function_count: 6585`** -- 6,585 functions have been analyzed at the L_proc level.
- **`field_write_count: 3299`** -- 3,299 field-write sites are indexed for mutation tracking.
- **`field_write_index_present: true`** -- the `cpg mutations` command is available.
- **`ast_skeleton_present: false`** -- the AST skeleton archive is not present (requires `discover --with-ast-skeleton`).

### 2. Resolve a Function

Look up a function by name and check if L_proc data exists:

```bash
rgctl -r example/coolstore -f json cpg function priceShoppingCart
```

**Output:**

```json
{
  "file_path": "example/coolstore/./src/main/java/com/redhat/coolstore/service/ShoppingCartService.java",
  "has_l_proc": true,
  "id": "7b380647-19dc-49d3-96e5-11216a9fde32",
  "is_constructor": false,
  "name": "priceShoppingCart",
  "qualified_name": "com.redhat.coolstore.service.ShoppingCartService.priceShoppingCart",
  "schema_version": 1,
  "start_line": 54
}
```

**What this tells you:**

- **`has_l_proc: true`** -- this function has CFG/PDG data available for deep analysis.
- **`start_line: 54`** -- the function starts at line 54, useful for `slice` and `flows` commands.
- **`qualified_name`** -- the fully qualified name for unambiguous reference.

### 3. Call Neighborhood

See all functions called by (and calling) `priceShoppingCart`:

```bash
rgctl -r example/coolstore -f json cpg calls priceShoppingCart
```

**Output (truncated):**

```json
{
  "edges": [
    {"direction": "out", "id": "06f37e6d-...", "name": "setShippingTotal"},
    {"direction": "out", "id": "21694893-...", "name": "getCartItemTotal"},
    {"direction": "out", "id": "2eee3e96-...", "name": "setCartItemPromoSavings"},
    {"direction": "out", "id": "49de8aad-...", "name": "getCartItemPromoSavings"},
    {"direction": "out", "id": "5b4ce417-...", "name": "initShoppingCartForPricing"},
    {"direction": "out", "id": "84eebe14-...", "name": "setCartTotal"},
    {"direction": "out", "id": "8536ade9-...", "name": "getShoppingCartItemList"},
    {"direction": "out", "id": "b6dd115f-...", "name": "setCartItemTotal"}
  ],
  "schema_version": 1
}
```

**What this tells you:**

- **`direction: "out"`** -- these are outgoing calls (functions that `priceShoppingCart` calls).
- The function calls setter/getter methods on `ShoppingCart` (`setShippingTotal`, `getCartItemTotal`, etc.), an initialization method (`initShoppingCartForPricing`), and accumulation methods.
- This reveals the function's dependencies without reading the source code.

### 4. Field Mutations

The `cpg mutations` command finds every place a type's fields are modified -- critical for DTO safety and encapsulation analysis:

```bash
rgctl -r example/coolstore -f json cpg mutations \
  --type ShoppingCart --exclude-ctors
```

**Output:**

```json
{
  "exclude_ctors": true,
  "include_unresolved": false,
  "mutations": [
    {
      "code": "this.shoppingCartItemList = shoppingCartItemList",
      "file": "example/coolstore/./src/main/java/com/redhat/coolstore/model/ShoppingCart.java",
      "function": "setShoppingCartItemList",
      "is_constructor": false,
      "kind": "ThisField",
      "line": 35,
      "member": "shoppingCartItemList",
      "receiver_local": "this",
      "receiver_type": "com.redhat.coolstore.model.ShoppingCart"
    },
    {
      "code": "this.cartItemTotal = cartItemTotal",
      "file": "example/coolstore/./src/main/java/com/redhat/coolstore/model/ShoppingCart.java",
      "function": "setCartItemTotal",
      "is_constructor": false,
      "kind": "ThisField",
      "line": 71,
      "member": "cartItemTotal"
    },
    {
      "code": "this.shippingTotal = shippingTotal",
      "function": "setShippingTotal",
      "kind": "ThisField",
      "line": 79,
      "member": "shippingTotal"
    },
    {
      "code": "this.cartTotal = cartTotal",
      "function": "setCartTotal",
      "kind": "ThisField",
      "line": 87,
      "member": "cartTotal"
    },
    {
      "code": "this.cartItemPromoSavings = cartItemPromoSavings",
      "function": "setCartItemPromoSavings",
      "kind": "ThisField",
      "line": 95,
      "member": "cartItemPromoSavings"
    },
    {
      "code": "this.shippingPromoSavings = shippingPromoSavings",
      "function": "setShippingPromoSavings",
      "kind": "ThisField",
      "line": 103,
      "member": "shippingPromoSavings"
    }
  ],
  "schema_version": 1,
  "type_name": "ShoppingCart"
}
```

**What this tells you:**

- Every field of `ShoppingCart` is mutated only through its setter methods (all `kind: "ThisField"`).
- The `--exclude-ctors` flag filters out constructor writes, focusing on post-construction mutations.
- Six fields are modified: `shoppingCartItemList`, `cartItemTotal`, `shippingTotal`, `cartTotal`, `cartItemPromoSavings`, and `shippingPromoSavings`.
- Each mutation includes the exact code, file, line number, and the function that performs the write.

### 5. Data Flows

Trace how a variable flows through a function:

```bash
rgctl -r example/coolstore -f json cpg flows \
  ./src/main/java/com/redhat/coolstore/service/ShoppingCartService.java \
  --line 68 --variable sc --function priceShoppingCart --direction forward
```

**Output:**

```json
{
  "direction": "forward",
  "function": "priceShoppingCart",
  "line": 68,
  "lines": [64, 68, 81, 83],
  "reduction_percent": 66.67,
  "steps": [
    {"code": "for-each sc.getShoppingCartItemList()", "line": 64},
    {"code": "sc.setCartItemTotal(sc.getCartItemTotal() + sci.getPrice() * sci.getQuantity())", "line": 68},
    {"code": "ps.applyShippingPromotions(sc)", "line": 81},
    {"code": "sc.setCartTotal(sc.getCartItemTotal() + sc.getShippingTotal())", "line": 83}
  ],
  "variable": "sc"
}
```

The `flows` subcommand wraps `slice` with a simpler interface, showing the step-by-step data flow path through the function.

### 6. PDG Overlay

Inspect the program dependence graph for a function:

```bash
rgctl -r example/coolstore -f json cpg pdg priceShoppingCart
```

**Output (truncated):**

```json
{
  "control_deps": 18,
  "data_deps": 5,
  "edges": [
    {"kind": "data", "source": "node_7", "target": "node_8", "variable": "sci"},
    {"kind": "data", "source": "node_7", "target": "node_8", "variable": "sc"},
    {"kind": "control", "source": "node_1", "target": "node_5"},
    {"kind": "control", "source": "node_1", "target": "node_12"}
  ],
  "schema_version": 1
}
```

This shows 18 control dependencies and 5 data dependencies in `priceShoppingCart`, with the full edge list for graph analysis.

## CPG Subcommands Reference

| Subcommand | Purpose | Requires |
|------------|---------|----------|
| `status` | Check archive readiness | `discover` |
| `function` | Resolve a function by name | `discover` |
| `calls` | Call neighborhood (in/out) | `discover` |
| `mutations` | Field mutations for a type | `discover --with-cfg` |
| `flows` | Data/control flow tracing | `discover --with-cfg` |
| `pdg` | PDG overlay | `discover --with-cfg` |
| `slice` | Line-level slice (wraps `slice`) | `discover --with-cfg` |
| `ast` | AST skeleton | `discover --with-ast-skeleton` |
| `export` | Export CPG view | `discover --with-cfg` |

## Benefits

- **Unified interface.** One command family for both repository-level and function-level analysis.
- **DTO safety.** `mutations` gives you a complete audit of where a type's fields are changed.
- **Readable data flows.** `flows` presents data dependencies as a step-by-step narrative.
- **No source code reading.** Get call neighborhoods, mutations, and flows without loading files.
- **Exportable.** Export the hybrid CPG to GraphSON or GraphML for external analysis tools.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- `discover --with-cfg` is required for most CPG subcommands
- [Program Slicing](program-slicing.md) -- the underlying technique that `cpg flows` and `cpg slice` use
- [Inspecting CFG, PDG, and Dominance](inspecting-cfg-pdg-dominance.md) -- lower-level inspection of the same data
- [Blast Radius Analysis](blast-radius-analysis.md) -- function-level impact analysis from the same graph

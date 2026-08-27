# Agent Skill

## Introduction

The rgctl **agent skill** is a structured instruction set that teaches AI coding agents (Claude Code, Cursor) how to use the rgctl CLI to answer structural questions about a codebase. When installed into a repository, the skill gives your agent the ability to automatically map natural-language questions to the right rgctl commands, interpret the results, and report findings -- all without the developer needing to know the CLI syntax.

The skill works by embedding a `SKILL.md` file into your project's agent skill directories (`.claude/skills/rgctl/` and `.cursor/skills/rgctl/`). This file is compiled directly into the `rgctl` binary, so installing it is a single command with no external downloads. Once installed, the agent follows a structured loop: parse the user's natural-language question, route it to the appropriate rgctl command, execute it, and summarize the results.

The skill turns rgctl from a CLI tool into an **always-available architectural advisor** inside your editor.

## Use Cases

The agent skill unlocks several powerful workflows where the combination of structural graph analysis and AI reasoning produces results that neither could achieve alone.

### Refactoring with Confidence

Before renaming, extracting, or restructuring a function, the agent can automatically check the blast radius, trace callers, and verify that no hidden dependencies will break. Instead of manually running commands, you ask the agent a natural-language question and it handles the analysis.

### Monolith-to-Microservice Migration

The agent can generate a complete migration roadmap, explain the ordering rationale, identify high-risk extraction targets, and guide you through each step -- all from conversational prompts like "generate a migration plan for this repo" or "what should we extract first?"

### Porting a Function from One Language to Another

When rewriting a function from Java to Go (or any language pair), the agent can extract the function's data-flow graph, call neighborhood, and field mutations from the source language, then verify that the target-language implementation preserves the same logic and data-flow structure. The graph provides a language-independent ground truth.

### Writing Better Test Cases

The agent can analyze a function's control-flow graph, identify all branch paths, trace data dependencies, and use that structural information to generate test cases that cover the actual code paths rather than guessing at coverage.

### Continuous Architecture Review

With the skill installed, every code review conversation has access to architectural context. The agent can check policy compliance, detect coupling drift, and flag high-impact changes before they are merged.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` first:

```bash
rgctl -r example/coolstore discover --with-cfg
```

## Step-by-Step

### 1. Install the Skill

Install the rgctl agent skill into your repository:

```bash
rgctl -r example/coolstore install --skill
```

**Output:**

```
[>] rgctl install
Installed rgctl skill:
  created  /path/to/example/coolstore/.claude/skills/rgctl/SKILL.md
  created  /path/to/example/coolstore/.cursor/skills/rgctl/SKILL.md
[✓] rgctl install finished in 1ms
```

**What happened:**

- rgctl extracted the embedded skill bundle (`SKILL.md`, `references/*.md`, etc.) and wrote it under each host skill directory.
- `.claude/skills/rgctl/` — for Claude Code
- `.cursor/skills/rgctl/` — for Cursor
- Subdirectories such as **`references/`** are preserved (GQL patterns, workflows, command encyclopedia).
- No network requests, no model downloads — the skill content is baked into the `rgctl` binary.

### 2. Verify with JSON Output

Check the install status programmatically:

```bash
rgctl -r example/coolstore -f json install --skill
```

**Output:**

```json
{
  "command": "install",
  "force": false,
  "repo": "/path/to/example/coolstore",
  "schema_version": 1,
  "skill": "rgctl",
  "writes": [
    {
      "host": "claude",
      "path": "/path/to/.claude/skills/rgctl/SKILL.md",
      "status": "unchanged"
    },
    {
      "host": "cursor",
      "path": "/path/to/.cursor/skills/rgctl/SKILL.md",
      "status": "unchanged"
    }
  ]
}
```

The `status` field for each write is one of:
- `created` -- new file written
- `unchanged` -- file already exists with identical content (idempotent)
- `overwritten` -- existing file replaced (with `--force`)
- `skipped_exists` -- file differs but `--force` was not set (exit code 1)

### 3. Install for a Specific Agent

If you only use one agent platform:

```bash
# Claude Code only
rgctl -r example/coolstore install --skill --host claude

# Cursor only
rgctl -r example/coolstore install --skill --host cursor
```

### 4. Update After Upgrading rgctl

When you upgrade rgctl, the embedded skill may have changed. Update it with `--force`:

```bash
rgctl -r example/coolstore install --skill --force
```

This overwrites any existing skill files, even if they have been modified locally.

### 5. The Agent Loop

Once installed, the agent follows a 5-step loop for every structural question:

```
1. USER PROMPT     "What's the impact of changing priceShoppingCart?"
2. TOOL CALL       rgctl -f json blast-radius priceShoppingCart
3. GRAPH FACTS     Parse JSON: score 40.35, 5 callers, impact zone 7
4. LLM REASONING   Summarize: moderate risk, spans service and REST layers
5. ACTION          Report findings, suggest next steps
```

The skill includes a **decision table** that maps 19 categories of natural-language questions to the right rgctl command. The agent does not need to be told which command to use -- it matches the user's intent automatically.

---

## Use Case: Refactoring with Confidence

### Scenario

You want to refactor `priceShoppingCart` in the CoolStore application -- perhaps splitting it into separate pricing methods for items and shipping. Before making changes, you need to understand the full impact.

### What the Agent Does

When you ask the agent: *"What happens if I change priceShoppingCart?"*

The agent follows the skill's decision table (row 10: "Impact if I change X") and runs:

```bash
rgctl -r example/coolstore -f json blast-radius priceShoppingCart
```

**Output:**

```json
{
  "metrics": {
    "score": 40.35,
    "direct_callers_count": 5,
    "impact_zone_size": 7
  },
  "target": {
    "canonical_fqn": "ShoppingCartService::priceShoppingCart",
    "file_path": "example/coolstore/./src/main/java/com/redhat/coolstore/service/ShoppingCartService.java",
    "signature": "public void priceShoppingCart(ShoppingCart sc) {"
  },
  "topology": {
    "direct_callers": [
      {"fqn": "com.redhat.coolstore.service.ShoppingCartService.checkOutShoppingCart"},
      {"fqn": "com.redhat.coolstore.rest.CartEndpoint.add"},
      {"fqn": "com.redhat.coolstore.rest.CartEndpoint.dedupeCartItems"},
      {"fqn": "com.redhat.coolstore.rest.CartEndpoint.delete"},
      {"fqn": "com.redhat.coolstore.rest.CartEndpoint.set"}
    ]
  }
}
```

The agent then reports:

> priceShoppingCart has a blast-radius score of 40.4/100 (moderate risk). It is called by 5 functions: checkOutShoppingCart in the service layer, and 4 REST endpoint methods (add, dedupeCartItems, delete, set). The total impact zone is 7 functions. If you change the signature, all 5 callers will need updating.

### Going Deeper: Call Neighborhood

The agent can then trace the call neighborhood to understand what `priceShoppingCart` itself depends on:

```bash
rgctl -r example/coolstore -f json cpg calls priceShoppingCart
```

**Output (truncated):**

```json
{
  "edges": [
    {"direction": "out", "name": "initShoppingCartForPricing"},
    {"direction": "out", "name": "getShoppingCartItemList"},
    {"direction": "out", "name": "setCartItemPromoSavings"},
    {"direction": "out", "name": "getCartItemTotal"},
    {"direction": "out", "name": "setCartItemTotal"},
    {"direction": "out", "name": "setShippingTotal"},
    {"direction": "out", "name": "setCartTotal"}
  ]
}
```

The agent reports: *"priceShoppingCart calls 7 functions: it initializes pricing, iterates cart items to compute totals and promo savings, calculates shipping, and sets the final cart total. A refactoring that splits item pricing from shipping pricing should preserve these call relationships."*

### Verifying Field Mutations

Before splitting the function, check which fields it modifies:

```bash
rgctl -r example/coolstore -f json cpg mutations \
  --type ShoppingCart --exclude-ctors
```

**Output:**

```json
{
  "mutations": [
    {"function": "setShoppingCartItemList", "member": "shoppingCartItemList", "line": 35},
    {"function": "setCartItemTotal", "member": "cartItemTotal", "line": 71},
    {"function": "setShippingTotal", "member": "shippingTotal", "line": 79},
    {"function": "setCartTotal", "member": "cartTotal", "line": 87},
    {"function": "setCartItemPromoSavings", "member": "cartItemPromoSavings", "line": 95},
    {"function": "setShippingPromoSavings", "member": "shippingPromoSavings", "line": 103}
  ]
}
```

The agent reports: *"ShoppingCart has 6 mutable fields. The priceShoppingCart method writes to cartItemTotal, shippingTotal, cartTotal, cartItemPromoSavings, and shippingPromoSavings through their setters. If you split the method, ensure both halves still set all required fields."*

---

## Use Case: Migration Planning

### Scenario

You need to decompose the CoolStore application into microservices and want a data-driven extraction plan.

### What the Agent Does

When you ask: *"Generate a migration plan for this codebase"*

The agent follows the skill's decision table (row 1: "Generate a migration plan") and runs:

```bash
rgctl -r example/coolstore discover \
  --with-cfg --with-harmonic --export-migration-hints \
  --migration-preset hybrid_default --migration-order scheduled
```

Then reads the plan:

```json
{
  "preset": "hybrid_default",
  "order_mode": "scheduled",
  "steps": [
    {
      "step": 1,
      "label": "main.webapp.bower_components.matches-selector",
      "max_blast": 0.0,
      "avg_pagerank": 0.0000634
    },
    {
      "step": 2,
      "label": "com.redhat.coolstore.model",
      "max_blast": 0.0,
      "avg_pagerank": 0.0000532
    }
  ]
}
```

The agent reports:

> The migration plan contains N steps using the hybrid_default preset (balanced weighting of PageRank, harmonic centrality, and blast radius). The first extraction targets are low-risk leaf modules with zero blast radius. Step 2 extracts the domain model (com.redhat.coolstore.model), which has no blast-radius impact and can be safely moved to a shared library.

### Investigating a Migration Step

When the user asks: *"Tell me more about the coolstore model community"*

The agent queries community members:

```bash
rgctl -r example/coolstore -f json gql \
  "MATCH (f:Function) WHERE f.community_id = '13' RETURN f LIMIT 20"
```

And checks blast radius on key functions:

```bash
rgctl -r example/coolstore blast-radius getShoppingCart --depth 3
```

---

## Use Case: Porting a Function to Another Language

### Scenario

You are migrating `priceShoppingCart` from Java to Go (or TypeScript, Rust, Python, etc.) and need to ensure the new implementation preserves the same data flow and logic.

### What the Agent Does

**Step 1: Extract the structural blueprint from the source language.**

The agent captures the function's data-flow graph, which is language-independent:

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
  "steps": [
    {"code": "for-each sc.getShoppingCartItemList()", "line": 64},
    {"code": "sc.setCartItemTotal(sc.getCartItemTotal() + sci.getPrice() * sci.getQuantity())", "line": 68},
    {"code": "ps.applyShippingPromotions(sc)", "line": 81},
    {"code": "sc.setCartTotal(sc.getCartItemTotal() + sc.getShippingTotal())", "line": 83}
  ],
  "reduction_percent": 66.67
}
```

**Step 2: Extract the call neighborhood.**

```bash
rgctl -r example/coolstore -f json cpg calls priceShoppingCart
```

This gives the agent the complete list of outgoing calls that the new implementation must replicate.

**Step 3: Extract field mutations.**

```bash
rgctl -r example/coolstore -f json cpg mutations \
  --type ShoppingCart --exclude-ctors
```

This lists every field that `priceShoppingCart` writes to, which the new implementation must also write.

**Step 4: Extract the PDG.**

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
    {"kind": "data", "source": "node_4", "target": "node_6", "variable": "sc"},
    {"kind": "control", "source": "node_1", "target": "node_5"}
  ]
}
```

**How the agent uses this:**

The agent now has four language-independent facts about `priceShoppingCart`:

1. **Data flow path**: iteration over cart items, accumulation of totals, shipping calculation, final total.
2. **Call contract**: the 7 functions it must call in the same order.
3. **Mutation contract**: the 5 ShoppingCart fields it must write.
4. **Dependency graph**: 18 control edges and 5 data edges encoding the computation structure.

When writing the Go implementation, the agent verifies each fact against the new code. If a data-flow step is missing, a call is omitted, or a field mutation is skipped, the agent flags it.

The agent can report: *"The Java implementation of priceShoppingCart has 4 data-flow steps, calls 7 functions, and mutates 5 fields on ShoppingCart. Your Go implementation should replicate the same iteration pattern (for-each over cart items), accumulate the same totals, and call equivalent functions for shipping calculation and promo application."*

---

## Use Case: Writing Better Test Cases

### Scenario

You need to write tests for `priceShoppingCart` and want to ensure you cover all branch paths and edge cases.

### What the Agent Does

**Step 1: Examine the control-flow graph.**

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
    {"kind": "iftrue", "source": "block_9", "target": "block_10"},
    {"kind": "iffalse", "source": "block_9", "target": "block_3"},
    {"kind": "iftrue", "source": "block_12", "target": "block_13"},
    {"kind": "iffalse", "source": "block_12", "target": "block_14"},
    {"kind": "jump", "source": "block_13", "target": "block_12"},
    {"kind": "iftrue", "source": "block_15", "target": "block_16"},
    {"kind": "iffalse", "source": "block_15", "target": "block_0"}
  ]
}
```

The agent identifies all branch points:

> priceShoppingCart has 4 branch conditions:
> 1. Null check: `sc != null` (block_7) -- test with null and non-null input
> 2. List check: `sc.getShoppingCartItemList() != null && size > 0` (block_9/10) -- test with empty and populated cart
> 3. Loop: `for-each sci : sc.getShoppingCartItemList()` (block_12/13) -- test with 0, 1, and many items
> 4. Price threshold: `sc.getCartItemTotal() >= 25` (block_15) -- test below and above the threshold

**Step 2: Examine data dependencies.**

```bash
rgctl -r example/coolstore -f json inspect priceShoppingCart pdg --edge-layer data
```

The agent identifies the key data flows:

> Variable `sc` flows through 3 data-dependency edges. Variable `sci` flows through the loop body. Test cases should verify that each accumulation step (cartItemTotal, cartItemPromoSavings, shippingTotal) produces correct values.

**Step 3: Check field mutations to verify test assertions.**

```bash
rgctl -r example/coolstore -f json cpg mutations \
  --type ShoppingCart --exclude-ctors
```

The agent reports: *"Your tests should assert on 5 ShoppingCart fields after calling priceShoppingCart: cartItemTotal, shippingTotal, cartTotal, cartItemPromoSavings, and shippingPromoSavings. Here are the test cases derived from the CFG:"*

The agent can then generate concrete test cases:

1. **Null input**: call `priceShoppingCart(null)` -- no fields should be mutated.
2. **Empty cart**: cart with no items -- totals should be zero.
3. **Single item below threshold**: one item priced below 25 -- no shipping insurance.
4. **Single item above threshold**: one item priced at or above 25 -- shipping insurance applied.
5. **Multiple items with promotions**: verify promo savings accumulate correctly.
6. **Boundary case**: cart total exactly 25 -- verify threshold behavior.

---

## The NL-to-Command Decision Table

The skill embeds a decision table that maps natural-language patterns to CLI commands. Here are the key mappings:

| What You Ask the Agent | What the Agent Runs |
|------------------------|---------------------|
| "Generate a migration plan" | `discover . --with-cfg --with-harmonic --export-migration-hints` |
| "What are the bottlenecks?" | `metrics --pagerank` |
| "List all functions" | `gql --macro-name all_functions unused` |
| "What communities exist?" | `communities list` |
| "Where is the checkout flow?" | `semantic index` then `semantic query "checkout flow"` |
| "What's the impact of changing X?" | `blast-radius X` |
| "Show the call stack around X" | `gql "MATCH (a)-[:CALLS*1..3]->(b) WHERE a.name = 'X'"` |
| "Where is ShoppingCart mutated?" | `cpg mutations --type ShoppingCart` |
| "Trace variable X forward" | `cpg flows FILE --line N --variable X --direction forward` |
| "Validate against policies" | `check --policy-file policy.json` |

The agent handles disambiguation (e.g., adding `--class` or `--file` when a symbol name is ambiguous) and error recovery (e.g., running `discover --with-cfg` if slicing fails because CFG data is missing).

## Install Options Reference

| Option | Default | Description |
|--------|---------|-------------|
| `--skill` | (required) | Install the rgctl agent skill |
| `--host` | `all` | Target: `all` (both), `claude`, or `cursor` |
| `--force` | off | Overwrite existing files that differ from the bundle |
| `-f json` | text | Structured JSON output with per-file status |

## How the Skill is Distributed

The skill bundle (`SKILL.md`, `references/`, …) is **compiled into the `rgctl` binary** at build time using Rust's `include_dir!` macro. This means:

- No network access needed to install.
- The skill version always matches the CLI version.
- Upgrading `rgctl` and running `install --skill --force` updates the skill.
- No dependency on the source repository being present.

## Benefits

- **Zero-configuration AI integration.** One command installs everything the agent needs to understand your codebase structurally.
- **Natural-language interface.** Developers ask questions in English; the agent translates to CLI commands.
- **Language-independent analysis.** Data-flow graphs, call neighborhoods, and mutation tracking work across all 9 Tier 1 languages.
- **Refactoring safety net.** Blast radius, call tracing, and mutation analysis catch breaking changes before they ship.
- **Test case generation.** CFG branch analysis and data dependencies produce higher-coverage tests.
- **Migration guidance.** The agent can generate, explain, and walk through a complete migration roadmap.
- **Cross-language porting.** Data-flow and dependency graphs provide a structural blueprint that transcends language syntax.
- **Always in sync.** The skill is embedded in the binary, so it always matches the CLI version.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- the `discover` step that all agent queries depend on
- [MCP Server](mcp-server.md) -- `serve --mode mcp` so the editor holds a warm session (query / search / impact / CPG without spawning CLI)
- [Blast Radius Analysis](blast-radius-analysis.md) -- the most common agent query for refactoring safety
- [Hybrid CPG](hybrid-cpg.md) -- mutations, flows, and call neighborhoods used in porting and testing
- [Program Slicing](program-slicing.md) -- statement-level analysis for data-flow tracing
- [Inspecting CFG, PDG, and Dominance](inspecting-cfg-pdg-dominance.md) -- the CFG data that drives test case generation
- [Migration Planning](migration-planning.md) -- the migration roadmap the agent can generate and explain
- [CI Policy Checks](ci-policy-checks.md) -- policy validation the agent runs for continuous architecture review

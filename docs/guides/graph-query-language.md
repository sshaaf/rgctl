# Graph Query Language

## Introduction

The `gql` command lets you query your code knowledge graph using a Cypher-like graph query language. Instead of reading source files or grepping for patterns, you write declarative `MATCH` queries that traverse nodes (functions, classes, modules, communities) and edges (calls, contains, imports) to answer structural questions about your codebase.

GQL is the primary interface for asking precise, composable questions: "What functions does this class contain?", "What calls this method?", "Which functions match this name pattern?", and many more.

## Use Cases

- **Inventory a codebase.** List all functions, classes, or communities without manually browsing directories.
- **Trace call chains.** Follow `CALLS` edges to see what a function depends on or what depends on it.
- **Find symbols by pattern.** Use `LIKE` filters to locate functions or classes matching a naming convention.
- **Understand structure.** Query `CONTAINS` edges to see which class owns which methods.
- **Power agent workflows.** Agents use GQL as their primary tool for answering structural questions cheaply.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` first:

```bash
rg-build -r example/coolstore discover .
```

## Step-by-Step

### 1. List All Functions (Macro)

rgBuilder ships built-in macros for common queries. The `all_functions` macro returns every function in the graph:

```bash
rg-build -r example/coolstore gql --macro-name all_functions unused
```

**Output (first 10 lines):**

```
anonymous
htmlParserImpl
Va
anonymous
ve
anonymous
d3_svg_symbolType
isMoment
Zr
unescape
```

The text format prints one function name per line. The argument `unused` is a placeholder required by the CLI syntax -- the macro ignores it.

### 2. JSON Output for Machine Consumption

Add `-f json` to get structured output with file paths, types, and qualified names:

```bash
rg-build -r example/coolstore -f json gql --macro-name all_functions unused
```

**Output (truncated):**

```json
{
  "count": 7526,
  "explain": false,
  "rows": [
    [
      {
        "binding": "f",
        "file": "example/coolstore/./src/main/webapp/bower_components/moment/src/lib/locale/ordinal.js",
        "node": "anonymous",
        "type": "Function"
      }
    ]
  ],
  "schema_version": 1
}
```

The `count` field tells you the graph contains **7,526 functions** total.

### 3. Find a Function by Name

Write a `MATCH` query with a `WHERE` clause to find a specific function:

```bash
rg-build -r example/coolstore -f json gql \
  "MATCH (f:Function) WHERE f.name = 'priceShoppingCart' RETURN f"
```

**Output:**

```json
{
  "count": 1,
  "explain": true,
  "rows": [
    [
      {
        "binding": "f",
        "file": "example/coolstore/./src/main/java/com/redhat/coolstore/service/ShoppingCartService.java",
        "node": "priceShoppingCart",
        "qualified_name": "com.redhat.coolstore.service.ShoppingCartService.priceShoppingCart",
        "type": "Function"
      }
    ]
  ],
  "schema_version": 1
}
```

This returns the exact function with its file path and fully qualified name.

### 4. Pattern Matching with LIKE

Use `LIKE` with wildcards (`%`) to find functions matching a pattern:

```bash
rg-build -r example/coolstore gql \
  "MATCH (f:Function) WHERE f.name LIKE '%Cart%' RETURN f.name, f.file LIMIT 15"
```

This finds all functions containing "Cart" in their name.

### 5. List All Classes

Query for `Class` nodes to see every class in the graph:

```bash
rg-build -r example/coolstore -f json gql \
  "MATCH (c:Class) RETURN c LIMIT 15"
```

**Output (truncated):**

```json
{
  "count": 15,
  "rows": [
    [
      {
        "binding": "c",
        "file": "example/coolstore/./src/main/java/com/redhat/coolstore/service/ShoppingCartService.java",
        "node": "ShoppingCartService",
        "qualified_name": "com.redhat.coolstore.service.ShoppingCartService",
        "type": "Class"
      }
    ],
    [
      {
        "binding": "c",
        "file": "example/coolstore/./src/main/java/com/redhat/coolstore/model/ShoppingCart.java",
        "node": "ShoppingCart",
        "qualified_name": "com.redhat.coolstore.model.ShoppingCart",
        "type": "Class"
      }
    ]
  ],
  "schema_version": 1
}
```

### 6. Find by Fully Qualified Name

Use `qualified_name` for precise lookups when a short name is ambiguous:

```bash
rg-build -r example/coolstore -f json gql \
  "MATCH (c:Class) WHERE c.qualified_name = 'com.redhat.coolstore.model.ShoppingCart' RETURN c"
```

### 7. List All Communities (Macro)

The `all_communities` macro returns detected functional clusters:

```bash
rg-build -r example/coolstore -f json gql --macro-name all_communities unused
```

**Output (truncated):**

```json
{
  "count": 10902,
  "rows": [
    [
      {
        "binding": "c",
        "community_id": 4391,
        "label": "bower_components.lodash (26)",
        "member_count": 120,
        "node": "bower_components.lodash (26)",
        "type": "Community"
      }
    ],
    [
      {
        "binding": "c",
        "community_id": 12715,
        "label": "coolstore.model::length",
        "member_count": 34,
        "type": "Community"
      }
    ]
  ],
  "schema_version": 1
}
```

### 8. Query Community Members

Find all functions in a specific community by its ID:

```bash
rg-build -r example/coolstore -f json gql \
  "MATCH (f:Function) WHERE f.community_id = '12715' RETURN f LIMIT 20"
```

### 9. Using --explain

Add the `--explain` flag to include extra metadata (like qualified names) in the response:

```bash
rg-build -r example/coolstore -f json gql --explain \
  "MATCH (f:Function) WHERE f.name = 'priceShoppingCart' RETURN f"
```

## GQL Syntax Reference

| Element | Syntax | Example |
|---------|--------|---------|
| Node match | `(alias:Type)` | `(f:Function)` |
| Edge match | `-[:EDGE_TYPE]->` | `-[:CALLS]->` |
| Multi-hop | `-[:EDGE_TYPE*1..N]->` | `-[:CALLS*1..3]->` |
| Filter | `WHERE alias.prop = 'value'` | `WHERE f.name = 'main'` |
| Pattern | `WHERE alias.prop LIKE '%pattern%'` | `WHERE f.name LIKE '%Service%'` |
| Limit | `LIMIT N` | `LIMIT 20` |
| Return | `RETURN alias` or `RETURN alias.prop` | `RETURN f.name, f.file` |

**Node types:** `Function`, `Class`, `Module`, `Community`

**Edge types:** `CALLS`, `CONTAINS`, `IMPORTS`, `REFERENCES`

**Built-in macros:** `all_functions`, `all_communities`, `direct_calls`, `call_chain`

## Benefits

- **Declarative and composable.** Write precise structural queries without imperative scripting.
- **Fast.** Queries run against the in-memory graph snapshot, returning results in milliseconds.
- **Multi-format output.** Use `-f text` for human reading, `-f json` for scripting and agent consumption.
- **Built-in macros.** Common queries like "list all functions" are one command away.
- **No source code reading required.** Agents and scripts can answer structural questions without loading files into context.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- build the graph that GQL queries
- [Blast Radius Analysis](blast-radius-analysis.md) -- higher-level impact analysis built on the same graph
- [Community Detection](community-detection.md) -- understand the communities that GQL can query
- [HTTP Server and Dashboard](http-server-and-dashboard.md) -- run GQL queries via HTTP API
- [MCP Server](mcp-server.md) -- IDE session; `rgbuilder_query` for MATCH / macros

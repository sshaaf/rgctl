# Community Detection

## Introduction

The `communities` command lets you **list and label functional clusters** that rgctl automatically detects in your codebase. During `discover`, the Louvain community detection algorithm partitions the call graph into groups of tightly connected functions. The `communities` command gives you tools to explore these clusters and generate human-readable labels for them.

Communities reveal the implicit architecture of your code -- groups of functions that work together closely even if they span multiple files or packages.

## Use Cases

- **Understand implicit architecture.** See how functions cluster together beyond the package/directory structure.
- **Identify cohesive modules.** Large communities with clear labels indicate well-defined functional domains.
- **Spot coupling problems.** An unexpectedly large community might indicate tight coupling between what should be separate modules.
- **Plan microservice extraction.** Communities map naturally to potential service boundaries.
- **Navigate unfamiliar code.** Browse communities to get a high-level map of what the codebase does.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` first:

```bash
rgctl -r example/coolstore discover .
```

## Step-by-Step

### 1. List Communities

List all detected communities, sorted by member count (largest first):

```bash
rgctl -r example/coolstore -f json communities list
```

**Output (truncated):**

```json
{
  "communities": [
    {
      "id": 4391,
      "label": "bower_components.lodash (26)",
      "member_count": 120
    },
    {
      "id": 14763,
      "label": "Infrastructure / Common Library",
      "member_count": 60
    },
    {
      "id": 10878,
      "label": "bower_components.angular (105)",
      "member_count": 53
    },
    {
      "id": 4983,
      "label": "Serializable",
      "member_count": 53
    },
    {
      "id": 12715,
      "label": "coolstore.model::length",
      "member_count": 34
    },
    {
      "id": 10882,
      "label": "coolstore.model::APPLICATION_JSON",
      "member_count": 30
    }
  ],
  "schema_version": 1
}
```

**What this tells you:**

- **`id`** -- the community's unique identifier, usable in GQL queries.
- **`label`** -- a heuristic label generated from the most representative member names and paths.
- **`member_count`** -- how many functions belong to this community.
- The largest community (120 members) is the lodash utility library. The second largest (60 members) is labeled "Infrastructure / Common Library" -- rgctl detected it as shared infrastructure code.
- The `coolstore.model` communities contain the domain model (entities, serialization).

### 2. Query Community Members with GQL

Once you have a community ID, use GQL to list its members:

```bash
rgctl -r example/coolstore -f json gql \
  "MATCH (f:Function) WHERE f.community_id = '12715' RETURN f LIMIT 20"
```

This returns all functions in the `coolstore.model::length` community (ID 12715), showing you which functions the algorithm grouped together.

### 3. Refresh Community Labels

If community labels are missing or stale, regenerate them:

```bash
rgctl -r example/coolstore communities label --write
```

The `--write` flag persists the updated labels into the analysis results, so subsequent `communities list` calls use the new labels.

### 4. Find Communities via GQL Macro

You can also list communities using the GQL macro:

```bash
rgctl -r example/coolstore -f json gql --macro-name all_communities unused
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
        "community_id": 14763,
        "label": "Infrastructure / Common Library",
        "member_count": 60,
        "type": "Community"
      }
    ]
  ],
  "schema_version": 1
}
```

This returns the same information as `communities list` but through the GQL interface, which allows additional filtering.

### 5. Semantic Search by Community

Use semantic search with community scope to find communities matching a concept:

```bash
rgctl -r example/coolstore semantic index
rgctl -r example/coolstore -f json semantic query "checkout" \
  --scope community --limit 10
```

This finds entire communities whose member functions are semantically related to "checkout".

## Understanding Community Labels

rgctl generates community labels heuristically from member names and file paths:

| Label Pattern | Meaning |
|---------------|---------|
| `coolstore.model::length` | Functions from the `coolstore.model` package, anchored by `length` |
| `Infrastructure / Common Library` | Cross-cutting utility functions without a dominant package |
| `bower_components.lodash (26)` | Functions from a third-party library, numbered for disambiguation |
| `coolstore.model::APPLICATION_JSON` | Domain model functions related to JSON serialization |

## Benefits

- **Automatic architecture map.** Community detection reveals the implicit structure of your code without any manual annotation.
- **Data-driven boundaries.** Communities are based on actual call relationships, not directory layout or naming conventions.
- **Microservice candidates.** Well-separated communities with clear labels are natural extraction targets.
- **Coupling visibility.** Large or oddly-named communities surface unexpected coupling between modules.
- **Composable with other commands.** Community IDs work in GQL queries, semantic search, and migration planning.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- `discover` runs community detection
- [Graph Metrics](graph-metrics.md) -- community modularity score via `metrics --communities`
- [Semantic Search](semantic-search.md) -- community-scoped semantic queries
- [Graph Query Language](graph-query-language.md) -- query community members with GQL
- [Migration Planning](migration-planning.md) -- communities define migration extraction steps

# Graph Metrics

## Introduction

The `metrics` command runs **network analytics** on your code knowledge graph: PageRank to find the most important functions, betweenness centrality to identify bridging nodes, and community detection to measure the modularity of your codebase. These are the same algorithms used to analyze web link graphs and social networks, applied to the structure of your code.

Metrics give you a quantitative view of your codebase's architecture -- which functions are critical hubs, which serve as bridges between modules, and how well the code separates into distinct clusters.

## Use Cases

- **Identify critical functions.** PageRank reveals the most structurally important functions -- the ones most depended upon.
- **Find architectural bottlenecks.** High betweenness centrality indicates functions that bridge otherwise disconnected parts of the codebase.
- **Measure modularity.** Community detection tells you how well your code separates into cohesive groups, quantified by a modularity score.
- **Prioritize refactoring.** Focus effort on high-PageRank, high-betweenness functions where changes have the largest structural impact.
- **Track architecture over time.** Run metrics on each release to detect modularity drift or emerging hotspots.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` first:

```bash
rgctl -r example/coolstore discover
```

## Step-by-Step

### 1. PageRank

PageRank ranks functions by structural importance -- how many other functions depend on them, directly or transitively. Functions called by many high-PageRank callers themselves receive a high score.

```bash
rgctl -r example/coolstore -f json metrics --pagerank
```

**Output (truncated):**

```json
{
  "pagerank": {
    "converged": false,
    "iterations": 20,
    "max_delta": 0.00028581400200106133,
    "top": [
      {
        "node": "b979f32a-ace5-4f25-b32f-4b5b66d73358",
        "pagerank": 0.0566426911008737
      },
      {
        "node": "dba829f7-ca5e-42b8-b25d-4c5b75583ef6",
        "pagerank": 0.021913956508072173
      },
      {
        "node": "220c19be-4c4f-4ecf-b1fe-71c8f536d59b",
        "pagerank": 0.005028400583110091
      }
    ]
  },
  "schema_version": 1
}
```

**What this tells you:**

- **`converged: false`** -- the algorithm ran for 20 iterations but did not fully converge (the `max_delta` is still above zero). For most practical purposes, the ranking is stable after 20 iterations.
- **`top`** -- the highest-ranked functions by PageRank score. The top function (`0.0567`) has nearly 3x the score of the second (`0.0219`), indicating it is a dominant structural hub.
- **`node`** -- the UUID of each function. Use GQL to resolve these to human-readable names.

To increase iterations for better convergence:

```bash
rgctl -r example/coolstore -f json metrics --pagerank --iterations 50
```

### 2. Betweenness Centrality

Betweenness centrality measures how often a function lies on the shortest path between two other functions. High-betweenness functions are architectural bridges -- removing them would disconnect parts of the call graph.

```bash
rgctl -r example/coolstore -f json metrics --betweenness
```

**Output (truncated):**

```json
{
  "betweenness": [
    {
      "node": "778cc6fc-8ae3-40c8-9137-82e040e5b5d1",
      "score": 0.000021622996659875876
    },
    {
      "node": "7a576df2-e51b-405e-b5a1-6e14bca1c4c1",
      "score": 0.000021567926001973067
    }
  ],
  "schema_version": 1
}
```

**What this tells you:**

- Functions with the highest betweenness scores are the most critical bridges in the call graph.
- These are the functions where a bug or breaking change would propagate most widely across otherwise separate modules.
- Low betweenness means a function is "internal" to a single cluster.

### 3. Community Detection

Community detection partitions the graph into clusters of tightly connected functions using the Louvain algorithm. The `modularity` score (0--1) measures how well the code separates into distinct groups.

```bash
rgctl -r example/coolstore -f json metrics --communities
```

**Output:**

```json
{
  "communities": {
    "assignments": 14763,
    "count": 11303,
    "modularity": 0.3076728222682732
  },
  "schema_version": 1
}
```

**What this tells you:**

- **`count: 11303`** -- the algorithm identified 11,303 distinct communities.
- **`assignments: 14763`** -- all 14,763 nodes in the graph were assigned to a community.
- **`modularity: 0.3077`** -- a modularity score of ~0.31. Values above 0.3 indicate meaningful community structure; values above 0.5 indicate strong modularity. The CoolStore application has moderate modularity, consistent with its nature as a single-deployment application.

### 4. Combining Metrics

You can run multiple metrics in a single command:

```bash
rgctl -r example/coolstore -f json metrics --pagerank --betweenness --communities
```

This returns all three analyses in a single JSON response.

### 5. Resolving Node UUIDs

The metrics output uses node UUIDs. To find out which function a UUID refers to, query the graph:

```bash
rgctl -r example/coolstore -f json gql \
  "MATCH (f:Function) WHERE f.name = 'priceShoppingCart' RETURN f"
```

Or use `blast-radius` which resolves names automatically:

```bash
rgctl -r example/coolstore blast-radius priceShoppingCart
```

## Understanding the Algorithms

| Metric | What It Measures | High Score Means |
|--------|-----------------|-----------------|
| **PageRank** | Recursive importance via incoming edges | Many important callers depend on this function |
| **Betweenness** | Frequency on shortest paths between pairs | This function bridges otherwise disconnected modules |
| **Community modularity** | Quality of graph partitioning | The codebase has strong, well-separated functional clusters |

## Benefits

- **Quantitative architecture analysis.** Replace subjective assessments with concrete scores.
- **Identify hotspots.** High-PageRank and high-betweenness functions are where bugs hurt most and refactoring pays off most.
- **Measure modularity.** Track whether your codebase is becoming more or less modular over time.
- **Standard algorithms.** PageRank, betweenness centrality, and Louvain community detection are well-understood network science tools.
- **Fast.** All metrics run in-memory on the graph snapshot, typically completing in under a second.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- must run `discover` before metrics
- [Community Detection](community-detection.md) -- dig deeper into community analysis
- [Blast Radius Analysis](blast-radius-analysis.md) -- per-function impact analysis that complements metrics
- [Migration Planning](migration-planning.md) -- metrics feed into migration ordering

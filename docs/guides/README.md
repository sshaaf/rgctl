# rgctl Guides

Practical, step-by-step guides for every major rgctl feature. Each guide uses the **CoolStore** application (`example/coolstore`) as a running example so you can follow along on a real Java EE codebase.

## Guides

| Guide | Feature | Description |
|-------|---------|-------------|
| [Discovering and Indexing a Codebase](discovering-and-indexing.md) | `discover` | Build the knowledge graph from source code |
| [Graph Query Language](graph-query-language.md) | `gql` | Query the code graph with Cypher-like syntax |
| [Blast Radius Analysis](blast-radius-analysis.md) | `blast-radius` | Measure upstream impact before changing a function |
| [Semantic Search](semantic-search.md) | `semantic` | Natural-language search over function symbols |
| [Graph Metrics](graph-metrics.md) | `metrics` | PageRank, betweenness, and community detection analytics |
| [Community Detection](community-detection.md) | `communities` | Identify and label functional clusters in your codebase |
| [Program Slicing](program-slicing.md) | `slice` | Extract the minimal set of statements affecting a variable |
| [Hybrid CPG](hybrid-cpg.md) | `cpg` | Combined call-graph + per-function CFG/PDG analysis |
| [Inspecting CFG, PDG, and Dominance](inspecting-cfg-pdg-dominance.md) | `inspect` | Examine low-level control flow, data dependence, and dominator trees |
| [Exporting Graphs](exporting-graphs.md) | `export` | Serialize graph data to JSON, GraphML, Graphviz, Mermaid, or Obsidian |
| [Markdown Context Graph](markdown-context-graph.md) | `discover -l markdown` · `export` | Index docs, Obsidian/OKF export, fixture feature tour (k8s-website scale example) |
| [CI Policy Checks](ci-policy-checks.md) | `check` | Enforce architectural rules in your CI pipeline |
| [HTTP Server and Dashboard](http-server-and-dashboard.md) | `serve` | Run an HTTP API and browser-based dashboard |
| [MCP Server](mcp-server.md) | `serve --mode mcp` | stdio MCP for Cursor / Claude Code (seven tools + auto full pipeline) |
| [Migration Planning](migration-planning.md) | `discover --export-migration-hints` | Generate a dependency-aware migration roadmap |
| [Agent Skill](agent-skill.md) | `install --skill` | Teach AI agents to use rgctl for refactoring, migration, porting, and testing |

## Prerequisites

All guides assume you have rgctl installed. See the [Installation guide](../installation.md) to get started.

To follow the examples, clone the repository and navigate to the example project:

```bash
git clone https://github.com/konveyor-ecosystem/coolstore.git
cd coolstore
```

## Related Resources

- [User Guide](../user-guide.md) -- full CLI reference and tutorial
- [JSON API Reference](../json-api.md) -- schema details for `-f json` output
- [Agent Recipes](../agent-recipes.md) -- copy-paste recipes for LLM agents
- [Glossary](../glossary.md) -- definitions of key terms

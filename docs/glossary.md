# Glossary

| Term | Definition |
|------|------------|
| **Blast radius** | Upstream call-graph impact of changing a symbol (callers, score, impact zone). |
| **Community** | Densely connected cluster from **label propagation**; named via `communities` / GQL `:Community`. |
| **CPG** | Code Property Graph — hybrid of L_repo (CALL/type) and L_proc (CFG/PDG). CLI: `cpg`. |
| **CFG** | Control-flow graph of a function (basic blocks and branches). |
| **Discover** | Index a repository into `.rgctl/` (graph snapshot + analytics). |
| **Fusion** | Late re-ranking of semantic hits with graph signals (blast, PageRank, sketches). |
| **GQL** | rgctl graph query language (`MATCH` / macros) over the knowledge graph. |
| **Hamming distance** | Bitwise distance used for packed semantic embedding retrieval. |
| **Harmonic centrality** | Reachability closeness metric; opt-in via `--with-harmonic` for migration ranking. |
| **L_proc** | Per-function procedural layer (CFG/PDG archive under `.rgctl/analysis/`). |
| **L_repo** | Repository-level topology (functions, CALL, types) in the graph snapshot. |
| **Louvain (field name)** | Historical name for community id (`louvain_community_id`); algorithm is label propagation. |
| **MRL** | Multi-representation learning ideas behind fused semantic ranking (see design docs). |
| **PDG** | Program dependence graph — data and control deps between statements. |
| **Schema version** | Integer on `-f json` payloads; bump means breaking field changes. |
| **Semantic index** | Opt-in function embedding store (`.rgctl/semantic_index.bin`). |
| **Slice** | Statements that affect / are affected by a line+variable (or taint trace). |
| **Tier 1** | Nine always-linked languages: Rust, Python, JS, TS, Go, Java, C#, C, C++. |

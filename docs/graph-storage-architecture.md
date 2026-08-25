# rgctl-graph Storage Architecture

The `rgctl-graph` crate stores the code knowledge graph: typed nodes and edges, secondary indexes, persistence, and a mini query language. Analysis algorithms live in `rgctl-analysis`.

## Layers

| Layer | Module | Role |
|-------|--------|------|
| Schema | `schema.rs` | `Node`, `Edge`, `NodeType`, `EdgeType` |
| Backend | `backend/memory.rs` | `MemoryBackend` — RwLock maps + indexes |
| Interning | `intern.rs` | `Arc<str>` dedup for index keys |
| Query | `query.rs` | Filter language (`type:Function`, compound `\|`) |
| Persistence | `snapshot.rs`, `columnar_snapshot.rs` | v1 bincode / v2 columnar mmap |
| API | `code_graph.rs` | `CodeGraph` wrapper |

## MemoryBackend

- **Nodes:** `HashMap<Uuid, Node>` under `RwLock`
- **Edges:** `Vec<Edge>` under `RwLock`
- **Indexes:** name, type, label, property, edge-type
- **Query cache:** invalidated on every mutation

Zero-clone APIs for analysis: `edge_topology_typed()`, `for_each_node`, `find_node_ids_by_*`.

## Snapshots

| Format | Version | Open cost |
|--------|---------|-----------|
| JSON | legacy | Full parse |
| bincode mmap | v1 | Deserialize payload |
| columnar mmap | v2 | Header + indexes only |

v2 uses fixed-width rows (64 B node, 40 B edge) + string pool. BLAKE3 `content_digest` enables cache invalidation.

## SQLite (blast-radius only)

rgctl does **not** store the knowledge graph in SQLite. The only SQLite usage is `macro_call_index.db` in `rgctl-analysis` — a precomputed **blast-radius lookup cache** written at `discover` and read by `blast-radius` T0 (`MacroCallLookupDb`). GQL, export, and metrics use the columnar mmap graph.

## Downstream

```
MemoryBackend / ColumnarGraphMmap → rgctl-analysis::PetGraphView → algorithms
```

See also: [analysis-architecture.md](analysis-architecture.md), [cli-io-sanity-qe.md](cli-io-sanity-qe.md).

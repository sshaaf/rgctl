# Graph snapshot diff (read path)

Structural comparison between two columnar v2 snapshots for PR gates.

## Layers

| Layer | Module | Role |
|-------|--------|------|
| L0 | `snapshot_diff::SnapshotPair` | Parallel open; digest fast path |
| L1 | `stable_key::StableNodeKey` | BLAKE3(path, name, type) — UUID-independent |
| L2 | `snapshot_diff` | `DiffSink`, node hash index, edge merge-join on stable keys |
| L3 | `rgctl_incremental::pr_scope` | Git `name-status` paths + hunk line overlap |
| L3b | `rgctl_incremental::cascade` | Reverse `Calls` expansion for caller re-extract |
| L4 | `pr-check` CLI | Temporal policy on scoped head entities |
| L5 | `policy_diff` | NEW / EXISTING / RESOLVED classification |
| L6 | `check --base-ref/--head-ref/--strict` | Working-tree / PR diff scoping fixes |

## Hot-path rules

- Mmap-only string pool reads via `string_at` (no `String` alloc on keys).
- Edge diff: map UUID → `StableNodeKey`, sort stable edge keys, merge-join (not `edge_meta: Vec` collect).
- Node index on head: rayon shard by `idx % 16`, merge disjoint maps.
- Sync CPU; async hosts use `spawn_blocking` at the boundary only.

## Benchmarks

```bash
cargo bench -p rgctl-graph --bench snapshot_diff
```

Groups: `digest_fast_path_equal`, `node_index_parallel`, `edge_merge_join`, `full_diff_noop_sink`.

## Reverse-dependency cascade

When a callee file changes (especially renames), call sites in untouched caller files must be
re-parsed so `Calls` edges target the new stable node IDs.

1. `ChangeSet.invalidation_paths()` yields modified/deleted/rename-old paths.
2. `incoming_callers_files_depth(base_mmap, seeds, N)` scans incoming `Calls` edges into nodes
   declared in those files and returns caller declaring files.
3. `IncrementalUpdater` unions cascaded paths with `extract_paths()` before delta extract +
   `rebuild_relations`.

`--cascade-depth` (default **1**, `0` = disabled) on incremental update and `pr-check` (used when
delta head synthesis lands in Phase 4).

## Scoped policy analysis (Phase 5)

`pr-check` no longer dual-hydrates full graphs for blast/centrality:

1. Resolve scoped entity UUIDs from `StableNodeKey` on each snapshot.
2. `collect_upstream_call_closure` → mmap `Calls` reverse BFS.
3. `hydrate_subset` → small `MemoryBackend` for policy node lookups.
4. `BlastRadiusEngine::build_scoped` on the upstream closure (parity with full graph for scoped seeds).
5. Centrality: reuse `.rgctl/analysis_results.bin` when node count matches; else `analyze_scoped` on seed entities only.

# Go Tier-1 completion plan (#46)

**Status:** Phase 0–3 done; high-impact Go CFG lowering landed (if/switch init, `for_clause`, switch case bodies). Remaining: fallthrough/goto/short-circuit/defer-unwind; Phase 4 polish.  
**Coverage map:** [go-language-coverage.md](./go-language-coverage.md)  
**Issue:** https://github.com/sshaaf/rgctl/issues/46

## Progress (2026-07-24)

| Item | State |
|------|--------|
| Coverage doc LF-01…LF-21 | done |
| `internal/langfeatures/` fixtures | done |
| `lf_*` expected-facts + `tests/go_langfeatures.rs` | done |
| `field_identifier` call extraction | done |
| Receiver FQN + type hints | done |
| Interface methods + `type_elem` embed promotion | done |
| Cross-file field-type late bind (`field_type_index`) | done |
| `var_spec` def-use + switch/select complexity | done |
| Struct anonymous embed fields | done |
| Kubernetes `createPodSandbox → RunPodSandbox` | **verified** |
| `IMPLEMENTS` (method-set) + embed `EXTENDS` | done |
| Import / const / TypeAlias / generics metadata | done |
| Tier-1 doc A6 “optional for Go” removal | done |
| Go CFG: if/switch initializer before condition | done |
| Go CFG: `for_clause` init/cond/update + `continue`→update | done |
| Go CFG: switch/select case `statement_list` + Return edges | done |
| Go CFG: `fallthrough`, `goto`/labels, `&&`/`\|\|` short-circuit | done |
| Go CFG: labeled break/continue, defer/panic unwind | done |
## Goals

1. Usable Go call graphs for idiomatic code (methods, interfaces, embeds).
2. No Tier-1 language surface silently optional — document honesty limits only where analysis is fundamentally undecidable.
3. Correctness enforced by `graph_correctness` on `ecommerce-go` (`lf_*` facts).

## Phase 0 — Spec & fixtures (this PR track)

| Task | Deliverable | Done when |
|------|-------------|-----------|
| 0.1 Coverage document | `docs/design/go-language-coverage.md` | Feature IDs LF-01…LF-21 |
| 0.2 Fixture package | `rgctl-tests/ecommerce-go/internal/langfeatures/` | Compiles; discover indexes symbols |
| 0.3 Expected facts | `lf_*` entries in `expected-facts.json` | `cargo test --test graph_correctness go` exercises them |
| 0.4 Plan + issue update | this doc + #46 | Linked from issue body |

## Phase 1 — P0 call graph (unblocks kubelet-style paths)

| Task | Change | Unlocks |
|------|--------|---------|
| 1.1 | `callee_name`: accept `field_identifier` | LF-02, LF-03, LF-04 extraction |
| 1.2 | Go methods: receiver type → `qualified_name` (`Type.Method`) + metadata | LF-02, LF-03, LF-18 browseability |
| 1.3 | Call relations: set `to_type_hint` / `to_qualified_hint` from receiver/local types (best-effort) | Cross-file same-name resolution |
| 1.4 | Unit tests in `rgctl-lang-go` for selector + collision | Prevents silent regression |
| 1.5 | Green LF-01…LF-03 (and LF-18) in graph_correctness | Phase 1 exit |

## Phase 2 — P0 dataflow / metrics / embedding fields

| Task | Change | Unlocks |
|------|--------|---------|
| 2.1 | `def_use`: walk `var_spec` under `var_declaration` | LF-08 |
| 2.2 | Complexity: real switch/select node kinds + cases | LF-11…LF-13 |
| 2.3 | Struct embed: record anonymous fields; emit embed relation | LF-06, LF-07 |
| 2.4 | Green LF-06…LF-09, LF-11…LF-13 | Phase 2 exit |

## Phase 3 — P1 interfaces, imports, types

| Task | Change | Unlocks |
|------|--------|---------|
| 3.1 | Extract interface `method_elem` as methods / signatures | LF-04 contract |
| 3.2 | Best-effort `IMPLEMENTS` (method-set satisfaction) | LF-05 |
| 3.3 | Interface call → candidate impls (multi-edge or ranked) | LF-04 |
| 3.4 | Import symbols / IMPORTS edges | LF-17 |
| 3.5 | Const, package var, type alias symbols | LF-10 |
| 3.6 | Generics: retain type param metadata; call name resolve | LF-16 |
| 3.7 | Green LF-04, LF-05, LF-10, LF-16, LF-17 | Phase 3 exit |

## Phase 4 — CPG / CFG polish / docs

| Task | Change | Unlocks |
|------|--------|---------|
| 4.1 | Receiver field-write golden (not only free func) | LF-19 |
| 4.2 | `defer` / `go` documented CFG semantics; call from `go f()` | LF-14, LF-15 |
| 4.2b | High-impact CFG: if/switch init, `for_clause`, case body lowering | done — see coverage “Go CFG lowering” |
| 4.2c | Labeled break/continue, defer/panic unwind | done |
| 4.3 | Struct tags + multi-return (best_effort → required if cheap) | LF-20, LF-21 |
| 4.4 | `docs/tier-1-language-support.md`: remove “optional for Go” on A6; point here | Policy |
| 4.5 | Dashboard gate asserts min `calls` among langfeatures | CI |

## Non-goals (honesty)

- Full points-to / reflection / `any` dynamic dispatch certainty
- Cross-goroutine channel taint (may stay sequential CFG forever; must be documented)

## Exit criteria for #46

- All **required** `lf_*` facts green in `graph_correctness` for go
- Kubernetes spot-check: `createPodSandbox` has `CALLS` to `RunPodSandbox` (name-level); blast-radius on `SyncPod` non-empty callees via GQL
- Tier-1 doc updated; coverage matrix rows LF-01…LF-19 required ✅

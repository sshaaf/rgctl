# Go language feature coverage (ecommerce-go)

**Purpose:** Canonical checklist of Go language surfaces that rgctl must index and analyze correctly. Fixture code lives under `rgctl-tests/ecommerce-go/internal/langfeatures/`. Expected graph facts live in `ecommerce-go/correctness/expected-facts.json` (ids prefixed `lf_`).

**Tracking:** [sshaaf/rgctl#46](https://github.com/sshaaf/rgctl/issues/46) · plan: [go-tier1-completion-plan.md](./go-tier1-completion-plan.md)

**Honesty limits (not “optional gaps”):** no full points-to across reflection/`any` casts; ambiguous multi-impl interface edges may be multi-target or annotated dynamic — but must not be silently dropped.

---

## Feature matrix

| ID | Language feature | Fixture file | Probe symbols (names) | Expected graph / analysis | Severity |
|----|------------------|--------------|------------------------|---------------------------|----------|
| LF-01 | Package function call | `calls_basic.go` | `LfPkgCaller` → `LfPkgCallee` | `CALLS` edge | required |
| LF-02 | Method call same type | `methods.go` | `(*LfCart).Checkout` → `(*LfCart).validate` | `CALLS`; receiver FQN `LfCart.Checkout` | required |
| LF-03 | Method call cross-type same name | `methods.go` | `(*LfOrchestrator).Run` → `(*LfBetaStore).ListItems` (not `LfAlphaStore.ListItems`) | Correct resolution under name collision | required |
| LF-04 | Interface method call | `interfaces.go` | `(*LfRuntimeClient).Start` → `RunSandbox` via `LfRuntime` | `CALLS` to impl(s); interface method symbol exists | required |
| LF-05 | Multiple interface implementations | `interfaces.go` | `LfRemoteRuntime.RunSandbox`, `LfFakeRuntime.RunSandbox` | Both method symbols; implements / satisfaction link to `LfRuntime` | required |
| LF-06 | Struct embedding (anonymous field) | `embedding.go` | `LfDerived` embeds `LfBase` | Embedded field in `fields[]`; embed/`EXTENDS`-like or `CONTAINS` relation | required |
| LF-07 | Promoted method via embed | `embedding.go` | `(*LfDerived).UseBase` → `(*LfBase).BaseMethod` | `CALLS` to promoted / base method | required |
| LF-08 | `var` declaration def-use | `vars.go` | `LfVarFlow` | PDG/def-use sees `var x int` / `var s string` defs | required |
| LF-09 | Short var `:=` | `vars.go` | `LfShortVarFlow` | def-use for `:=` (already partially covered by harness) | required |
| LF-10 | Const + typed iota / alias | `vars.go` | `LfStatus`, `LfStatusPending`, type alias `LfUserID` | TypeAlias / const or Variable symbols | required |
| LF-11 | Expression switch | `control.go` | `LfExprSwitch` | Complexity counts cases; CFG lowers case `statement_list` (Return edges from case bodies) | required |
| LF-12 | Type switch | `control.go` | `LfTypeSwitch` | CFG/complexity include `type_case`; case bodies lowered | required |
| LF-13 | `select` + channel | `control.go` | `LfSelectLoop` | CFG select arms via same case lowering; complexity counts `communication_case` | required |
| LF-14 | `defer` | `control.go` | `LfWithDefer` | CFG records defer; return/panic route through defer chain LIFO | required |
| LF-15 | `go` statement | `control.go` | `LfSpawn` | CFG records go stmt; call edge to spawned func name | required |
| LF-16 | Generics (func + type) | `generics.go` | `LfIdentity[T]`, `LfBox[T]` | Symbols retained; calls to `LfIdentity` resolve | required |
| LF-17 | Imports | `imports_probe.go` | import of `fmt` / internal pkg | `Import` symbols or `IMPORTS` edges | required |
| LF-18 | Constructor `NewT` | `methods.go` | `NewLfCart` | `LfCart.<init>`, `is_constructor` | required |
| LF-19 | Receiver field write (CPG) | `fieldwrite.go` | `(*LfOrderDTO).MarkProcessed` writes `Status` | `cpg mutations` / field-write index hit with `--exclude-ctors` | required |
| LF-20 | Multi-value return | `methods.go` | `(*LfCart).Totals` | Structured or preserved return signature string | best_effort |
| LF-21 | Struct tags | `fieldwrite.go` | `LfOrderDTO.Status` `json:"status"` | Tag in field metadata | best_effort |

---

## How to re-verify

```bash
# From rgctl repo root
cargo build --release
./target/release/rgctl discover rgctl-tests/ecommerce-go -l go -e vendor \
  --with-cfg --with-taint -v

# Correctness suite (includes lf_* facts once present)
cargo test --test graph_correctness go -- --nocapture

# Spot-check call edges
./target/release/rgctl -r rgctl-tests/ecommerce-go -f json \
  gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'Run' RETURN a,b"
```

When adding a new Go surface, **add a row here**, a fixture symbol, and an `lf_*` expected-fact before claiming support.

---

## Go CFG lowering (Tree-sitter)

Shared builder: `crates/rgctl-analysis/src/cfg_builder.rs`. Unit tests: `cfg_builder::tests::test_go_*`.

| Construct | Lowering | Status |
|-----------|----------|--------|
| Straight-line (`var` / `:=` / assign / call / `i++` / send) | Basic-block statements | done |
| `if` + `else` / `else if` | Condition Branch + `IfTrue`/`IfFalse` | done |
| `if init; cond` | Init statement **before** condition block | done |
| `for` three-part (`for_clause`) | Init → cond header → body → **update** → cond; `continue` → update | done |
| `for` while-style / infinite / `range` | Header + body cycle (range text on header) | done (coarse range) |
| Expr / type `switch` + `select` | Fan-out; case **`statement_list` lowered** (returns emit `Return`) | done |
| `switch init; …` | Init before branch | done |
| Unlabeled `break` / `continue` | Exit / continue-target (innermost for/switch/select) | done |
| Labeled `break` / `continue` | Jump to labeled loop/switch exit or continue-target | done |
| `go` | Expression in BB; no parallel CFG fork | approx (LF-15) |
| `defer` | Register on stack; return/panic unwind LIFO as FunctionCall blocks | done |
| `fallthrough` | Jump from case body into next case body | done |
| `goto` / `labeled_statement` | Eager label blocks; forward/back `goto` → Jump | done |
| `&&` / `\|\|` short-circuit | Condition lowered into chained IfTrue/IfFalse | done |
| `panic` | Call + unwind through active defers (Exception terminal) | done |

Honesty: `go` does not fork a parallel CFG; defer is a static stack (loop-deferred multiplicity not modeled).

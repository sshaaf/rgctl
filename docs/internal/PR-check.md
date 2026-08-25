# PR Check Template

Use this template to run a consistent, merge-readiness review for rgBuilder PRs.

## 0) PR context

- PR title/link:
- Branch:
- Reviewer:
- Date:
- Scope summary (1-3 bullets):
- Risk level (`low` / `medium` / `high`):

## 1) Pre-flight and baseline

- [ ] Pull latest branch and sync local state.
- [ ] Confirm no unintended local changes will pollute results.
- [ ] Identify comparison baseline (e.g. `v0.4.4`, `main`, prior PR run).
- [ ] Define target datasets/fixtures (linux, markdown corpus, mixed fixture, etc.).
- [ ] If large example repos are missing under `example/`, run:
  - `./scripts/fetch-profile-repos.sh`
  - This fetches: `linux`, `kafka`, `metasfresh-4.9.8b`, `coolstore-weblogic`, `kubernetes`, `k8s-website`.

Evidence:

- Baseline ref:
- Commands used:

## 2) Build and compile health

- [ ] Build release binary:
  - `cargo build --release --bin rgctl`
- [ ] Run crate tests for touched areas (examples):
  - `cargo test -p rgbuilder-analysis`
  - `cargo test -p rgbuilder-extraction`
  - `cargo test -p rgbuilder-pipeline`
  - `cargo test -p rgbuilder-incremental`

Evidence:

- Failures found:
- Fixes applied:
- Final pass status:

## 3) Reliability checks (must be fail-loud)

- [ ] Confirm extraction failures are surfaced (not silently dropped).
- [ ] Verify discover metrics are internally consistent:
  - `files_discovered`
  - `files_processed`
  - `files_failed`
- [ ] Add/verify tests that assert failure accounting behavior.

Evidence:

- Tests:
- Metrics sample:

## 4) Functionality regression checks

- [ ] Confirm no accidental functionality loss in core behavior.
- [ ] Validate new/changed behaviors with targeted tests.
- [ ] Check cross-feature consistency (graph, blast, semantic, CFG agreement):
  - `cargo test --test cross_feature_qe`
- [ ] Check CLI behavior for affected features:
  - `cargo test --test markdown_context_cli`

Evidence:

- Regressions found:
- Resolved?:

## 5) Data structure and algorithm review

- [ ] Review hotspot structures for avoidable O(n^2)/allocation-heavy behavior.
- [ ] Confirm chosen structures fit access patterns (maps/sets/vectors/indices).
- [ ] Document any bounded/index fan-out decisions and trade-offs.
- [ ] Add regression tests for ambiguity and edge cases.

Evidence:

- Hot path files/functions:
- Changes made:
- Test additions:

## 6) Parallelism / async / contention review

- [ ] Confirm CPU-bound work uses safe parallel strategy where beneficial.
- [ ] Check for hidden contention (locks, channels, queue pressure, serial bottlenecks).
- [ ] Validate deterministic behavior under parallel execution.
- [ ] Record opportunities not implemented in this PR.

Evidence:

- Current parallel model:
- Contention risks:
- Follow-ups:

## 7) Performance profiling (cold + A/B)

- [ ] **Cold profile definition:** rebuild the release binary immediately before profiling:
  - `cargo build --release --bin rgctl`
  - Use the freshly built `target/release/rgctl` only; do not use debug/stale binaries.
- [ ] Run cold profiles with clean `.rgbuilder` per run.
- [ ] Use at least 3 runs and compare **median**.
- [ ] Capture:
  - wall time
  - peak RSS
  - key stage timings (`index_extract`, `index_graph_build`, `community`, etc.)
- [ ] Compare against baseline branch/tag.
- [ ] Call out deltas and whether they are acceptable.

Recommended command pattern:

- `RUST_LOG=info,profile=info target/release/rgctl -r <repo> -f json discover . -v`

Evidence table:

| Case | Baseline median wall | PR median wall | Delta | Decision |
|---|---:|---:|---:|---|
| linux |  |  |  |  |
| k8s markdown |  |  |  |  |
| mixed fixture |  |  |  |  |

## 8) Gate checks

- [ ] Run targeted gate tests relevant to changes.
- [ ] For cold profile gates, run sequentially to avoid host contention when needed.
- [ ] Record any gate failures and resolution.

Example:

- `cargo test --release --test cold_profile_gates -- --ignored --nocapture --test-threads=1 <gate_name>`

Evidence:

- Gates run:
- Status:

## 9) Documentation and spec alignment

- [ ] Confirm docs reflect behavior/performance changes.
- [ ] Update user/developer guidance where operational semantics changed.
- [ ] Ensure OpenSpec artifacts are updated (proposal/design/specs/tasks) if used.

Evidence:

- Docs updated:
- Spec/tasks updated:

## 10) Final review output

- [ ] Summarize fixed issues.
- [ ] Summarize residual risks/hotspots.
- [ ] Provide concrete follow-up plan for deferred items.
- [ ] Provide merge recommendation (`approve` / `approve with follow-ups` / `request changes`).

Final summary template:

```text
PR Review Outcome:
- Build/test reliability:
- Functional correctness:
- Performance impact:
- Residual risks:
- Recommendation:
```

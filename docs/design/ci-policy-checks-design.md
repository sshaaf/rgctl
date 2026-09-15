# CI Policy Checks — Engineering Design

**`rgctl check`** — fail CI when blast-radius policy rules are violated on symbols touched in the current git working tree. Complements interactive **`blast-radius --policy-file`** gatekeeping.

![Blast scores that feed policy thresholds (gbuilder)](../images/design/ci-policy-checks/policy-blast-scores.png)

*Figure 1: **Blast Radius** tab — impact scores and caller fan-in that inform policy thresholds. Policy enforcement itself runs in CI via `check` (CLI); there is no separate dashboard tab.*

---

## 1. Goals

| Goal | How |
|------|-----|
| Block risky merges | Exit code `1` when violations found |
| Scope to changes | `git diff` changed functions (fallback: all functions) |
| Reuse blast engine | Same `BlastRadiusEngine` + centrality as `blast-radius` |
| Declarative rules | JSON policy file — see [policy-format.md](../policy-format.md) |

---

## 2. Architecture overview

```mermaid
flowchart TB
  subgraph inputs["Inputs"]
    POL[policy.json]
    GIT[git changed symbols]
    G[graph + analysis_results]
  end

  subgraph check["rgctl check"]
    RES[resolve_unique_symbol]
    ENG[BlastRadiusEngine.analyze]
    POLCHK[analyze_with_policy]
    CASCADE[cascade hazard vs betweenness threshold]
    RES --> ENG --> POLCHK --> CASCADE
  end

  subgraph output["Output"]
    JSON["-f json: passed, violations[]"]
    EXIT[exit 0 / 1]
  end

  POL --> check
  GIT --> check
  G --> check
  check --> JSON
  check --> EXIT
```

**`blast-radius --policy-file`:** evaluates policy on a **single** symbol; emits `gatekeeping.policy_status` (`PASSED` / `VIOLATED` / `SKIPPED`) and may exit `1` after printing JSON.

---

## 3. Policy rule types

| Rule | Trigger |
|------|---------|
| `max_blast_score` | Target impact score exceeds cap |
| `max_impact_zone_size` | Transitive caller count too large |
| `centrality_alert_threshold` | Upstream node betweenness in impact zone |
| Custom registry entries | `PolicyFile` → `PolicyRegistry` |

Example files: [policy-permissive.json](../examples/policy-permissive.json), [policy-strict.json](../examples/policy-strict.json).

---

## 4. Rust implementation map

| Component | Path |
|-----------|------|
| `check` command | `src/cli/check.rs` |
| JSON output | `src/cli/check_output.rs` |
| Policy load | `src/cli/policy_file.rs` |
| Engine | `crates/rgctl-analysis/src/blast_radius_scc.rs` |
| Centrality | `crates/rgctl-analysis/src/centrality.rs` |
| Git diff symbols | `src/cli/check.rs` (`changed_function_symbols`) |

---

## 5. Dashboard relationship

Policy is **CLI-first**. The dashboard helps **calibrate** thresholds:

- **Blast Radius** tab — empirical scores and caller counts
- **Functions** tab — betweenness / PageRank for cascade hazard tuning
- **Migration** tab — package risk context (orthogonal to per-PR `check`)

---

## 6. CLI usage

```bash
# One-off blast with policy gate
rgctl -f json blast-radius ShoppingCartService --policy-file policy.json

# CI on PR — evaluate changed functions
rgctl check --policy-file policy.json
rgctl -f json check --policy-file policy.json | jq '.passed, .violations'
```

Typical GitHub Actions pattern: run `discover` in a setup job, then `check` on each PR with the same `.rgctl/` cache artifact.

### Temporal PR gate (`pr-check`)

For merge gates that compare **base vs head** graph artifacts (not just the working tree), use **`rgctl pr-check`**. Default mode **synthesizes the head snapshot** from a cached base artifact + git name-status delta (no second full `discover`). `check --temporal` delegates to the same pipeline.

```mermaid
flowchart TB
  subgraph inputs["Inputs"]
    BASE[".rgctl-base/ or $RGCTL_BASE_ARTIFACT"]
    POL[policy.json + scope + temporal]
    GIT["git diff base_ref head_ref (or worktree)"]
    LEDGER[violation_ledger.jsonl]
  end

  subgraph head["Delta head synthesis"]
    SEED[seed_head_artifact_from_base]
    DELTA[IncrementalUpdater + cascade]
    SEED --> DELTA
  end

  subgraph diff["Graph diff + scope"]
    PAIR[SnapshotPair::open]
    DIFF[diff_snapshots]
    PATHS[ScopedPaths + HunkIndex]
    ENT[EntityScope.changed_entities]
    PAIR --> DIFF
    PATHS --> ENT
  end

  subgraph policy["Scoped policy eval"]
    SCOPED[BlastRadiusEngine::build_scoped]
    TEMP[evaluate_temporal]
    CAL[calendar_policy grace / SLA / sunset]
    REG[ledger regression class]
    SCOPED --> TEMP --> REG --> CAL
  end

  BASE --> SEED
  GIT --> DELTA
  DELTA --> PAIR
  GIT --> PATHS
  ENT --> SCOPED
  POL --> TEMP
  POL --> CAL
  LEDGER --> REG
  LEDGER --> CAL
  DIFF --> OUT["JSON v2: passed, violations_summary, graph_diff, scope"]
  CAL --> OUT
```

| Class | Meaning | Fails when `scope.new_violations_only` |
|-------|---------|----------------------------------------|
| `new` | Violation appears only on head | Yes |
| `existing` | Violation on both snapshots | No (unless calendar SLA / post-grace) |
| `resolved` | Violation cleared on head | No (debt paid down) |
| `regression` | Reintroduced after ledger resolution | When `scope.fail_on_regression` |

Calendar fields (`temporal.*`) apply after temporal classification: grace windows emit `severity: warn` (exit 0 unless `--strict-calendar`); `violation_sla_days` + ledger `first_seen` can fail stale `existing` violations.

Artifact layout: base via `--base-artifact`, `$RGCTL_BASE_ARTIFACT`, or `{repo}/.rgctl-base/`; head synthesized into `{repo}/.rgctl/` unless `--full-snapshots`. Example CI: [.github/workflows/rgctl-pr-check.yml](../../.github/workflows/rgctl-pr-check.yml). User guide: [ci-policy-checks.md](../guides/ci-policy-checks.md).

---

## 7. Testing

| Layer | Location |
|-------|----------|
| Subprocess contract | `tests/cli_output/all_commands_sanity.rs` (`check` pass/fail, strict scope) |
| Temporal policy unit | `crates/rgctl-analysis/src/policy_diff.rs` |
| PR gate integration | `tests/pr_check_integration.rs` |
| PR gate golden | `tests/cli_output/subprocess_golden_path.rs` (`pr_check_json_*`) |
| Policy parsing | `src/cli/policy_file.rs` tests |
| Blast gatekeeping | `all_commands_sanity` blast-radius + policy exit 1 |

Screenshots: `capture-design-screenshots.mjs` → `docs/images/design/ci-policy-checks/`.

---

## 8. Related docs

- [Policy format](../policy-format.md)
- [Blast radius design](blast-radius-design.md)
- [Agent recipes](../agent-recipes.md) — CI workflow examples

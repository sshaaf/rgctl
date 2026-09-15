# Policy file format

JSON policy files gate **blast-radius**, **`check`**, and **`pr-check`** commands. They encode architecture rules as numeric limits and optional domain boundaries.

**Examples:** [examples/policy-permissive.json](examples/policy-permissive.json), [examples/policy-strict.json](examples/policy-strict.json)

---

## Schema

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `forbidden_crossings` | `[[string, string], ...]` | `[]` | Pairs of domain names that must not call across each other |
| `max_impact_nodes` | integer | unlimited | Fail if blast impact zone exceeds this count |
| `centrality_alert_threshold` | number | unlimited | Fail if betweenness (or related centrality signal) exceeds threshold |
| `node_domains` | object | `{}` | Map of node UUID string → domain label |
| `scope.new_violations_only` | boolean | `false` | **`pr-check` only:** exit 1 only on `new` temporal violations |
| `scope.fail_on_regression` | boolean | `true` | **`pr-check` only:** exit 1 on `regression` (reintroduced after ledger resolution) |
| `scope.strict_diff` | boolean | `false` | **`check`:** treat empty git diff scope as failure |
| `temporal.effective_from` | ISO date | — | Policy effective date (`YYYY-MM-DD`) |
| `temporal.grace_period_days` | integer | — | Days after `effective_from` where violations may warn instead of fail |
| `temporal.severity_during_grace` | `warn` \| `fail` | `warn` | Gate behavior during grace |
| `temporal.fail_existing_after_grace` | boolean | `false` | Fail `existing` violations after grace elapses |
| `temporal.violation_sla_days` | integer | — | Max age (days) for `existing` violations when `enforce_sla` is set |
| `temporal.enforce_sla` | boolean | `false` | Fail `existing` violations older than SLA (ledger `first_seen`) |
| `temporal.sunset_date` | ISO date | — | Escalate to fail on/after this date |
| `temporal.sunset_warn_days` | integer | — | Emit `severity: warn` within this many days of `sunset_date` |
| `size_limits.max_changed_files` | integer | unlimited | **`pr-check`:** abort if PR touches more files |
| `size_limits.max_scoped_entities` | integer | unlimited | **`pr-check`:** abort if scoped entity count exceeds limit |

### Minimal strict policy (CI fail on any impact)

```json
{
  "max_impact_nodes": 0
}
```

### Permissive policy (smoke tests)

```json
{
  "max_impact_nodes": 1000000,
  "centrality_alert_threshold": 1e12
}
```

### Domain crossing example

```json
{
  "forbidden_crossings": [["legacy", "payments"]],
  "node_domains": {
    "550e8400-e29b-41d4-a716-446655440000": "legacy"
  },
  "max_impact_nodes": 50
}
```

Assign domains via GQL (`RETURN n` includes node `id`) or from blast-radius JSON (`target.id`).

---

## CLI usage

### One-off blast-radius gate

```bash
rgctl -r "$REPO" -f json blast-radius ShoppingCartService \
  --policy-file policy.json
```

Exit code **1** when the policy is violated (`gatekeeping.policy_status` = `VIOLATED` in JSON).

### CI check on changed functions

```bash
rgctl -r "$REPO" -f json check --policy-file policy.json
```

Evaluates symbols touched in the git working tree (or the full graph if git is unavailable). Exit **1** when `passed` is false.

```bash
rgctl -f json check --policy-file policy.json | jq '{passed, violations: (.violations | length)}'
```

Scoped to commits:

```bash
rgctl -r "$REPO" -f json check \
  --policy-file policy.json \
  --base-ref origin/main \
  --head-ref HEAD \
  --strict
```

Temporal mode (same semantics as `pr-check`, delta head from base artifact):

```bash
rgctl -r "$REPO" -f json check --temporal --policy-file policy.json \
  --base-ref origin/main --head-ref HEAD
```

### Temporal PR gate (`pr-check`)

```bash
rgctl -r "$REPO" -f json pr-check \
  --policy-file rgctl-tests/rgctl-pr-policy.json \
  --base-ref origin/main \
  --head-ref HEAD \
  --strict
```

Defaults: delta head synthesis from base artifact; `--base-artifact` = `$RGCTL_BASE_ARTIFACT` or `{repo}/.rgctl-base/`. Use `--full-snapshots` for pre-built dual artifacts. Additional flags: `--bisect`, `--synthetic-head worktree`, `--cascade-depth`, `--strict-calendar` (treat grace/sunset warnings as failures). Outcomes append to `.rgctl/violation_ledger.jsonl`. Example workflow: [.github/workflows/rgctl-pr-check.yml](../.github/workflows/rgctl-pr-check.yml). Walkthrough: [CI Policy Checks guide](guides/ci-policy-checks.md).

---


## Response fields

See [json-api.md](json-api.md) blast-radius `gatekeeping` and `check` field catalogs.

---

## See also

- [Introduction — CI policy](Introduction.md#ci-policy-checks)
- [User Guide §14](user-guide.md#14-ci-policy-check)
- [Building a migration plan](building-migration-plan.md) — Phase 5 governance

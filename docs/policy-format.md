# Policy file format

JSON policy files gate **blast-radius** and **`check`** commands. They encode architecture rules as numeric limits and optional domain boundaries.

**Examples:** [examples/policy-permissive.json](examples/policy-permissive.json), [examples/policy-strict.json](examples/policy-strict.json)

---

## Schema

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `forbidden_crossings` | `[[string, string], ...]` | `[]` | Pairs of domain names that must not call across each other |
| `max_impact_nodes` | integer | unlimited | Fail if blast impact zone exceeds this count |
| `centrality_alert_threshold` | number | unlimited | Fail if betweenness (or related centrality signal) exceeds threshold |
| `node_domains` | object | `{}` | Map of node UUID string → domain label |

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

---

## Response fields

See [json-api.md](json-api.md) blast-radius `gatekeeping` and `check` field catalogs.

---

## See also

- [Introduction — CI policy](Introduction.md#ci-policy-checks)
- [User Guide §14](user-guide.md#14-ci-policy-check)
- [Building a migration plan](building-migration-plan.md) — Phase 5 governance

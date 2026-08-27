# CI Policy Checks

## Introduction

The `check` command is a **CI policy gateway** that evaluates your codebase against a set of architectural rules defined in a JSON policy file. It scans every function in the graph, runs blast-radius analysis, and reports any violations. If any rule is violated, the command exits with code 1, making it suitable as a CI gate that blocks merges when architectural constraints are broken.

Policy checks automate what would otherwise be manual code review: ensuring that no single function has too large an impact zone, no high-centrality function is left unprotected, and that domain isolation boundaries are respected.

## Use Cases

- **CI pipeline integration.** Add `check` as a build step to catch architectural violations before merge.
- **Blast-radius guardrails.** Prevent changes to functions whose impact zone exceeds a threshold.
- **Centrality alerts.** Flag functions with high centrality scores that may need extra review.
- **Domain isolation.** Enforce separation between modules that should not depend on each other.
- **Continuous architecture monitoring.** Track policy compliance over time as the codebase evolves.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). Make sure you have run `discover` first:

```bash
rgctl -r example/coolstore discover
```

## Step-by-Step

### 1. Examine the Policy File

The CoolStore example ships with a policy file at `example/coolstore/policy.json`:

```json
{"max_impact_nodes": 15, "centrality_alert_threshold": 0.8}
```

This policy defines two rules:

| Rule | Value | Meaning |
|------|-------|---------|
| `max_impact_nodes` | 15 | A function's blast-radius impact zone must not exceed 15 nodes |
| `centrality_alert_threshold` | 0.8 | Functions with centrality above 0.8 trigger an alert |

### 2. Run the Policy Check

```bash
rgctl -r example/coolstore -f json check \
  --policy-file example/coolstore/policy.json
```

**Output (truncated):**

```json
{
  "passed": false,
  "policy": "example/coolstore/policy.json",
  "schema_version": 1,
  "violations": [
    {
      "error": "Graph error: scale failure: impact zone size 114 exceeds max 15",
      "symbol": "baseIsEqual"
    },
    {
      "error": "Graph error: scale failure: impact zone size 648 exceeds max 15",
      "symbol": "indexOf"
    },
    {
      "error": "Graph error: scale failure: impact zone size 110 exceeds max 15",
      "symbol": "getMatchData"
    },
    {
      "error": "Graph error: scale failure: impact zone size 16 exceeds max 15",
      "symbol": "_fnInitComplete"
    },
    {
      "error": "Graph error: scale failure: impact zone size 456 exceeds max 15",
      "symbol": "trimmedLeftIndex"
    }
  ]
}
```

**What this tells you:**

- **`passed: false`** -- the codebase has policy violations.
- **`violations`** -- each violation lists the offending symbol and the rule it broke.
- `indexOf` has the largest impact zone at 648 nodes -- changing this function could affect 648 other functions.
- `baseIsEqual` (impact: 114) and `trimmedLeftIndex` (impact: 456) are lodash utility functions deeply embedded in the call graph.
- `_fnInitComplete` barely exceeds the threshold at 16 nodes.

### 3. Check the Exit Code

The `check` command exits with code 1 on violations, making it suitable for CI:

```bash
rgctl -r example/coolstore check \
  --policy-file example/coolstore/policy.json
echo "Exit code: $?"
```

```
Exit code: 1
```

In a CI pipeline:

```yaml
# GitHub Actions example
- name: Architecture check
  run: rgctl -r . check --policy-file policy.json
```

If any violation is found, the step fails and the build is blocked.

### 4. Text Format for Human Review

Use text format for readable output in pull request comments:

```bash
rgctl -r example/coolstore check \
  --policy-file example/coolstore/policy.json
```

### 5. Per-Function Policy Check with Blast Radius

You can also apply a policy to a single function using `blast-radius --policy-file`:

```bash
rgctl -r example/coolstore -f json blast-radius priceShoppingCart \
  --policy-file example/coolstore/policy.json
```

This runs blast-radius analysis on `priceShoppingCart` and checks the result against the policy. The `gatekeeping` section of the output shows whether the function passes or violates the policy.

### 6. Writing a Custom Policy

Create a policy file tailored to your project:

```json
{
  "max_impact_nodes": 25,
  "centrality_alert_threshold": 0.7
}
```

Stricter policies (lower thresholds) catch more violations; permissive policies (higher thresholds) only flag extreme cases.

The `docs/examples/` directory contains example policies:

| File | Purpose |
|------|---------|
| `policy-strict.json` | Tight thresholds for well-modularized codebases |
| `policy-permissive.json` | Relaxed thresholds for monoliths in early migration |

## Policy File Format

```json
{
  "max_impact_nodes": <integer>,
  "centrality_alert_threshold": <float 0.0-1.0>
}
```

| Field | Type | Description |
|-------|------|-------------|
| `max_impact_nodes` | integer | Maximum allowed impact zone size for any function |
| `centrality_alert_threshold` | float | Centrality score above which a function triggers a violation |

See the [Policy Format Reference](../policy-format.md) for the full schema.

## Understanding Violations

| Violation Type | Message Pattern | Cause |
|----------------|----------------|-------|
| Scale failure | `impact zone size N exceeds max M` | A function's blast radius exceeds `max_impact_nodes` |
| Centrality alert | `centrality N exceeds threshold M` | A function's centrality score exceeds `centrality_alert_threshold` |
| Domain isolation | `cross-domain call from A to B` | A function calls across a domain boundary |
| Cascade hazard | `cascade depth N exceeds max M` | A function's call chain depth exceeds the maximum |

## Benefits

- **Automated architecture enforcement.** Replace manual review with machine-checked policies.
- **Clear exit codes.** Exit 0 = pass, exit 1 = violations -- integrates with any CI system.
- **Structured output.** JSON violations are easy to parse, aggregate, and trend over time.
- **Customizable thresholds.** Tune policies to match your project's maturity and architecture goals.
- **Preventive, not reactive.** Catch architectural drift before it ships, not after it causes problems.

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- must run `discover` before `check`
- [Blast Radius Analysis](blast-radius-analysis.md) -- the per-function analysis that `check` runs at scale
- [Graph Metrics](graph-metrics.md) -- centrality scores that feed into policy checks
- [Migration Planning](migration-planning.md) -- combine policy checks with migration roadmaps

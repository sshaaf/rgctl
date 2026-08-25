# Expected-facts schema (`expected-facts.json`)

Hand-labeled graph facts checked by `cargo test --test graph_correctness` (approach **A**) plus
cross-feature invariants (approach **B**). See [sshaaf/rgctl#26](https://github.com/sshaaf/rgctl/issues/26)
and [QE.md](./QE.md) for required-red policy (option B).

## Location

```text
ecommerce-<lang>/correctness/expected-facts.json
```

Projects without this file are skipped by the graph_correctness tests.

## Top-level fields

| Field | Type | Meaning |
|-------|------|---------|
| `schema_version` | int | Currently `1` |
| `language` | string | Language id (`java`, `rust`, …) |
| `project` | string | Directory name (`ecommerce-java`, …) |
| `graph` | object | Repo-level minimums |
| `symbols` | object | Named expected-fact entries (keys are stable ids) |
| `invariants` | object | Cross-feature checks (B1…) |

## Severity

| Value | CI behavior |
|-------|-------------|
| `required` | Failure → test fails |
| `best_effort` | Reported only |
| `unsupported` | Skipped |

## Symbol entry (thorough)

Prefer **exact** fields for harness symbols. Soft `min_*` / `*_any` remain for domain edges that are still noisy.

```json
{
  "match": { "name": "correctnessMid", "class": "CorrectnessHarness" },
  "severity": "required",
  "unique": true,
  "identity": {
    "exact_name": "correctnessMid",
    "canonical_fqn": "CorrectnessHarness::correctnessMid",
    "class_context": "CorrectnessHarness",
    "file_suffix": "CorrectnessHarness.java",
    "language": "java"
  },
  "exact_callees": ["correctnessLeaf"],
  "exact_callers": ["correctnessRoot"],
  "blast": {
    "exact_direct_callers": ["correctnessRoot"],
    "exact_impact_zone": ["correctnessRoot"],
    "min_score": 0.1
  },
  "ast": {
    "must_call": ["correctnessLeaf"],
    "statements_contain": ["return correctnessLeaf() + 1;"],
    "exact_statement_kinds": ["Return"]
  },
  "cfg": {
    "exact_block_count": 2,
    "exact_edge_count": 1,
    "exact_edge_kinds": ["return"]
  },
  "pdg": {
    "exact_data_deps": 0,
    "exact_control_deps": 0,
    "exact_node_count": 1,
    "data_edges": [{ "variable": "value" }],
    "node_labels_contain": ["correctnessLeaf()"]
  },
  "dom": {
    "exact_block_count": 2,
    "exact_idom": [
      { "block": 0, "immediate_dominator": 1 },
      { "block": 1, "immediate_dominator": 1 }
    ]
  }
}
```

### Layers

| Section | Source of truth | What we assert |
|---------|-----------------|----------------|
| `identity` | `blast-radius` `target` + `inspect` `symbol` | Exact function name, FQN, class, file, language |
| `exact_callees` / `exact_callers` | GQL `CALLS` | Exact short-name sets (no extras) |
| `blast` | `blast-radius` topology/metrics | Exact callers, impact zone, counts, score |
| `ast` | CFG statement texts/kinds | Call sites and statement text as lowered from AST |
| `cfg` | `inspect SYMBOL cfg` | Exact blocks, edges, edge kinds, statement text |
| `pdg` / `dataflow` | `inspect SYMBOL pdg` | Data/control deps, node kinds/labels, data-edge variables |
| `dom` / `dominance` | `inspect SYMBOL dom` | Block count, idom pairs |

### `match`

Used to resolve a unique function via `blast-radius` / `inspect`:

- Prefer `Class::name` when `class` is set
- Fall back to bare `name` (must be unique when `unique: true`)

## Invariants (B)

| Id | Check |
|----|--------|
| `B1_blast_vs_calls` | Blast `topology.direct_callers` names ≡ reverse CALLS neighbors |
| `B2_gql_calls_nonzero` | At least one CALLS edge exists after discover |
| `B5_inspect_cfg_present` | `inspect <symbol> cfg` returns nodes |
| `B6_cfg_calls_subset_of_calls_edges` | Expected callees appear in both CALLS graph and CFG/AST text |
| `B7_pdg_lines_subset_cfg` | Every PDG node line appears in CFG statements |
| `B8_dom_blocks_match_cfg` | DOM `block_index` set ≡ CFG `block_index` set |
| `B9_blast_target_name` | Blast `canonical_fqn` short name ≡ `match.name` |

## Harness conventions

Each `ecommerce-*` app should include a small intentional call chain:

`correctnessRoot` → `correctnessMid` → `correctnessLeaf`

Additionally, each language ships **CFG feature probes** (`CfgFeatures` / `cfg_features.*`) that exercise
control-flow lowering unique to that language (short-circuit, loops, switch, try/cleanup, await, …).
Those symbols are asserted via `cfg.min_blocks` + `cfg.statements_contain` in `expected-facts.json`.

(Language-idiomatic spellings: `correctness_root`, `CorrectnessRoot`, etc.)

Prefer **direct calls** with **no intermediate locals** assigned from call results **in C/C++**.
Some C/C++ extractors have mis-attributed `int value = mid();` as a CALLS edge from `value` → `mid`.
Java may keep `int value = mid();` when PDG data-deps on `value` are part of the expected-facts.

Keep harness definitions in **one translation unit** (avoid header prototypes that become duplicate Function nodes).

## Versioning

Bump `schema_version` when fields are removed or semantics change. Additive fields may keep `1`.

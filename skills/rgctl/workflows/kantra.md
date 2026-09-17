# Kantra workflow

**Primary output:** `.rgctl/kantra_findings.json` and `KantraRule` / `VIOLATES` in the graph.

**This workflow is not migration roadmap export.** Do not present `migration_plan.json` as the main Kantra deliverable.

Native evaluation of [Konveyor Kantra](https://github.com/konveyor/kantra) rules against the rgctl graph and source cache. Release builds embed Konveyor `stable/java` (~2.6k rules); no external Kantra CLI required.

### Default Kantra discover

**User intent:** *"Run Konveyor migration rules on this Java codebase"*

```bash
rgctl discover . -l java --with-kantra
# violations: .rgctl/kantra_findings.json
# rules in graph: KantraRule / KantraRuleset nodes (GQL)
```

Report `catalog_id`, `evaluated_rules`, violation count, sample hits (`rule_id`, `file`, `line`, `matched_by`), and top `skipped_rules` reasons.

### Target-filtered eval

**User intent:** *"What Quarkus migration rules apply?" / "Audit for Spring Boot 3+"*

```bash
rgctl discover . -l java --with-kantra --kantra-target quarkus
# or: --kantra-target spring-boot3+
```

`target_filter` appears in `kantra_findings.json`. Only rules with `konveyor.io/target=<NAME>` labels are evaluated.

### Rules inventory (GQL)

**User intent:** *"List migration rules indexed in the graph" / "Which rules target Quarkus?"*

```bash
rgctl -f json gql "MATCH (r:KantraRule) RETURN r LIMIT 20"
# Konveyor labels are node properties — use backtick-quoted keys:
rgctl -f json gql 'MATCH (r:KantraRule) WHERE r.`konveyor.io/target` = '\''quarkus'\'' RETURN r'
```

`KantraRuleset` nodes link to rules via `CONTAINS` edges. After full eval, `VIOLATES` edges connect rules to code nodes; `kantra_findings.json` has line-level detail and enrichment.

### Fixture / CI override

**User intent:** *"Run a small custom ruleset in CI"*

```bash
rgctl discover . --with-kantra --kantra-rules tests/fixtures/kantra-rules
```

Mutually exclusive with `--kantra-catalog`. Embedded catalog is the default when neither override is set.

### Index only

**User intent:** *"Index rules into the graph without running eval"*

```bash
rgctl discover . --with-kantra --kantra-index-only
```

Useful when you only need GQL rule inventory. Eval stage is skipped; `kantra_findings.json` is not written.

**Pitfalls:**

- Does **not** require `--with-cfg`
- Many upstream Konveyor rules use unsupported providers (`builtin.xml`, `java.dependency`) or Windup-style regex — expect a large `skipped_rules` list with full catalog
- Re-run discover after rule/catalog changes; kantra index rewrites `graph.snapshot.bin` at end of pipeline

**See:** [User guide — Kantra](../../docs/user-guide.md#kantra-migration-rules---with-kantra), [JSON API](../../docs/json-api.md#kantra_findingsjson), [KANTRA_ARCHITECTURE_OPTIONS.md](../../KANTRA_ARCHITECTURE_OPTIONS.md)

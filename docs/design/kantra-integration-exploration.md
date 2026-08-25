# Kantra Integration Exploration — `--with-kantra`

Feasibility analysis for evaluating [Konveyor Kantra](https://github.com/konveyor/kantra) rules natively against the rgctl graph, without requiring an external LSP or container runtime.

---

## 1. Context

Kantra is the Konveyor CLI for static code analysis and transformation. It uses YAML-based rules with a `when` clause that dispatches to language providers (`java.referenced`, `go.referenced`, `builtin.filecontent`, etc.). Rules produce violations (issues) with severity, effort estimates, and remediation guidance.

rgctl already performs line-level code traversal during CFG/PDG/taint analysis and maintains a rich graph of symbols, imports, call edges, and community structure. The question is: **can we evaluate Kantra rules against our graph and source cache?**

---

## 2. Current rgctl Rule/Security Systems

rgctl has four independent rule and security systems today. They do not share a rule format or cross-reference each other.

### 2.1 Taint Analysis (hardcoded, per-language)

- **Crate:** `rgctl-analysis`, `taint.rs`
- **Trigger:** `--with-taint` (implies `--with-cfg`)
- Sources, sinks, and sanitizers are detected via `text.contains()` on PDG node statement text
- 8 languages supported (Python, JS/TS, Rust, Go, Java, C#, C, C++)
- Forward BFS propagation over PDG data-dependency edges
- **All patterns are hardcoded** in Rust source — no config file

### 2.2 CWE/Security Patterns (hardcoded)

- **Crate:** `rgctl-security`, `cve_patterns.rs` / `analyzer.rs`
- 10 CWE patterns (SQL injection, XSS, command injection, path traversal, etc.)
- Regex-based matching of taint flows against CWE source/sink patterns
- `CwePattern` struct is `Serialize/Deserialize` but **always loaded from `default_cwe_patterns()`** — no file-loading path exists

### 2.3 Rule Engine (JSON config)

- **Crate:** `rgctl-rules`, `schema.rs` / `matcher.rs` / `actions.rs`
- JSON-serializable rulesets with composite boolean conditions (`And`/`Or`/`Not`/`Leaf`)
- Leaf conditions: `NodeType`, `NamePattern`, `HasLabel`, `ComplexityGt/Lt`, `HasProperty`, `CallsAny`, `NodeTypeField`
- Actions: `AddLabel`, `SetMetadata`, `SetComplexityOverride`
- Operates on graph nodes — **completely separate from taint/security**
- **Note:** `RuleActionRaw` exists as an unused placeholder for shorthand JSON; `normalize_rule_actions()` is a no-op stub. No CLI command exists to create rules.

### 2.4 Policy/Check (JSON config)

- **Crate:** `rgctl-service`, `policy.rs` / `check.rs`
- Blast-radius policy guardrails: `forbidden_crossings`, `max_impact_nodes`, `centrality_alert_threshold`
- CI gate: `rgctl check --policy-file` exits 1 on violations
- **Does not evaluate taint, security, or rule engine results** — only blast-radius impact

---

## 3. Kantra Rule Format

A Kantra rule is YAML with three parts: metadata, condition (`when`), and action (message/tag).

```yaml
- ruleID: crypto-change-00010
  description: "x/crypto/hkdf bypasses FIPS module"
  category: mandatory          # mandatory | optional | potential
  effort: 3                    # story points
  labels:
    - konveyor.io/source=go
    - konveyor.io/target=go
  message: "detailed remediation with ### Before / ### After"
  links:
    - url: https://pkg.go.dev/crypto/hkdf
      title: "Migration Documentation"
  when:
    go.referenced:
      pattern: golang.org/x/crypto/hkdf
```

### 3.1 Condition Types

| Provider | Capability | Fields | Description |
|----------|-----------|--------|-------------|
| `builtin` | `filecontent` | `pattern` (regex), `filePattern` (glob) | Regex match in source files |
| `builtin` | `file` | `pattern` (glob) | Match filenames |
| `builtin` | `xml` | `xpath`, `namespaces`, `filepaths` | XPath queries on XML |
| `builtin` | `json` | `xpath`, `filepaths` | JSONPath queries |
| `builtin` | `hasTags` | (inline list) | Check for previously-applied tags |
| `java` | `referenced` | `pattern`, `location`, `annotated` | Java symbol references |
| `java` | `dependency` | `name`, `upperbound`, `lowerbound` | Dependency version check |
| `go` | `referenced` | `pattern` | Go symbol references |
| `go` | `dependency` | `name`, `upperbound`, `lowerbound` | Dependency version check |

Conditions compose with `and:`, `or:`, `not:`.

### 3.2 Java `location` Values

`IMPORT`, `PACKAGE`, `TYPE`, `INHERITANCE`, `ANNOTATION`, `IMPLEMENTS_TYPE`, `FIELD`, `METHOD`, `METHOD_CALL`, `CONSTRUCTOR_CALL`, `CLASS`, `RETURN_TYPE`, `VARIABLE_DECLARATION`, `ENUM_CONSTANT`.

### 3.3 Ruleset Structure

Rules live in a directory with a `ruleset.yaml` golden file plus one or more `*.yaml` rule files:

```
my-rules/
  ruleset.yaml          # { name, description, labels }
  cloud-readiness.yaml  # list of rules
  crypto.yaml           # list of rules
```

---

## 4. What rgctl Already Has for Evaluation

### 4.1 Source Text in Memory

`discover_cfg.rs` preloads every source file into `FileSourceCache` via `preload_file_sources()`. This cache is available during the entire CFG/PDG/taint batch. Kantra's `builtin.filecontent` evaluation would be a regex match against this already-loaded text.

### 4.2 Import Nodes

Import statements are **first-class graph nodes** (`NodeType::Import`):

| Language | `name` field | `qualified_name` field |
|----------|-------------|----------------------|
| Java | Full import text (`import java.util.List;`) | `None` |
| Go | Short name (last path segment) | Full import path (`github.com/pkg/errors`) |
| C/C++ | `#include` path | — |
| C# | `using` directive | — |

Connected via `EdgeType::Uses` and `EdgeType::DefinedIn` edges.

### 4.3 Annotation Nodes and Edges

Java annotations are fully extracted during **initial graph indexing** (plain `discover`, no `--with-cfg` required):

- **Annotation type declarations** (`@interface Foo`) become `NodeType::Annotation` nodes with `qualified_name`, fields (constants), and modifiers.
- **Annotation element declarations** (`String value() default ""`) become `NodeType::Function` nodes under the owning annotation type, with `metadata["is_annotation_element"]: true` and `metadata["default_value"]` if present.
- **Annotation usage** (`@RequestMapping(path="/x")`) creates `EdgeType::AnnotatedWith` edges from the annotated symbol to the annotation type.
- **Annotation arguments** are captured as a **raw string** in edge `metadata["arguments"]` — e.g. `(path = "/x")`. This is an opaque string, not decomposed into individual key-value pairs, but it is regex-matchable.
- **Unresolved annotations** (e.g. `@Override` from `java.lang`) create external stub nodes with `NodeType::Annotation` and `is_external_stub: "true"`.

Supported annotation sites: classes, interfaces, enums, records, methods, constructors, fields, formal parameters, and type-use annotations (`List<@NonNull String>`).

GQL queryable: `MATCH (a:Function)-[:ANNOTATED_WITH]->(b:Annotation) RETURN a,b`

**Important:** `--with-cfg` does NOT add annotation data. The CFG/PDG/taint pipeline contains zero Java-annotation-specific logic. All annotation extraction happens in the language plugin during initial indexing.

### 4.4 Symbol Metadata

Every graph `Node` has:

- `qualified_name` — e.g. `com.example.Foo`, `github.com/pkg`
- `signature` — full function/method signature
- `return_type` — return type
- `parameters` — structured parameter list with types
- `file_path`, `start_line`, `end_line` — source location
- `properties` — open key-value bag (`modifiers`, `visibility`, `field_type`, `member_of`, `owner_qualified_name`, etc.)
- `labels` — for categorization and rule output

### 4.5 Edge Types Available

`Calls`, `Uses`, `Inherits`, `DefinedIn`, `Contains`, `Implements`, `AnnotatedWith` — all relevant for `*.referenced` evaluation with `location` semantics. Note: `AnnotatedWith` edges are not traversed during blast-radius analysis (only `Calls` edges are), but they are visible in GQL and available for rule matching.

### 4.6 Taint/PDG Infrastructure

The taint detection loop (`taint.rs:detect_*_patterns()`) already iterates every PDG node's `statement.text` doing string matching. Kantra filecontent evaluation is structurally identical but uses regex instead of `contains()`.

---

## 5. Feasibility Mapping

### 5.1 Evaluation Coverage

| Kantra Condition | rgctl Data | Feasibility | Cost |
|-----------------|----------------|-------------|------|
| `builtin.filecontent` | `FileSourceCache` (source text) | **Full** | Trivial — regex on cached source |
| `builtin.file` | `File` nodes with `file_path` | **Full** | Trivial — glob match |
| `builtin.hasTags` | `labels[]` on nodes | **Full** | Zero — maps to existing `HasLabel` |
| `go.referenced` (simple) | `Import` nodes with `qualified_name` + source text | **Full** | Low — pattern match on Import nodes + source grep |
| `java.referenced` `IMPORT` | `Import` nodes (full import text in `name`) | **Full** | Low — regex on Import node name |
| `java.referenced` `PACKAGE` | Import nodes + source text | **Full** | Low — Import match + filecontent regex |
| `java.referenced` `TYPE` | `Class`/`Struct` nodes with `qualified_name` | **Full** | Low — NodeType + qualified_name match |
| `java.referenced` `INHERITANCE` | `Inherits` edges | **Full** | Low — follow Inherits edges |
| `java.referenced` `FIELD` | `Variable` nodes with `field_type`, `member_of` | **Partial** | Low — match on Variable node properties |
| `java.referenced` `METHOD` | `Function` nodes with `qualified_name` + `signature` | **Partial** | Moderate — signature pattern parsing needed |
| `java.referenced` `ANNOTATION` | `Annotation` nodes + `AnnotatedWith` edges | **Full** | Low — match annotation name via edge target, filter by annotated symbol type |
| `java.referenced` `annotated.pattern` | `AnnotatedWith` edges to annotation nodes | **Full** | Low — regex match on annotation node name/qualified_name |
| `java.referenced` `annotated.elements` | Raw arguments string in edge `metadata["arguments"]` | **Partial** | Moderate — regex match on opaque `"(key = value)"` string works for simple cases; no structured per-element decomposition |
| `*.dependency` | No dependency graph (no pom.xml/go.mod parsing) | **Not feasible** | Different problem space |
| `builtin.xml` (xpath) | No XML AST | **Not feasible** | Out of scope |
| `builtin.json` (jsonpath) | No JSON AST | **Not feasible** | Out of scope |
| Condition chaining (`as:`/`from:`) | No inter-condition variable passing | **Not feasible** | Complex runtime plumbing |
| Custom variables | No template interpolation | **Not feasible** | Formatting concern, lower priority |
| `and:` / `or:` / `not:` | `MatchCondition` already supports composite logic | **Full** | Zero — already implemented |

### 5.2 Coverage of Real-World Rulesets

Analysis of the [konveyor/rulesets](https://github.com/konveyor/rulesets) repository shows the distribution of condition types:

| Pattern | Approximate Frequency | rgctl Coverage |
|---------|----------------------|-------------------|
| `*.referenced` with simple pattern (no location) | ~60% | Full |
| `builtin.filecontent` (regex in source) | ~25% | Full |
| `builtin.file` (filename match) | ~5% | Full |
| Composite `and:`/`or:` of above | ~5% | Full |
| `*.dependency` (version checks) | ~3% | Not feasible |
| `builtin.xml`/`builtin.json` | ~1% | Not feasible |
| `java.referenced` with `annotated.elements` | ~1% | Partial (regex on raw args string) |

**Estimated coverage: ~90-95% of real-world Kantra rules are evaluable with existing rgctl data.** The annotation upgrade (from "not feasible" to "partial") comes from the fact that `AnnotatedWith` edges with raw argument strings already exist in the graph — no `--with-cfg` needed.

---

## 6. Architecture Options

### 6.1 Option A: Translation Layer

Parse Kantra YAML, translate to rgctl `MatchCondition` + actions, evaluate against the graph.

- Pro: No external dependency
- Pro: Graph-enhanced evaluation
- Con: Lossy translation for unsupported conditions
- Con: Cannot do LSP-level queries

### 6.2 Option B: Kantra as Peer Runner

Shell out to the `kantra` CLI during discover, ingest its `output.yaml`, map violations to graph nodes by file + line.

- Pro: 100% fidelity for all condition types
- Pro: All providers work (Java LSP, Go LSP, etc.)
- Con: External dependency (kantra + podman/docker)
- Con: Slow (JDT LS startup, container overhead)

### 6.3 Option C: Native Evaluation with Graph Enrichment (Recommended)

Parse Kantra YAML natively. Classify each rule by evaluability. Evaluate supported conditions against the graph and source cache. Skip unsupported conditions with a clear report. Enrich results with graph context that Kantra alone cannot provide.

```
  ┌─────────────────────────────────────────────────────────┐
  │  1. Parse Kantra YAML natively (serde + yaml)           │
  │  2. Classify each rule's when clause:                   │
  │     Evaluable::Graph   — *.referenced                   │
  │     Evaluable::Source  — builtin.filecontent             │
  │     Evaluable::File    — builtin.file                   │
  │     Evaluable::Tag     — builtin.hasTags                │
  │     NotSupported       — *.dependency, xml, json,       │
  │                          annotated.elements              │
  │  3. Evaluate supported conditions                       │
  │  4. Enrich with graph context (blast-radius,            │
  │     community, centrality, taint correlation)           │
  │  5. Produce structured output                           │
  └─────────────────────────────────────────────────────────┘
```

---

## 7. Pipeline Integration

### 7.1 Does `--with-kantra` Imply `--with-cfg`?

No. Kantra evaluation has two tiers:

**Standalone (`--with-kantra`):**
- `builtin.filecontent` — uses `FileSourceCache` (preloaded once)
- `builtin.file` — uses graph `File` nodes
- `*.referenced` (simple) — uses `Import`/`Class`/`Function` nodes
- `builtin.hasTags` — uses node labels

**Enhanced (`--with-kantra --with-cfg`):**
- All of the above, plus:
- PDG-level matches — "is this reference on a taint path?"
- Flow-aware enrichment — "this javax import reaches a SQL sink"

### 7.2 Where It Hooks In

The discover pipeline in `discover_impl.rs` (`run_full_analysis`) would add a Kantra evaluation stage:

```
  Existing stages:
    1. File indexing          (always)
    2. Complexity analysis    (always)
    3. Topology / CSR         (always)
    4. Community detection    (always)
    5. Centrality analysis    (always)
    6. Security scanning      (opt-in: --with-security)
    7. CFG/PDG/Taint batch    (opt-in: --with-cfg / --with-taint)
    8. Blast radius           (always after topology)
    9. Dashboard export       (opt-in: --with-dashboard)

  New stage:
    6.5  Kantra evaluation    (opt-in: --with-kantra)
         Runs AFTER indexing (needs graph + file nodes)
         Runs BEFORE CFG batch (doesn't need PDG)
         Uses FileSourceCache (preloaded if --with-cfg,
         otherwise triggers its own preload)
```

### 7.3 Incremental Caching

The existing CFG batch uses `code_hash`-based incremental caching. Kantra evaluation can use the same strategy: if the source file hasn't changed and the ruleset hasn't changed (hash the ruleset YAML), skip re-evaluation for that file.

---

## 8. The Graph Advantage

This is what differentiates rgctl from running Kantra directly. For every Kantra violation, rgctl can cross-reference with:

| Enrichment | What It Adds |
|------------|-------------|
| **Blast radius** | "This import is used by 47 functions across 3 communities" |
| **Community** | "All violations cluster in community 7 — that's your payment module" |
| **Centrality** | "This is a high-PageRank hotspot — fixing it has outsized impact" |
| **Taint correlation** | "This matched pattern is also on a vulnerable taint path (CWE-89)" |
| **Call neighborhood** | "These 12 functions transitively depend on the violated symbol" |

Kantra tells you *what* is wrong. The graph tells you *how much it matters*.

---

## 9. Boundaries (What This Cannot Do)

1. **Limited annotation element inspection** — `annotated: { elements: [{ name: url, value: "..." }] }` can be partially matched via regex on the raw `metadata["arguments"]` string (e.g. matching `url` and `"http://..."` in `(url = "http://...")`), but there is no structured per-element decomposition. Complex multi-element matching or type-aware element inspection is not feasible without deeper AST extraction.
2. **No dependency version checking** — `*.dependency` needs manifest parsing (`pom.xml`, `go.mod`, `package.json`)
3. **No XPath/JSONPath queries** — no XML/JSON AST
4. **No cross-condition chaining** — `as:` / `from:` / `filepaths: "{{poms.filepaths}}"` inter-condition variable passing
5. **No custom variables** — template interpolation in messages (formatting concern, lower priority)

Items 2-4 are fundamental limitations of graph-based evaluation. Item 1 is a partial limitation with a reasonable workaround for simple cases. Item 5 is implementable but low priority.

---

## 10. Open Questions

1. **Rule loading path** — Kantra rules use a directory structure with `ruleset.yaml` + `*.yaml`. Does `--with-kantra` take `--kantra-rules ./my-rules/`? Or a path to individual YAML files?

2. **Output format** — Should violations produce Kantra-compatible `output.yaml` (for Konveyor Hub ingestion)? Or rgctl's own JSON format with Kantra metadata attached? Or both?

3. **`check` integration** — Could `rgctl check --kantra-rules ./rules/ --policy-file policy.json` gate CI on mandatory Kantra violations? This would unify the blast-radius policy engine with Kantra rule evaluation.

4. **Rules create CLI** — The existing rule engine has no CLI for creating rules. Should `--with-kantra` also motivate `rgctl rules init` (scaffold a Kantra-compatible YAML ruleset)?

5. **Fast Kantra substitute** — If rgctl can evaluate ~90% of Kantra rules without containers or LSP servers, it could serve as a fast pre-check in CI before a full Kantra run. Is that a valuable user story?

---

## 11. Related Docs

- [Taint Analysis Design](taint-analysis-design.md)
- [CI Policy Checks Design](ci-policy-checks-design.md)
- [Blast Radius Design](blast-radius-design.md)
- [Kantra Rules Quickstart](https://github.com/konveyor/kantra/blob/main/docs/rules-quickstart.md)
- [Konveyor Rulesets](https://github.com/konveyor/rulesets)
- [analyzer-lsp Rules Reference](https://github.com/konveyor/analyzer-lsp/blob/main/docs/rules.md)

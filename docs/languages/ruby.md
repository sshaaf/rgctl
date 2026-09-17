# Ruby

Tier 1 plugin for Ruby source (Rails-style apps, gems, scripts). Extracts classes, modules, methods, `require` graph, mixin edges, calls, and constructor metadata.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-ruby` (`RubyPlugin`) |
| **Grammar** | `tree-sitter-ruby` (pinned in crate `Cargo.toml`) |
| **Extensions** | `.rb`, `.rake`, `.gemspec` (via `languages.toml`) |
| **Discover** | `rgctl discover . -l ruby -e vendor,tmp,node_modules --with-cfg` |
| **CFG / taint** | Enabled (`LanguageAnalysisProfile`) |

AST coverage is tracked in `crates/rgctl-lang-ruby/ruby-ast-coverage.json` (CI: `ruby_ast_coverage_manifest_matches_grammar`).

## What is extracted

### Nodes

- **Function** — instance and singleton methods; FQN uses `::` for constants/modules and `#` / `.` for methods (see [ruby-extract-honesty.md](../ruby-extract-honesty.md))
- **Class** / **Module**
- **Import** — `require` / `require_relative` targets (unresolved string paths as import symbols)
- **Field** — `attr_*` and ivars assigned in `initialize` (Layer F symbols)

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Statically resolved calls; dynamic calls tagged `metadata.unresolved` |
| `EXTENDS` | `include` / `prepend` into class/module |
| `USES` | `extend` on singleton |
| `INSTANTIATES` | `.new` on constant/receiver |
| `Import` | Require graph |

## Honesty limits

See [ruby-extract-honesty.md](../ruby-extract-honesty.md) — no Ruby method lookup, refinements, or full block/yield CFG for arbitrary procs.

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-ruby` |
| **Langfeatures** | `tests/fixtures/ruby/langfeatures` |
| **Example corpus** | `example/discourse` (`-l ruby`; `RGCTL_DISCOURSE_REPO`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-ruby.sh` |

```bash
RGCTL=target/release/rgctl ./rgctl-tests/gql-verification-smoke/verify-extraction-gql-ruby.sh
```

Integration tests: `tests/ruby_langfeatures.rs`, `tests/ruby_cfg_analysis.rs`, `tests/dashboard_ecommerce_ruby.rs`, `tests/ruby_taint.rs` (taint unit tests in `rgctl-analysis`).

## Related

- [Languages index](README.md)
- [Tier 1 parity row](../tier-1-language-support.md#8-current-parity-snapshot-2026-07)

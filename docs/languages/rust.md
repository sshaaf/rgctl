# Rust

Tier 1 plugin for Rust source. Extracts modules, traits, structs, enums, impl blocks, attributes, and call graph edges.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-rust` (`RustPlugin`) |
| **Grammar** | `tree-sitter-rust` |
| **Extensions** | `.rs` |
| **Discover** | `rgctl discover . -l rust -e target --with-cfg` |
| **CFG / taint** | Enabled |

Tree-sitter node kinds: `function_item`, `struct_item`, `enum_item`, `impl_item`, `use_declaration`.

## What is extracted

### Nodes

- **Function** — free functions and methods
- **Struct**, **Enum**, **Trait** (via impl/class kinds)
- **Import** — `use` declarations (module graph)

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Function and method calls |
| `IMPLEMENTS` | Trait implementations |
| `ANNOTATEDWITH` | Attributes (`#[derive]`, `#[test]`, …) |
| `INSTANTIATES` | Struct/enum construction |
| `Import` | `use` path relations |

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-rust` |
| **Example corpus** | `example/rust` (`-l rust`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-rust.sh` |

## GQL verification queries

### Fixture probes

| Probe | GQL |
|-------|-----|
| Module graph (Import) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| Trait heritage (IMPLEMENTS) | `MATCH (a)-[:IMPLEMENTS]->(b) RETURN a,b LIMIT 10000` |
| Attributes (ANNOTATEDWITH) | `MATCH (a)-[:ANNOTATEDWITH]->(b) RETURN a,b LIMIT 10000` |
| Instantiation (INSTANTIATES) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| Call resolution (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

### Example smoke (`example/rust`)

| Probe | GQL |
|-------|-----|
| Import (scale) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| IMPLEMENTS (scale) | `MATCH (a)-[:IMPLEMENTS]->(b) RETURN a,b LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- Openspec: `rust-module-graph`, `rust-trait-heritage`, `rust-attributes`, `rust-call-resolution`

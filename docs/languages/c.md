# C

Tier 1 plugin for C source and headers. Focuses on include graph, function symbols, call resolution, and `file::symbol` qualified names.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-c` (`CPlugin`) |
| **Grammar** | `tree-sitter-c` |
| **Extensions** | `.c`, `.h` |
| **Discover** | `rgctl discover . -l c -e build,cmake-build-debug,.rgctl --with-cfg` |
| **CFG / taint** | Enabled |

Tree-sitter node kinds: `function_definition`, `struct_specifier`, `enum_specifier`, `preprocessor_include`.

## What is extracted

### Nodes

- **Function** — `qualified_name` as `file::symbol` (e.g. `review_repository::init`)
- **Struct**, **Enum**
- **Import** — `#include` preprocessor edges (`.c` files)

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Function calls |
| `Import` | Include / header dependencies |

No OOP `EXTENDS`/`INSTANTIATES` — C uses struct composition at the type level only.

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-c` |
| **Example corpus** | `example/linux` (default discover) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-c.sh` |

## GQL verification queries

### Fixture probes

| Probe | GQL |
|-------|-----|
| Call resolution (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |
| Include graph (Import from .c) | `MATCH (n:Import) WHERE n.file_path LIKE '*.c' RETURN n LIMIT 10` |
| Qualified symbols (file::symbol) | `MATCH (n:Function) WHERE n.qualified_name LIKE "review_repository::*" RETURN n LIMIT 20` |

### Example smoke (`example/linux`)

| Probe | GQL |
|-------|-----|
| Import (scale) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- [C++](cpp.md)
- Openspec: `c-include-graph`, `c-call-resolution`, `c-qualified-symbols`

# C++

Tier 1 plugin for C++ source and headers. Extracts class inheritance, template instantiation edges, and call resolution.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-cpp` (`CppPlugin`) |
| **Grammar** | `tree-sitter-cpp` |
| **Extensions** | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.hxx` |
| **Discover** | `rgctl discover . -l cpp -e build,cmake-build-debug,.rgctl --with-cfg` |
| **CFG / taint** | Enabled |

Tree-sitter node kinds: `function_definition`, `class_specifier`, `struct_specifier`, `enum_specifier`, `preproc_include`, `using_declaration`.

## What is extracted

### Nodes

- **Function** — free functions and methods
- **Class**, **Struct**, **Enum**
- **Import** — `#include` and `using` declarations

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Function and method calls |
| `EXTENDS` | Class inheritance (`: public Base`) |
| `INSTANTIATES` | Template and constructor instantiation |

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-cpp` |
| **Example corpus** | `example/llvm-project/clang` (`-l cpp`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-cpp.sh` |

## GQL verification queries

### Fixture probes

| Probe | GQL |
|-------|-----|
| Inheritance (EXTENDS) | `MATCH (a)-[:EXTENDS]->(b) RETURN a,b LIMIT 10000` |
| Instantiation (INSTANTIATES) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| Call resolution (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

### Example smoke (`example/llvm-project/clang`)

| Probe | GQL |
|-------|-----|
| EXTENDS (scale) | `MATCH (a)-[:EXTENDS]->(b) RETURN a,b LIMIT 10000` |
| INSTANTIATES (scale) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- [C](c.md)
- Openspec: `cpp-inheritance-edges`, `cpp-instantiation`, `cpp-call-resolution`

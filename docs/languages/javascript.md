# JavaScript

Tier 1 plugin for JavaScript (ES modules, CommonJS patterns, classes). Shared architecture with TypeScript plugin; no type-only syntax.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-javascript` (`JavaScriptPlugin`) |
| **Grammar** | `tree-sitter-javascript` |
| **Extensions** | `.js`, `.jsx`, `.mjs` |
| **Discover** | `rgctl discover . -l javascript -e node_modules --with-cfg` |
| **CFG / taint** | Enabled |

Tree-sitter node kinds: `function_declaration`, `method_definition`, `arrow_function`, `class_declaration`, `import_statement`.

## What is extracted

### Nodes

- **Function** — declarations, methods, arrow functions
- **Class**
- **Import** — ES module imports

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Call expressions |
| `EXTENDS` | `class Foo extends Bar` |
| `INSTANTIATES` | `new Foo()` |
| `Import` | Module import graph |

Method `qualified_name` uses `ClassName.method` form (e.g. `OrderService.checkout`).

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-javascript` |
| **Example corpus** | `example/node/test` (`-l javascript`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-javascript.sh` |

## GQL verification queries

### Fixture probes

| Probe | GQL |
|-------|-----|
| Module graph (Import) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| Heritage (EXTENDS) | `MATCH (a)-[:EXTENDS]->(b) RETURN a,b LIMIT 10000` |
| Instantiation (INSTANTIATES) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| Call resolution (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |
| Class method FQN (OrderService.*) | `MATCH (n:Function) WHERE n.qualified_name LIKE 'OrderService.*' RETURN n LIMIT 20` |

### Example smoke (`example/node/test`)

| Probe | GQL |
|-------|-----|
| Import (scale) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| EXTENDS (scale) | `MATCH (a)-[:EXTENDS]->(b) RETURN a,b LIMIT 10000` |
| INSTANTIATES (scale) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- [TypeScript](typescript.md) — shared extraction model with interfaces and decorators
- Openspec: `javascript-module-graph`, `javascript-heritage`, `javascript-call-resolution`

# TypeScript

Tier 1 plugin for TypeScript and TSX. Extends the JavaScript model with interfaces, `implements` clauses, and decorators.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-typescript` (`TypeScriptPlugin`) |
| **Grammar** | `tree-sitter-typescript` |
| **Extensions** | `.ts`, `.tsx` |
| **Discover** | `rgctl discover . -l typescript -e node_modules,dist --with-cfg` |
| **CFG / taint** | Enabled |

Tree-sitter node kinds: `function_declaration`, `method_definition`, `arrow_function`, `class_declaration`, `interface_declaration`, `import_statement`.

## What is extracted

### Nodes

- **Function**, **Class**, **Interface**
- **Import** — ES module imports

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Call expressions |
| `EXTENDS` | Class extends |
| `IMPLEMENTS` | Class implements interface |
| `ANNOTATEDWITH` | Decorators |
| `Import` | Module import graph |

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-typescript` |
| **Example corpus** | `example/vscode/src` (`-l typescript`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-typescript.sh` |

## GQL verification queries

### Fixture probes

| Probe | GQL |
|-------|-----|
| Module graph (Import) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| Heritage (EXTENDS) | `MATCH (a)-[:EXTENDS]->(b) RETURN a,b LIMIT 10000` |
| Implements (IMPLEMENTS) | `MATCH (a)-[:IMPLEMENTS]->(b) RETURN a,b LIMIT 10000` |
| Decorators (ANNOTATEDWITH) | `MATCH (a)-[:ANNOTATEDWITH]->(b) RETURN a,b LIMIT 10000` |
| Call resolution (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

### Example smoke (`example/vscode/src`)

| Probe | GQL |
|-------|-----|
| Import (scale) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| IMPLEMENTS (scale) | `MATCH (a)-[:IMPLEMENTS]->(b) RETURN a,b LIMIT 10000` |
| EXTENDS (scale) | `MATCH (a)-[:EXTENDS]->(b) RETURN a,b LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- [JavaScript](javascript.md)
- Openspec: `typescript-module-graph`, `typescript-heritage`, `typescript-decorators`, `typescript-call-resolution`

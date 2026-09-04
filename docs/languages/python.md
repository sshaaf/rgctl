# Python

Tier 1 plugin for Python 3 source. Extracts modules, classes, functions, imports, inheritance, decorators, instantiation, and call edges.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-python` (`PythonPlugin`) |
| **Grammar** | `tree-sitter-python` |
| **Extensions** | `.py`, `.pyw` |
| **Discover** | `rgctl discover . -l python -e .venv,__pycache__ --with-cfg` |
| **CFG / taint** | Enabled (`LanguageAnalysisProfile`) |

Tree-sitter node kinds (from `languages.toml`): `function_definition`, `class_definition`, `import_statement`, `import_from_statement`.

## What is extracted

### Nodes

- **Function** — module-level and class methods; `qualified_name` for class methods (e.g. `OrderService.checkout`)
- **Class** — classes with inheritance
- **Import** — `import` and `from … import` module graph
- **Module** — file-level module nodes

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Resolved function/method calls |
| `EXTENDS` | Class inheritance (`class Foo(Bar)`) |
| `ANNOTATEDWITH` | Decorators on functions and classes |
| `INSTANTIATES` | `new` / constructor calls (`Foo()`) |
| `Import` | Module import relations |

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-python` |
| **Example corpus** | `example/home-assistant` (`-l python`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-python.sh` |

```bash
RGCTL=target/release/rgctl ./rgctl-tests/gql-verification-smoke/verify-extraction-gql-python.sh
```

## GQL verification queries

Run after `discover` on the fixture. Replace `<repo>` with the fixture path.

### Fixture probes

| Probe | GQL |
|-------|-----|
| Module graph (Import) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| Heritage (EXTENDS) | `MATCH (a)-[:EXTENDS]->(b) RETURN a,b LIMIT 10000` |
| Decorators (ANNOTATEDWITH) | `MATCH (a)-[:ANNOTATEDWITH]->(b) RETURN a,b LIMIT 10000` |
| Instantiation (INSTANTIATES) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| Call resolution (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |
| Method FQN (OrderService.*) | `MATCH (n:Function) WHERE n.qualified_name LIKE 'OrderService.*' RETURN n LIMIT 20` |

### Example smoke (`example/home-assistant`)

| Probe | GQL |
|-------|-----|
| Import (scale) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| EXTENDS (scale) | `MATCH (a)-[:EXTENDS]->(b) RETURN a,b LIMIT 10000` |
| ANNOTATEDWITH (scale) | `MATCH (a)-[:ANNOTATEDWITH]->(b) RETURN a,b LIMIT 10000` |
| INSTANTIATES (scale) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- Openspec: `python-module-graph`, `python-heritage`, `python-decorators`, `python-call-resolution`

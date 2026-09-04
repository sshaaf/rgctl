# C#

Tier 1 plugin for C# source. Extracts namespaces, classes, attributes, `new` expressions, and method call binding with namespace-qualified names.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-csharp` (`CSharpPlugin`) |
| **Grammar** | `tree-sitter-c-sharp` |
| **Extensions** | `.cs` |
| **Discover** | `rgctl discover . -l csharp -e bin,obj,data --with-cfg` |
| **CFG / taint** | Enabled |

Tree-sitter node kinds: `method_declaration`, `local_function_statement`, `constructor_declaration`, `class_declaration`, `struct_declaration`, `interface_declaration`, `using_directive`.

## What is extracted

### Nodes

- **Function** — methods, local functions, constructors
- **Class**, **Struct**, **Interface**, **Enum**
- **Import** — `using` directives

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Method and constructor calls |
| `ANNOTATEDWITH` | Attributes (`[Authorize]`, …) |
| `INSTANTIATES` | `new` expressions |
| `EXTENDS` / `IMPLEMENTS` | Inheritance and interface implementation |

`qualified_name` uses namespace prefix (e.g. `Ecommerce.Services.OrderService.CheckoutAsync`).

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-csharp` |
| **Example corpus** | `example/roslyn/src` (`-l csharp`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-csharp.sh` |

## GQL verification queries

### Fixture probes

| Probe | GQL |
|-------|-----|
| Attributes (ANNOTATEDWITH) | `MATCH (a)-[:ANNOTATEDWITH]->(b) RETURN a,b LIMIT 10000` |
| Instantiation (INSTANTIATES) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| Call binding (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |
| Namespace FQN | `MATCH (n:Function) WHERE n.qualified_name LIKE 'Ecommerce.*' RETURN n LIMIT 20` |

### Example smoke (`example/roslyn/src`)

| Probe | GQL |
|-------|-----|
| ANNOTATEDWITH (scale) | `MATCH (a)-[:ANNOTATEDWITH]->(b) RETURN a,b LIMIT 10000` |
| INSTANTIATES (scale) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- Openspec: `csharp-annotations`, `csharp-instantiation`, `csharp-call-binding`, `csharp-namespace-fqn`

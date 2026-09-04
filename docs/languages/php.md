# PHP

Tier 1 plugin for PHP source. Covers namespaces, `use` imports, classes, traits, static calls, and cross-file resolution.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-php` (`PhpPlugin`) |
| **Grammar** | `tree-sitter-php` |
| **Extensions** | `.php` |
| **Discover** | `rgctl discover . -l php -e vendor,generated --with-cfg --with-taint` |
| **CFG / taint** | Enabled (taint on fixture discover) |

Tree-sitter node kinds: `function_definition`, `method_declaration`, `arrow_function`, `anonymous_function`, `class_declaration`, `interface_declaration`, `trait_declaration`, `namespace_use_declaration`.

## What is extracted

### Nodes

- **Function**, **Class**, **Interface**, **Trait**
- **Import** — `use` statements (including aliases)

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Function, method, and static calls |
| `Import` | Namespace import graph |
| `USES` | Trait composition *(openspec probe — may not emit yet)* |
| `ANNOTATEDWITH` | Attributes *(openspec probe)* |
| `INSTANTIATES` | `new` / anonymous class *(openspec probe)* |

Cross-file static call resolution is verified (`SampleService.run` → `AuthService.login`).

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-php` |
| **Example corpus** | `example/magento2` (`app lib setup -l php -e vendor -e generated`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-php.sh` |

## GQL verification queries

### Fixture probes

| Probe | GQL | Notes |
|-------|-----|-------|
| Namespace imports (Import) | `MATCH (n:Import) RETURN n LIMIT 10000` | |
| Import by name (AuthService) | `MATCH (n:Import) WHERE n.name = 'AuthService' RETURN n` | |
| Aliased import (Order) | `MATCH (n:Import) WHERE n.name = 'Order' RETURN n` | |
| Call resolution (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` | |
| Cross-file static call | `MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'run' AND b.name = 'login' RETURN a,b` | |
| Namespace FQN on Class | `MATCH (n:Class) WHERE n.name = 'AuthService' RETURN n` | |
| Method FQN (AuthService.login) | `MATCH (n:Function) WHERE n.name = 'login' RETURN n` | |
| Trait composition (USES) | `MATCH (a)-[:USES]->(b) RETURN a,b LIMIT 10000` | Soft probe |
| Attributes (ANNOTATEDWITH) | `MATCH (a)-[:ANNOTATEDWITH]->(b) RETURN a,b LIMIT 10000` | Soft probe |
| Anonymous class / new (INSTANTIATES) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` | Soft probe |

### Example smoke (`example/magento2`)

| Probe | GQL |
|-------|-----|
| Import (scale) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- Openspec: `php-trait-and-imports`, `php-framework-symbols`, `php-analysis-polish`

# Go

Tier 1 plugin for Go source. Covers structs, interfaces, embedding, generics, type aliases, constants, and import paths.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-go` (`GoPlugin`) |
| **Grammar** | `tree-sitter-go` |
| **Extensions** | `.go` |
| **Discover** | `rgctl discover . -l go -e vendor --with-cfg` |
| **CFG / taint** | Enabled |

Tree-sitter node kinds: `function_declaration`, `method_declaration`, `type_declaration`, `import_declaration`.

Struct embedding maps to `EXTENDS`; interface satisfaction maps to `IMPLEMENTS`. See [go-language-coverage.md](../design/go-language-coverage.md).

## What is extracted

### Nodes

- **Function**, **Struct**, **Interface**, **TypeAlias**, **Variable** (consts)
- **Import** — package import paths (`fmt`, local packages)

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Function and method calls |
| `IMPLEMENTS` | Struct satisfies interface |
| `EXTENDS` | Struct embedding |
| `Import` | Package imports |

## Verification

| | |
|---|---|
| **Fixture** | `rgctl-tests/ecommerce-go` |
| **Example corpus** | `example/kubernetes` (`-l go -e vendor`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-go.sh` |

## GQL verification queries

### Fixture probes

| Probe | GQL |
|-------|-----|
| LF-05 implements (LfRemoteRuntime) | `MATCH (a:Struct)-[:IMPLEMENTS]->(b:Interface) WHERE a.name = 'LfRemoteRuntime' RETURN a,b` |
| LF-06 embed extends (LfDerived) | `MATCH (a:Struct)-[:EXTENDS]->(b:Struct) WHERE a.name = 'LfDerived' RETURN a,b` |
| LF-10 const (LfStatusPending) | `MATCH (n:Variable) WHERE n.name = 'LfStatusPending' RETURN n` |
| LF-10 type alias (LfUserID) | `MATCH (n:TypeAlias) WHERE n.name = 'LfUserID' RETURN n` |
| LF-16 generics (LfIdentity) | `MATCH (n:Function) WHERE n.name = 'LfIdentity' RETURN n` |
| LF-16 generics (LfBox) | `MATCH (n:Struct) WHERE n.name = 'LfBox' RETURN n` |
| LF-17 import (fmt) | `MATCH (n:Import) WHERE n.name = 'fmt' RETURN n` |
| LF-17 import (timeutil) | `MATCH (n:Import) WHERE n.name = 'timeutil' RETURN n` |
| Call resolution (CALLS) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

### Example smoke (`example/kubernetes`)

| Probe | GQL |
|-------|-----|
| Import (scale) | `MATCH (n:Import) RETURN n LIMIT 10000` |
| IMPLEMENTS (scale) | `MATCH (a)-[:IMPLEMENTS]->(b) RETURN a,b LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- [Go language coverage](../design/go-language-coverage.md)

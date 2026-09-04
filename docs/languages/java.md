# Java

Tier 1 plugin for Java source including JPMS modules, annotations, generics, lambdas, and qualified names.

## Implementation

| | |
|---|---|
| **Plugin crate** | `crates/rgctl-lang-java` (`JavaPlugin`) |
| **Grammar** | `tree-sitter-java` |
| **Extensions** | `.java` |
| **Discover** | `rgctl discover . -l java -e target,data --with-cfg` |
| **CFG / taint** | Enabled; Kantra rules with `--with-kantra` |

Tree-sitter node kinds: `method_declaration`, `class_declaration`, `interface_declaration`, `enum_declaration`, `import_declaration`.

## What is extracted

### Nodes

- **Function** — methods and constructors; `is_lambda`, generic/throws properties
- **Class**, **Interface**, **Enum**
- **Module** — JPMS `module-info.java`
- **Import** — import declarations

### Edges

| Edge | Meaning |
|------|---------|
| `CALLS` | Method and constructor calls |
| `INSTANTIATES` | `new` expressions |
| `ANNOTATED_WITH` | Annotations on types and members |
| `REFERENCES` | Field and class literal references |
| `DEPENDSON` | JPMS module dependencies (`Module` → target) |
| `EXTENDS` / `IMPLEMENTS` | Class hierarchy |

## Verification

| | |
|---|---|
| **GQL fixture** | `tests/fixtures/java/langfeatures` |
| **Command fixture** | `rgctl-tests/ecommerce-java` |
| **Example corpus** | `example/metasfresh-4.9.8b` (`discover --full`) |
| **Smoke script** | `rgctl-tests/gql-verification-smoke/verify-extraction-gql-java.sh` |

```bash
RGCTL=target/release/rgctl ./rgctl-tests/gql-verification-smoke/verify-extraction-gql-java.sh
```

## GQL verification queries

### Langfeatures probes (`tests/fixtures/java/langfeatures`)

| Probe | GQL |
|-------|-----|
| JF-01 instantiates (String) | `MATCH (a:Function)-[:INSTANTIATES]->(b) WHERE a.name = 'instantiates' RETURN a,b` |
| JF-02 annotated with (NonNull) | `MATCH (a:Function)-[:ANNOTATED_WITH]->(b) WHERE a.name = 'typeUse' RETURN a,b` |
| JF-03 references (field/class literal) | `MATCH (a:Function)-[:REFERENCES]->(b) WHERE a.name = 'fieldAndClassLiteral' RETURN a,b` |
| JF-04 module depends on (JPMS) | `MATCH (m:Module)-[:DEPENDSON]->(t) RETURN m,t` |
| JF-05 lambda (is_lambda) | `MATCH (f:Function) WHERE f.is_lambda = 'true' RETURN f LIMIT 20` |
| JF-06 generic/throws properties | `MATCH (f:Function) WHERE f.name = 'genericThrows' RETURN f` |
| JF-07 class FQN (qualified_name) | `MATCH (n:Class) WHERE n.qualified_name = 'demo.LangFeatures' RETURN n` |
| JF-07 FQN LIKE filter | `MATCH (n:Class) WHERE n.qualified_name LIKE 'demo.*' RETURN n` |

### Example smoke (`example/metasfresh-4.9.8b`)

| Probe | GQL |
|-------|-----|
| Class (scale) | `MATCH (n:Class) RETURN n LIMIT 10000` |
| CALLS (scale) | `MATCH (a)-[:CALLS]->(b) RETURN a,b LIMIT 10000` |
| INSTANTIATES (scale) | `MATCH (a)-[:INSTANTIATES]->(b) RETURN a,b LIMIT 10000` |

## Related

- [Languages index](README.md)
- [tier-1-language-support.md](../tier-1-language-support.md) — Java is the Layer F reference

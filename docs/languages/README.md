# Languages

rgctl indexes source through **Tier 1 custom language plugins** (`LanguagePlugin` + tree-sitter). Each guide below documents what a language extracts, how to discover it, and the GQL probes from the [gql-verification-smoke](../../rgctl-tests/gql-verification-smoke/) scripts.

**Metadata source of truth:** [`languages.toml`](../../languages.toml) · **Contributor bar:** [tier-1-language-support.md](../tier-1-language-support.md)

## Tier 1 languages

| Language | Extensions | Smoke script |
|----------|------------|--------------|
| [C](c.md) | `.c`, `.h` | `verify-extraction-gql-c.sh` |
| [C++](cpp.md) | `.cpp`, `.hpp`, … | `verify-extraction-gql-cpp.sh` |
| [C#](csharp.md) | `.cs` | `verify-extraction-gql-csharp.sh` |
| [Go](go.md) | `.go` | `verify-extraction-gql-go.sh` |
| [Java](java.md) | `.java` | `verify-extraction-gql-java.sh` |
| [JavaScript](javascript.md) | `.js`, `.jsx`, `.mjs` | `verify-extraction-gql-javascript.sh` |
| [PHP](php.md) | `.php` | `verify-extraction-gql-php.sh` |
| [Python](python.md) | `.py`, `.pyw` | `verify-extraction-gql-python.sh` |
| [Rust](rust.md) | `.rs` | `verify-extraction-gql-rust.sh` |
| [TypeScript](typescript.md) | `.ts`, `.tsx` | `verify-extraction-gql-typescript.sh` |

## Tiers

| Tier | Handler | Indexing | CFG / PDG / taint |
|------|---------|----------|-------------------|
| **Tier 1** | Custom `LanguagePlugin` | Rich symbols + `Calls` | Full when `--with-cfg` / taint enabled |
| **Tier 2** | Generic tree-sitter | From `LanguageConfig` | Limited |
| **Tier 3** | Regex | Pattern symbols | None |

All guides in this section are **Tier 1**.

## Discover depth

| Flags | Use |
|-------|-----|
| (default) | Fast graph + metrics |
| `--with-cfg` | CFG/PDG for slice, inspect, cpg |
| `--with-taint` | Discover-time taint (with CFG) |
| `--full` | Full pipeline (used on large Java example corpora) |

```bash
rgctl discover . -l python,go,rust
rgctl discover . -e node_modules,target,.git
```

## Run all smoke tests

```bash
cargo build --release --bin rgctl
RGCTL_SKIP_EXAMPLE=1 ./rgctl-tests/gql-verification-smoke/run-all-extraction-gql.sh
```

## Related

- [Graph Query Language guide](../guides/graph-query-language.md)
- [Discovering and indexing](../guides/discovering-and-indexing.md)
- [rgctl-tests README](../../rgctl-tests/README.md#extraction-depth-gql--rgctl-command-verification)
- **Markdown / docs:** separate markup plugin — [markdown-context.md](../markdown-context.md)

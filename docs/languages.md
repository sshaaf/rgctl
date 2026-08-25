# Languages

rgBuilder indexes source through **language plugins**. Tier metadata: [`languages.toml`](../languages.toml).

**Contributors adding a language:** [tier-1-language-support.md](tier-1-language-support.md).

## Tiers

| Tier | Handler | Indexing | CFG / PDG / taint |
|------|---------|----------|-------------------|
| **Tier 1** | Custom `LanguagePlugin` | Rich symbols + `Calls` | Full when `--with-cfg` / taint enabled |
| **Tier 2** | Generic tree-sitter | From `LanguageConfig` | Limited |
| **Tier 3** | Regex | Pattern symbols | None |

## Tier 1 (always in the release binary)

| Language | Extensions | Notes |
|----------|------------|-------|
| Java | `.java` | Strong golden coverage |
| Go | `.go` | |
| Rust | `.rs` | |
| Python | `.py`, `.pyw` | |
| JavaScript | `.js`, `.jsx`, `.mjs` | |
| TypeScript | `.ts`, `.tsx` | |
| C# | `.cs` | |
| C | `.c`, `.h` | |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp`, … | |

```bash
rgctl discover . -l java,go,rust
rgctl discover . -e node_modules,target,.git
```

Config format plugins (JSON, YAML, TOML, properties, …) add structure nodes; they do not run CFG/PDG. See `crates/rgbuilder-config-formats`.

**Markdown / docs:** custom markup plugin `rgbuilder-lang-markdown` (tree-sitter-md). Not Tier 1, not generic Tier 2. See [markdown-context.md](markdown-context.md). `.md` and `.mdx` are indexed by default when registered via `rgbuilder-languages`.

## Discover depth

| Flags | Use |
|-------|-----|
| (default) | Fast graph + metrics |
| `--with-security` | Secret scanning on config-like files |
| `--with-cfg` | CFG/PDG for slice/inspect/cpg (slower on huge repos) |
| `--with-taint` | Discover-time taint (with CFG) |
| `--export-migration-hints` | Migration plan JSON |
| `--with-dashboard` | Optional static UI assets |

## CLI tips

- **GQL** / **blast-radius** use indexed node names (often bare method names in Java).
- **`inspect SYMBOL`** is a function symbol only (no `--class`).
- **`blast-radius`** supports `--class` / `--file` on collisions.
- **`slice --function`** is the method/function name, not the class.

See [User Guide §4](user-guide.md#4-index-with-discover).

# rgctl-kantra

Native Konveyor Kantra rule evaluator for [rgctl](https://github.com/sshaaf/rgctl).

## Embedded rules catalog

Release builds embed Konveyor `stable/java` rules as a compiled `RBKC` blob (`build.rs` → `include_bytes!`). No runtime YAML walk is required for `rgctl discover --with-kantra`.

### Submodule (source builds)

The catalog source is vendored as a git submodule:

```text
crates/rgctl-kantra/assets/rulesets  →  https://github.com/konveyor/rulesets
```

After cloning rgctl:

```bash
git submodule update --init crates/rgctl-kantra/assets/rulesets
```

The parent repo pins a specific commit (currently **v0.9.2** / `022bbd34`). To match that tag explicitly:

```bash
cd crates/rgctl-kantra/assets/rulesets
git fetch --tags
git checkout v0.9.2
```

When the submodule is absent, `build.rs` falls back to `tests/fixtures/kantra-rules/` so CI and partial checkouts still compile.

### Catalog identity

Embedded catalogs record provenance in `catalog_id`:

```text
stable-java@<rulesets-git-sha>
```

Example: `stable-java@022bbd34b34eca53d04b6cb2b97b27e47fef479b`

Fixture fallback uses `fixture@<content-hash>`.

## CLI

```bash
rgctl discover . --with-kantra                         # embedded catalog
rgctl discover . --with-kantra --kantra-target quarkus # target filter
rgctl discover . --with-kantra --kantra-rules PATH     # override (fixtures/CI)
rgctl discover . --with-kantra --kantra-catalog ROOT   # local rulesets tree
```

## Attribution

Konveyor rulesets are Apache-2.0. See [NOTICE](NOTICE) and [konveyor/rulesets](https://github.com/konveyor/rulesets).

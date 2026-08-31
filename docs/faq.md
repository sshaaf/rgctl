# FAQ

Short answers for common first-hour questions. Commands → [User Guide](user-guide.md). Terms → [Glossary](glossary.md).

### Where are `.rgctl/` artifacts stored?

**Default (background daemon):** `~/.rgctl/cache/{reponame}/.rgctl/` — not in your source tree. **CI / in-repo:** `rgctl --no-daemon discover .` writes `{repo}/.rgctl/`. See [Installation — Daemon vs no-daemon](installation.md#daemon-vs-no-daemon).

### I ran `discover` but queried the wrong repo

`rgctl -r PATH discover .` indexes **shell cwd**, not `PATH`. Use `cd repo && rgctl discover .` or `rgctl -r PATH discover` (no trailing `.`).

### Discover vs semantic index?

`discover` builds the **code knowledge graph** and reachability caches. `semantic index` is a separate **opt-in** embedding index for natural-language / keyword search. Run discover first, then `rgctl semantic index` if you need Search / `semantic query`. For **doc sections**, use `semantic index --scope docs` (indexes headings + code blocks); query embedder is fixed at index time — see [markdown-context.md](markdown-context.md#semantic-search-doc-sections).

### When do I need `--with-cfg`?

For CFG/PDG archives used by `inspect`, `slice`, `cpg`, and discover-time taint (`--with-taint` implies the CFG pass). Plain `gql` / `blast-radius` / `metrics` work after a default discover.

### Why is the dashboard empty after `discover .`?

Dashboard export is **off by default**. Pass `--with-dashboard`, then `rgctl serve --open`.

### How do I get a migration plan?

```bash
rgctl discover . --with-cfg --with-security --with-taint \
  --with-dashboard --with-harmonic --export-migration-hints
```

### code-daemon vs vocab vs hash?

| Embedder | When |
|----------|------|
| `vocab` (default) | Fast compiled token table; no ONNX; declaration metadata only |
| `code-daemon` | Higher quality code retriever; needs `git lfs pull` for ONNX weights (~206 MB) |
| `hash` | Offline smoke / CI without model weights |

`--embed-bodies` (off by default) re-reads function source and appends identifier tokens. Query fusion still uses discover token-blooms without this flag.

### What does exit code 1 mean?

Usually a **policy violation** (`check`, or `blast-radius --policy-file`) or a command error. JSON still may be on stdout for some commands — see [json-api.md](json-api.md#13-exit-codes).

### Is there an `--all` flag?

No. Combine `--with-cfg --with-security --with-taint` (and dashboard/migration flags) explicitly.

### Louvain or label propagation?

rgctl runs **label propagation** (Raghavan 2007). The field `louvain_community_id` is a historical name only.

### Coolstore or ecommerce-java?

Prefer the in-tree **ecommerce-java** fixture in [User Guide §3](user-guide.md#3-example-project-ecommerce-java).

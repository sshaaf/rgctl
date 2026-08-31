# rgctl integration test matrix

How to verify CLI subprocess integration before shipping changes. Shared harness: `tests/rgctl_harness.rs`.

**Quick PR gate (Tier A):**

```bash
cargo test --test rgctl_no_daemon -- --test-threads=1
```

---

## Tier overview

| Tier | When | Corpus | Confidence |
|------|------|--------|------------|
| **A** | Every PR | `tests/fixtures/tiny_polyglot_repo` (temp copy) | In-repo `.rgctl/` layout, gql/metrics, discover target pitfalls |
| **B** | Manual / nightly | `example/linux`, metasfresh, … | Scale + cold perf baselines |

---

## Tier A — fast CI

### Test targets

| File | Tests | What it proves |
|------|-------|----------------|
| `tests/rgctl_no_daemon.rs` | 5 (+ 1 ignored) | Artifacts under `{repo}/.rgctl/`; gql/metrics; `-r` + `discover .` pitfall; absolute-path discover |

### Commands

```bash
cargo test --test rgctl_no_daemon -- --test-threads=1
```

### Harness conventions

| Pattern | Use |
|---------|-----|
| `cd repo && rgctl discover .` | Correct indexing into `{repo}/.rgctl/` |
| `-r OTHER discover .` from another cwd | **Indexes cwd, not `-r`** — regression test documents this |
| `discover /abs/path/to/repo` | Works from any cwd |

### Tier A corpora

- **tiny_polyglot_repo** — copied to `tempfile`; no `.rgctl` in git fixture tree.

---

## Tier B — corpus / perf (ignored)

| File | Test | Corpus | Wall (reference M3 Pro) |
|------|------|--------|---------------------------|
| `tests/rgctl_no_daemon.rs` | `linux_no_daemon_discover_and_gql_smoke` | `example/linux` | ~145 s discover |
| `tests/cold_profile_gates.rs` | `linux_cold_discover_within_baseline` | `example/linux` | baseline 145 s |
| `tests/cold_profile_gates.rs` | `metasfresh_cold_discover_within_baseline` | `example/metasfresh-4.9.8b` | baseline 74 s (`--full`) |

```bash
cargo build --release --bin rgctl
cargo test --release --test rgctl_no_daemon linux_no_daemon -- --ignored --nocapture
cargo test --release --test cold_profile_gates -- --ignored --nocapture --test-threads=1
```

**Discover cwd:** cold gates use `.current_dir(repo)` + `discover .` — not `-r repo discover .` from project root.

Override paths: `RGCTL_LINUX_REPO`, `METASFRESH_REPO`.

---

## Golden ecommerce corpus

`rgctl-tests/ecommerce-java/` (~993 nodes, Java). Use when validating CLI JSON payloads on a real indexed app:

```bash
export RGCTL=/path/to/rbuilder/target/debug/rgctl
export ECOMM=/path/to/rbuilder/rgctl-tests/ecommerce-java

cd "$ECOMM"
"$RGCTL" discover . --languages java -e target,data
"$RGCTL" semantic index
"$RGCTL" -f json gql 'MATCH (n:Function) RETURN n LIMIT 5'
```

Reference symbols from [ecommerce-java README](../../rgctl-tests/ecommerce-java/README.md).

---

## Related tests (not in matrix above)

| File | Role |
|------|------|
| `tests/discover_full_serve.rs` | `--full` pipeline + foreground serve |
| `tests/cold_profile_gates.rs` | kafka, k8s-website markdown, obsidian export gates |
| Dashboard `tests/dashboard_ecommerce_*.rs` | Per-language dashboard bundles |

---

## Checklist before merging CLI changes

- [ ] Tier A green with `--test-threads=1`
- [ ] CLI `-f json` payloads still match [json-api.md](../json-api.md) shapes
- [ ] If touching discover perf: Tier B cold gate on reference machine (optional)

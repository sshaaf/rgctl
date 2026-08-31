# Cold discover profile (maintainer)

How to run **accurate** discover timings on large local checkouts under `example/`. This doc replaces the old scratch file `temp.md`.

**Scope:** developer-machine smoke tests — **not** isolated CI. Numbers vary with CPU governor, background apps, disk cache, and thermal throttling. Use gates for regression bounds; use manual runs here to inspect `[profile]` stages.

---

## Reference machine (Aug 2026)

Hardware and OS where the **latest numbers below** were recorded. This is a **developer laptop**, not a dedicated benchmark host — treat timings as directional, not reproducible across machines.

| Field | Value |
|-------|--------|
| Model | MacBook Pro (Apple Silicon) |
| CPU | **Apple M3 Pro** — 12 cores (6 performance + 6 efficiency) |
| RAM | **36 GB** |
| OS | macOS **darwin 25.6.0** (Sequoia family) |
| Binary | `target/release/rgctl` (release build immediately before each run) |
| Mode | **`--no-daemon`** (in-process; artifacts in `{corpus}/.rgctl/`) |
| Isolation | **None** — desktop apps, thermal limits, and APFS cache affect wall time |

Re-profile on **your** hardware before changing baselines in `tests/cold_profile_gates.rs`.

---

## Primary corpora

Large trees live under **`example/`** (gitignored). Fetch with `./scripts/fetch-profile-repos.sh` or clone manually. Override paths with `RGCTL_LINUX_REPO` / `METASFRESH_REPO`.

| Corpus | Path | Typical use |
|--------|------|-------------|
| **Linux kernel** | `example/linux/` | Default discover (no CFG / dashboard / harmonic) — scale ingest + analysis |
| **metasfresh** | `example/metasfresh-4.9.8b/` | Deep / full pipeline — CFG, dashboard export, optional semantic |

See also `example/README.md` (kafka, k8s-website markdown gates).

---

## Cold profile policy

1. **Release binary only:** `cargo build --release --bin rgctl`
2. **Delete artifacts:** `rm -rf example/<corpus>/.rgctl/` (and daemon cache if you used the daemon — not for `--no-daemon` runs)
3. **`--no-daemon`:** avoids auto-start daemon + `~/.rgctl/cache/` (keeps gates comparable to pre-daemon baselines)
4. **Run from the corpus directory** (see pitfall below)
5. **Logging:** `RUST_LOG=info,profile=info` and `discover … -v`

Warm or partial `.rgctl/` caches **invalidate** wall times (often seconds instead of minutes).

---

## Commands

### Automated gates (preferred for regression)

```bash
cargo build --release --bin rgctl
cargo test --release --test cold_profile_gates -- --ignored --nocapture --test-threads=1
```

| Gate | Corpus | Discover flags | Baseline (wall, +10%) |
|------|--------|----------------|------------------------|
| `linux_cold_discover_within_baseline` | `example/linux` | default | **145 s** |
| `metasfresh_cold_discover_within_baseline` | `example/metasfresh-4.9.8b` | `--full` | **74 s** |
| `kafka_cold_discover_within_baseline` | `example/kafka` | default | env `RGCTL_KAFKA_COLD_BASELINE_SECS` (default 600 s) |
| `k8s_website_markdown_cold_discover_within_baseline` | `example/k8s-website` | `-l markdown` | **3 s** |
| `ecommerce_java_inheritance_cold_discover_within_baseline` | `rgctl-tests/ecommerce-java` | default | **0.31 s** wall; **0.008 s** `index_graph_build` |
| `ecommerce_java_kantra_cold_discover_within_baseline` | `rgctl-tests/ecommerce-java` | `--with-kantra --kantra-rules <fixture>` | env `RGCTL_ECOMMERCE_JAVA_KANTRA_*` (fixture catalog; fast CI path) |

Gates call `run_cold_discover_timed` in `tests/cold_profile_gates.rs` (`--no-daemon`, `-r <corpus>`, `discover . -v`).

**ecommerce-java gate** (inheritance external stubs): small Java fixture; asserts wall time and `[profile] stage index_graph_build` after `Extends`/`Implements`/`Permits` stub edges. Override with `RGCTL_ECOMMERCE_JAVA_COLD_BASELINE_SECS` / `RGCTL_ECOMMERCE_JAVA_INDEX_GRAPH_BUILD_BASELINE_SECS`.

### Manual profile (stage breakdown)

**Important:** `discover .` sets the session root from the positional `.` (cwd). **`-r /path/to/linux` is ignored** when you pass `.`. Always `cd` into the corpus (or pass an absolute path instead of `.`).

**Linux — default discover:**

```bash
cargo build --release --bin rgctl
rm -rf example/linux/.rgctl
cd example/linux
/usr/bin/time -l env RUST_LOG=info,profile=info \
  ../../target/release/rgctl --no-daemon -f json discover . -v \
  2>&1 | tee /tmp/linux-cold-profile.log
grep '\[profile\]' /tmp/linux-cold-profile.log
```

**metasfresh — full pipeline** (same as cold gate):

```bash
rm -rf example/metasfresh-4.9.8b/.rgctl
cd example/metasfresh-4.9.8b
/usr/bin/time -l env RUST_LOG=info,profile=info \
  ../../target/release/rgctl --no-daemon -f json discover . --full -v \
  2>&1 | tee /tmp/metasfresh-full-profile.log
```

Optional single-pass deep discover (not the cold gate):

```bash
cd example/metasfresh-4.9.8b
rm -rf .rgctl
/usr/bin/time -l env RUST_LOG=info,profile=info \
  ../../target/release/rgctl --no-daemon -f json discover . \
  --with-cfg --with-security --with-taint -v \
  2>&1 | tee /tmp/metasfresh-deep-profile.log
```

---

## Reading `[profile]` output

| Line | Meaning |
|------|---------|
| `[profile] discover summary` | Wall time, `index_secs`, `post_index_secs`, peak RSS (`peak_rss_mb`, `ingest_peak_rss_mb`, `analysis_peak_rss_mb`), node/function counts |
| `[profile] stage` | Per-stage wall seconds and `%` of discover wall (`index_extract`, `index_graph_build`, `centrality`, `cfg_total`, `save_dashboard`, `kantra_eval`, `kantra_index`, …) |
| `[profile] centrality breakdown` | PageRank / betweenness / harmonic sub-times |
| `[profile] save_dashboard stage` | Dashboard export substeps (e.g. `export_cfg_slice`) |
| `[profile] cfg cpu stage` | CFG thread CPU sums (can exceed wall on parallel passes) |

Harmonic runs only when `--with-harmonic` or **`discover --full`** (deep stage). Default linux discover skips harmonic and dashboard export.

### Kantra stages (`--with-kantra`)

| Stage | When | Notes |
|-------|------|-------|
| `kantra_load` | Eval | Catalog decode / engine setup |
| `kantra_eval` | Eval | Total eval wall (includes sub-stages below) |
| `kantra_filecontent` | Eval | `builtin.filecontent` / `builtin.file` |
| `kantra_referenced` | Eval | `go.referenced` / `java.referenced` |
| `kantra_compose` | Eval | `and` / `or` / `not` composition |
| `kantra_index` | After analysis persist | Rewrites `graph.snapshot.bin` with `KantraRule` nodes (runs after all cold mmap use) |

Cold gate `ecommerce_java_kantra_cold_discover_within_baseline` uses `--kantra-rules tests/fixtures/kantra-rules` (small fixture, stable timing). Embedded-catalog discover on the same corpus is heavier (~2.6k rules); profile manually when bumping the rulesets submodule pin.

Algorithm detail (sampled betweenness, HyperBall, adaptive gating): [analysis-architecture.md](../analysis-architecture.md), [harmonic-centrality.md](../harmonic-centrality.md), [graph-metrics-design.md](../design/graph-metrics-design.md).

---

## Gate baselines (2026-08-25)

Recorded on the **reference machine** above, release `rgctl`, **`--no-daemon`**, cold `.rgctl/` removed. These values are the **`cold_profile_gates`** baselines (+10% tolerance).

### Linux (`example/linux`) — default discover

| Metric | Value |
|--------|-------|
| Wall (real / profile) | **144.7 s** / **143.5 s** |
| **Gate baseline** | **145 s** (pass ≤ 159.5 s) |
| Peak RSS | **~14.7 GB** |
| User / sys CPU | 250.6 s / 47.1 s |
| Nodes / functions | 2,657,548 / 1,862,845 |
| Files indexed | 71,341 / 71,348 discovered |

Top stages (% of wall): `index_extract` **71.9 s** (50%), `index_graph_build` **19.2 s** (13%), `save_tracker` **9.4 s** (7%).

### metasfresh (`example/metasfresh-4.9.8b`) — `--full`

| Metric | Value |
|--------|-------|
| Wall (real) | **73.9 s** |
| **Gate baseline** | **74 s** (pass ≤ 81.4 s) |
| Peak RSS | **~6.4 GB** |
| User / sys CPU | 242.6 s / 64.4 s |
| Nodes / functions | 319,175 / 133,112 |

Stage walls (profile summaries): **basic ~18 s**, **deep ~46 s**, **semantic ~9 s** (inferred from timestamps; semantic has little `[profile]` logging).

Deep-pass hotspots: `cfg_total` **~18 s**, `save_dashboard` **~11 s** (`export_cfg_slice` **~9.7 s**), harmonic centrality **~6.7 s**, `field_write` **~4.6 s**.

---

## Related

- `tests/cold_profile_gates.rs` — baselines and `TOLERANCE` (+10%)
- `example/README.md` — fetch script and optional corpora
- `docs/analysis-architecture.md` — discover pipeline and centrality gating
- `AGENTS.md` — agent cold-profile recipe

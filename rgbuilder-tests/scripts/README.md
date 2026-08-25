# Scripts

## `run_rgbuilder_report.py`

Runs the full rgBuilder feature matrix against all Tier 1 `ecommerce-*` apps and publishes reports under `rgbuilder-reports/`.

### Outputs

| File | Description |
|------|-------------|
| `rgbuilder-reports/REPORT.md` | Cross-project **summary** report |
| `rgbuilder-reports/REPORT.html` | HTML summary |
| `rgbuilder-reports/languages/<id>.md` | **Comprehensive** per-language report |
| `rgbuilder-reports/languages/<id>.html` | Per-language HTML report |
| `rgbuilder-reports/README.md` | Index linking all artifacts |
| `rgbuilder-reports/all-results.json` | Combined JSON results |
| `rgbuilder-reports/<id>-summary.json` | Per-project summaries |
| `rgbuilder-reports/<id>-metrics.json` | Raw metrics output |
| `rgbuilder-reports/<id>-blast.json` | Raw blast-radius output (checkout target) |
| `rgbuilder-reports/<id>-blast-top.json` | Top blast scores from full function scan |
| `rgbuilder-reports/<id>-export.json` | Exported function subgraph |

### Usage

```bash
# from rgbuilder-tests/ (auto-detect: PATH, RGBUILDER, or ../../target/{release,debug}/rgctl when embedded)
./scripts/run_rgbuilder_report.sh

# explicit binary + refresh README summary tables
RGBUILDER=/path/to/rgctl ./scripts/run_rgbuilder_report.py --update-readmes

# subset of projects, keep existing .rgbuilder caches
./scripts/run_rgbuilder_report.py --projects rust java --no-clean
```

### Options

| Flag | Description |
|------|-------------|
| `--rgctl PATH` | rgctl binary |
| `--output-dir PATH` | default: `rgbuilder-reports/` |
| `--repo-root PATH` | default: parent of `scripts/` |
| `--no-clean` | skip deleting `.rgbuilder/` before discover |
| `--update-readmes` | sync summary tables into root + project READMEs |
| `--projects rust python …` | run subset only |
| `--blast-top N` | keep top N blast scores per project (default: 10) |
| `--skip-blast-scan` | skip full function blast scan (faster; omits top scores) |

Exit code **0** if every project `discover` succeeds; **1** otherwise.

### Graph correctness

Hand-labeled facts live in `ecommerce-*/correctness/expected-facts.json` (see [`correctness/SCHEMA.md`](../correctness/SCHEMA.md)). They are checked by `cargo test --test graph_correctness` in the rgBuilder repo (not this report script).

### Install rgctl from GitHub Releases

[`install_rgbuilder_release.sh`](install_rgbuilder_release.sh) downloads the platform archive published by [rgBuilder releases](https://github.com/sshaaf/rgBuilder/releases):

```bash
./scripts/install_rgbuilder_release.sh
RGBUILDER_TAG=v0.1.0 ./scripts/install_rgbuilder_release.sh
RGBUILDER=/path/to/.rgbuilder-bin/rgctl ./scripts/run_rgbuilder_report.py
```

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `RGBUILDER_REPO` | `sshaaf/rgBuilder` | GitHub `owner/repo` |
| `RGBUILDER_TAG` | _(latest)_ | Release tag, e.g. `v0.1.0` |
| `RGBUILDER_TARGET` | auto-detect | Rust triple, e.g. `x86_64-unknown-linux-gnu` |
| `RGBUILDER_INSTALL_DIR` | `.rgbuilder-bin/` | Extract destination |
| `GITHUB_TOKEN` / `GH_TOKEN` | — | Optional; higher API rate limits / private release assets |

### GitHub Actions

[`.github/workflows/rgbuilder-report.yml`](../.github/workflows/rgbuilder-report.yml) runs when:

1. **rgBuilder publishes a release** — the [rgBuilder release workflow](https://github.com/sshaaf/rgBuilder/blob/main/.github/workflows/release.yml) sends a `repository_dispatch` (`rgbuilder-released`) with the new tag.
2. **Manual run** — Actions → **rgBuilder Report** → Run workflow.

The workflow:

1. Downloads the matching rgctl binary from GitHub Releases
2. Runs `./scripts/run_rgbuilder_report.py --no-clean`
3. Packages `rgbuilder-reports/` as **`rgbuilder-reports-<tag>-<run_id>.tar.gz`** (+ SHA256 sidecar)
4. Uploads the archive as a **workflow artifact** (90-day retention) — nothing is committed to git

#### Setup (one-time)

In **sshaaf/rgBuilder** repository secrets:

| Secret | Purpose |
|--------|---------|
| `RGBUILDER_TESTS_DISPATCH_TOKEN` | PAT (classic `repo` or fine-grained **Actions: write** on `rgbuilder-tests`) to trigger the report workflow |

Optional in **sshaaf/rgbuilder-tests**:

| Secret | Purpose |
|--------|---------|
| `RGBUILDER_DOWNLOAD_TOKEN` | Only if rgBuilder releases are private |

#### Download a report

Open the workflow run → **Artifacts** → download `rgbuilder-reports-<tag>-<run_id>.tar.gz`.

```bash
tar -xzf rgbuilder-reports-v0.1.0-123456789.tar.gz
open rgbuilder-reports/REPORT.html
```

### Requirements

- `rgctl` built with language bundles (uses `discover . --cfg`)
- [`rgbuilder-policy.json`](../rgbuilder-policy.json) at repo root (for `check`)

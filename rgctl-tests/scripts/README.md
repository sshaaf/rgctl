# Scripts

## `run_rgctl_report.py`

Runs the full rgctl feature matrix against all Tier 1 `ecommerce-*` apps and publishes reports under `rgctl-reports/`.

### Outputs

| File | Description |
|------|-------------|
| `rgctl-reports/REPORT.md` | Cross-project **summary** report |
| `rgctl-reports/REPORT.html` | HTML summary |
| `rgctl-reports/languages/<id>.md` | **Comprehensive** per-language report |
| `rgctl-reports/languages/<id>.html` | Per-language HTML report |
| `rgctl-reports/README.md` | Index linking all artifacts |
| `rgctl-reports/all-results.json` | Combined JSON results |
| `rgctl-reports/<id>-summary.json` | Per-project summaries |
| `rgctl-reports/<id>-metrics.json` | Raw metrics output |
| `rgctl-reports/<id>-blast.json` | Raw blast-radius output (checkout target) |
| `rgctl-reports/<id>-blast-top.json` | Top blast scores from full function scan |
| `rgctl-reports/<id>-export.json` | Exported function subgraph |

### Usage

```bash
# from rgctl-tests/ (auto-detect: PATH, RGCTL, or ../../target/{release,debug}/rgctl when embedded)
./scripts/run_rgctl_report.sh

# explicit binary + refresh README summary tables
RGCTL=/path/to/rgctl ./scripts/run_rgctl_report.py --update-readmes

# subset of projects, keep existing .rgctl caches
./scripts/run_rgctl_report.py --projects rust java --no-clean
```

### Options

| Flag | Description |
|------|-------------|
| `--rgctl PATH` | rgctl binary |
| `--output-dir PATH` | default: `rgctl-reports/` |
| `--repo-root PATH` | default: parent of `scripts/` |
| `--no-clean` | skip deleting `.rgctl/` before discover |
| `--update-readmes` | sync summary tables into root + project READMEs |
| `--projects rust python …` | run subset only |
| `--blast-top N` | keep top N blast scores per project (default: 10) |
| `--skip-blast-scan` | skip full function blast scan (faster; omits top scores) |

Exit code **0** if every project `discover` succeeds; **1** otherwise.

### Graph correctness

Hand-labeled facts live in `ecommerce-*/correctness/expected-facts.json` (see [`correctness/SCHEMA.md`](../correctness/SCHEMA.md)). They are checked by `cargo test --test graph_correctness` in the rgctl repo (not this report script).

### Install rgctl from GitHub Releases

[`install_rgctl_release.sh`](install_rgctl_release.sh) downloads the platform archive published by [rgctl releases](https://github.com/sshaaf/rgctl/releases):

```bash
./scripts/install_rgctl_release.sh
RGCTL_TAG=v0.1.0 ./scripts/install_rgctl_release.sh
RGCTL=/path/to/.rgctl-bin/rgctl ./scripts/run_rgctl_report.py
```

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `RGCTL_REPO` | `sshaaf/rgctl` | GitHub `owner/repo` |
| `RGCTL_TAG` | _(latest)_ | Release tag, e.g. `v0.1.0` |
| `RGCTL_TARGET` | auto-detect | Rust triple, e.g. `x86_64-unknown-linux-gnu` |
| `RGCTL_INSTALL_DIR` | `.rgctl-bin/` | Extract destination |
| `GITHUB_TOKEN` / `GH_TOKEN` | — | Optional; higher API rate limits / private release assets |

### GitHub Actions

[`.github/workflows/rgctl-report.yml`](../.github/workflows/rgctl-report.yml) runs when:

1. **rgctl publishes a release** — the [rgctl release workflow](https://github.com/sshaaf/rgctl/blob/main/.github/workflows/release.yml) sends a `repository_dispatch` (`rgctl-released`) with the new tag.
2. **Manual run** — Actions → **rgctl Report** → Run workflow.

The workflow:

1. Downloads the matching rgctl binary from GitHub Releases
2. Runs `./scripts/run_rgctl_report.py --no-clean`
3. Packages `rgctl-reports/` as **`rgctl-reports-<tag>-<run_id>.tar.gz`** (+ SHA256 sidecar)
4. Uploads the archive as a **workflow artifact** (90-day retention) — nothing is committed to git

#### Setup (one-time)

In **sshaaf/rgctl** repository secrets:

| Secret | Purpose |
|--------|---------|
| `RGCTL_TESTS_DISPATCH_TOKEN` | PAT (classic `repo` or fine-grained **Actions: write** on `rgctl-tests`) to trigger the report workflow |

Optional in **sshaaf/rgctl-tests**:

| Secret | Purpose |
|--------|---------|
| `RGCTL_DOWNLOAD_TOKEN` | Only if rgctl releases are private |

#### Download a report

Open the workflow run → **Artifacts** → download `rgctl-reports-<tag>-<run_id>.tar.gz`.

```bash
tar -xzf rgctl-reports-v0.1.0-123456789.tar.gz
open rgctl-reports/REPORT.html
```

### Requirements

- `rgctl` built with language bundles (uses `discover . --cfg`)
- [`rgctl-policy.json`](../rgctl-policy.json) at repo root (for `check`)

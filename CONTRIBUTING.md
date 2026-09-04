# Contributing to rgctl

Thanks for helping improve rgctl. This guide covers local setup, tests, and where to put changes.

**Documentation map:** [docs/README.md](docs/README.md)

---

## Prerequisites

- **Rust** stable (via [rustup](https://rustup.rs/))
- **Node.js 18+** and npm (dashboard UI only)
- **git**

Optional: **Playwright** (dashboard browser tests) — installed via `dashboard/` npm scripts.

---

## Clone and build

```bash
git clone https://github.com/sshaaf/rgctl.git
cd rgctl
cargo build --release
./target/release/rgctl --version
```

### Dashboard (when changing `dashboard/`)

```bash
cd dashboard
npm ci
npm run build
cd ..
cargo build --release   # embeds dashboard/dist
```

WASM worker:

```bash
# from repo root — see dashboard/wasm/ or project scripts if present
cargo build -p rgctl-wasm --target wasm32-unknown-unknown --release
```

---

## Running tests

```bash
# Unit / integration (workspace)
cargo test

# Release-mode CLI golden paths (slower)
cargo test --release --test subprocess_golden_path
cargo test --release --test all_commands_sanity

# Dashboard bundle assertions
cargo test dashboard_harness

# Golden repos (optional, long)
./scripts/validate-golden-repos.sh
# Discover timing baselines (manual): cargo test --release --test discover_perf_baselines -- --ignored --nocapture
```

### Dashboard Playwright scripts

Serve a discovered dashboard, then:

```bash
cd dashboard
DASHBOARD_URL=http://127.0.0.1:8765/ node scripts/test-guide-cli.mjs
```

---

## Project layout (short)

| Area | Path |
|------|------|
| CLI entry | `src/cli/` |
| Analysis (CFG, PDG, taint) | `crates/rgctl-analysis/` |
| Graph storage | `crates/rgctl-graph/` |
| Dashboard export | `crates/rgctl-dashboard/` |
| Browser UI | `dashboard/src/` |
| WASM engine | `crates/rgctl-wasm/` |
| Language plugins | `crates/rgctl-lang-*/` |

Full map: [docs/Code_structure.md](docs/Code_structure.md)

---

## Adding or improving a language / feature

Use the hub checklist for path choice, test matrices, and pre-PR commands:

**[docs/contributor-checklist.md](docs/contributor-checklist.md)**

Tier 1 depth (Layers A–F): [docs/tier-1-language-support.md](docs/tier-1-language-support.md) · language list: [docs/languages/README.md](docs/languages/README.md)

---

## Documentation changes

- **User-facing:** `docs/Introduction.md`, `docs/user-guide.md`, `docs/dashboard-user-guide.md`
- **Agents:** `AGENTS.md`, `docs/json-api.md`, `docs/agent-recipes.md`
- **Accuracy:** keep CLI examples aligned with `dashboard/scripts/validate-guide-cli-gbuilder.sh` where possible

---

## Pull requests

1. Branch from `main` (or the active integration branch).
2. Keep commits focused; match existing Rust style and `cargo fmt` / `clippy` expectations.
3. **Sign every commit** with **DCO sign-off** (`git commit -s`). See [Developer Certificate of Origin](https://developercertificate.org/) and [contributor checklist §6](docs/contributor-checklist.md#6-documentation--pr).
4. Run the [standard test workflow](docs/contributor-checklist.md#5-standard-test-workflow) and list the commands you ran in the PR template.

---

## Releases

Maintainers: [docs/releasing.md](docs/releasing.md)

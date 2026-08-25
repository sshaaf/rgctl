# Releasing rgctl

How maintainers publish versioned binaries and GitHub Releases.

---

## Version numbers

- **Crate / CLI version** lives in root [`Cargo.toml`](../Cargo.toml) (`[package].version`).
- **Workspace crates** share the same version in their `Cargo.toml` files and `[workspace.dependencies]` pins.
- **Git tags** use a `v` prefix: `v0.2.0` (not `0.2.0` alone).

Bump all workspace versions together before tagging.

---

## Release workflow (automated)

Pushing a tag matching `v*` triggers [`.github/workflows/release.yml`](../.github/workflows/release.yml):

1. **Build** `rgctl` release binaries for:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
   - `x86_64-pc-windows-msvc`
2. **Package** as `rgctl-<version>-<target>.tar.gz` (or `.zip` on Windows).
3. **Publish** a GitHub Release with notes from `docs/releases/<tag>.md` (if present) plus GitHub-generated PR/commit notes, and `SHA256SUMS.txt`.

Curated notes live under [`docs/releases/`](releases/) (`v0.4.6.md`, …). If that file is missing, the workflow still publishes auto-generated notes.

### Tag and push

```bash
# On main, with a clean tree and versions already bumped
# Write docs/releases/v0.4.6.md first so CI attaches it to the GitHub Release.
git tag -a v0.4.6 -m "Release v0.4.6"
git push origin v0.4.6
```

Track the run: **Actions → Release**.

### Manual re-run

From the Actions tab, run **Release** via **workflow_dispatch** with:

- `tag`: e.g. `v0.2.0`
- `ref`: branch or SHA to build (default `main`)
- `draft`: optional draft release

---

## Pre-release checks (local)

```bash
cargo build --release
cargo test --release

# Dashboard asset build (if UI changed)
cd dashboard && npm ci && npm run build && cd ..

# Optional: golden repo validation
./scripts/validate-golden-repos.sh
```

---

## Assets users download

From [GitHub Releases](https://github.com/sshaaf/rgctl/releases):

| Platform | Asset pattern |
|----------|----------------|
| macOS Apple Silicon | `rgctl-*-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `rgctl-*-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `rgctl-*-x86_64-unknown-linux-gnu.tar.gz` |
| Windows | `rgctl-*-x86_64-pc-windows-msvc.zip` |

Extract and run `rgctl --version`. See [User Guide §1](user-guide.md#1-installation).

---

## After release

- Verify the Release page lists all four platform archives and checksums.
- Smoke-test `discover` + `gql` on a small repo with the downloaded binary.
- If `RGCTL_TESTS_DISPATCH_TOKEN` is configured, CI dispatches `rgctl-released` to the external test repo (see workflow comments).

---

## Naming

| Thing | Name |
|-------|------|
| Project / crates / GitHub repo | **rgctl** / **rgctl** (`sshaaf/rgctl`) |
| CLI binary users run | **`rgctl`** |
| On-disk index directory | **`.rgctl/`** |

Release archives stay `rgctl-${VERSION}-${target}.tar.gz` (project name) and contain the `rgctl` binary.

---

## See also

- [CONTRIBUTING.md](../CONTRIBUTING.md) — development setup
- [User Guide](user-guide.md) — install from release artifacts

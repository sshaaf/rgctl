# Large local example checkouts (not in git)

`/example/` is **gitignored**. Clone stress corpora here for manual profile and integration gates.

| Path | Fetch | Gate |
|------|-------|------|
| `linux/` | Linux kernel tree (maintainer checkout) | `linux_cold_discover_within_baseline` — default discover, wall ≤ **145 s** (+10%) |
| `metasfresh-4.9.8b/` | metasfresh ERP checkout | `metasfresh_cold_discover_within_baseline` — `discover --full`, wall ≤ **74 s** (+10%) |
| `kafka/` | Kafka source tree | `kafka_cold_discover_within_baseline` |
| `k8s-website/` | [kubernetes/website](https://github.com/kubernetes/website) `content/en` | `./scripts/fetch-profile-repos.sh` then `k8s_website_markdown_cold_discover_within_baseline -- --ignored` or `k8s_website_obsidian_export_to_vault -- --ignored` (after discover) |

Manual stage breakdown: [docs/internal/profile.md](../docs/internal/profile.md) (`example/linux` + `example/metasfresh-4.9.8b`, `--no-daemon`, run from inside the corpus dir).

**k8s Obsidian export (manual):**

```bash
./scripts/fetch-profile-repos.sh
export REPO="$(pwd)/example/k8s-website"
cargo build --release --bin rgctl
rgctl -r "$REPO" discover -l markdown
rgctl -r "$REPO" export --export-format obsidian --export-output "$REPO/vault" --query all
# Open example/k8s-website/vault in Obsidian
```

The fetch script now pulls all large profiling fixtures in one go:

- `example/linux`
- `example/kafka`
- `example/metasfresh-4.9.8b`
- `example/coolstore-weblogic`
- `example/kubernetes`
- `example/k8s-website` (sparse `content/en`)

Override paths with `RGCTL_LINUX_REPO`, `RGCTL_KAFKA_REPO`, or `RGCTL_K8S_WEBSITE_REPO`.

**Cold profile:** gates remove `example/<repo>/.rgctl/` before discover and require `target/release/rgctl` (`cargo build --release --bin rgctl`). Do not profile against a warm or partial cache — numbers will be wrong.

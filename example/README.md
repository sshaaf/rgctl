# Large local example checkouts (not in git)

`/example/` is **gitignored**. Clone stress corpora here for manual profile and integration gates.

| Path | Fetch | Gate |
|------|-------|------|
| `linux/` | Linux kernel tree (maintainer checkout) | `linux_cold_discover_within_baseline` — default discover, wall ≤ **145 s** (+10%) |
| `metasfresh-4.9.8b/` | metasfresh ERP checkout | `metasfresh_cold_discover_within_baseline` — `discover --full`, wall ≤ **74 s** (+10%) |
| `kafka/` | Kafka source tree | `kafka_cold_discover_within_baseline` |
| `kubernetes/` | Kubernetes source tree | manual / future gate |
| `magento2/` | [Magento Open Source](https://github.com/magento/magento2) | PHP stress corpus — `discover app lib setup -l php` (exclude `vendor/`, `generated/`) |
| `k8s-website/` | [kubernetes/website](https://github.com/kubernetes/website) `content/en` | `./scripts/fetch-profile-repos.sh` then `k8s_website_markdown_cold_discover_within_baseline -- --ignored` or `k8s_website_obsidian_export_to_vault -- --ignored` (after discover) |

### Language-scale corpora (~10k source files)

Per-language cold discover gates for extraction-depth work. Fetch via `./scripts/fetch-profile-repos.sh`. Filter with `-l <lang>`; exclude `vendor/`, `node_modules/`, `target/`.

| Language | Path | Upstream | Discover | ~Files |
|----------|------|----------|----------|--------|
| C | `linux/` | torvalds/linux | default (Gate A) | ~10k+ `.c`/`.h` |
| C++ | `llvm-project/` | llvm/llvm-project (`clang/`) | `-l cpp` | ~10k+ `.cpp`/`.h` |
| C# | `roslyn/` | dotnet/roslyn (`src/`) | `-l csharp` | ~8k+ `.cs` |
| Go | `kubernetes/` | kubernetes/kubernetes | `-l go` | ~10k+ `.go` |
| Java | `metasfresh-4.9.8b/` | metasfresh/metasfresh | `--full` | ~10k+ `.java` |
| JavaScript | `node/` | nodejs/node (`test/`, sparse) | `-l javascript` on `test/` | ~9k `.js` |
| PHP | `magento2/` | magento/magento2 | `-l php` on `app/` `lib/` `setup/` | ~10k+ `.php` |
| Python | `home-assistant/` | home-assistant/core | `-l python` | ~12k+ `.py` |
| Rust | `rust/` | rust-lang/rust (`library/` `compiler/`) | `-l rust` | ~10k+ `.rs` |
| TypeScript | `vscode/` | microsoft/vscode (`src/`) | `-l typescript` | ~10k+ `.ts` |

OpenSpec reference: `openspec/changes/_shared/starting-context.md`

**JavaScript corpus note:** `nodejs/node` `lib/` is only ~400 `.js` files (the runtime stdlib). The language-scale gate uses sparse-checkout **`test/`** (~9.2k discoverable `.js`/`.mjs`). Set `RGCTL_NODE_REPO` to override the discover root (default `example/node/test`).

Manual stage breakdown: [docs/internal/profile.md](../docs/internal/profile.md) (`example/linux` + `example/metasfresh-4.9.8b`; run from inside the corpus dir).

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
- `example/magento2`
- `example/k8s-website` (sparse `content/en`)

Override paths with `RGCTL_LINUX_REPO`, `RGCTL_KAFKA_REPO`, `RGCTL_K8S_WEBSITE_REPO`, `RGCTL_MAGENTO2_REPO`, `RGCTL_RUST_REPO`, `RGCTL_HOME_ASSISTANT_REPO`, `RGCTL_VSCODE_REPO`, `RGCTL_NODE_REPO`, `RGCTL_ROSLYN_REPO`, `RGCTL_LLVM_REPO`.

**Cold profile:** gates remove `example/<repo>/.rgctl/` before discover and require `target/release/rgctl` (`cargo build --release --bin rgctl`). Do not profile against a warm or partial cache — numbers will be wrong.

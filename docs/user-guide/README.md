# User-guide workflow tests

Runnable commands from [user-guide.md](../user-guide.md) §16 and the VHS tapes
(`docs/videos/user-guide-cli.tape`, `docs/videos/markdown-context-cli.tape`) are
covered by Rust integration tests:

| Test crate | Scope |
|------------|--------|
| `tests/user_guide_scenarios.rs` | Full workflow on all nine `rgctl-tests/ecommerce-*` projects (each copied to an isolated temp repo) |
| `tests/markdown_context_cli.rs` | Markdown fixture GQL queries + VHS tape `jq` pipes |

Each project run copies the fixture into a temp directory so `.rgctl/` artifacts stay isolated (see `tests/support/user_guide_harness.rs`).
Requires `jq` on `PATH`. Uses `CARGO_BIN_EXE_rgctl` when set (CI builds release first).

## Per-project symbols

Java uses the exact VHS tape strings (`CartService::clearCart`, `ShoppingCart`, etc.).
Other languages use mapped equivalents (see `PROJECTS` in `tests/user_guide_harness.rs`).

Steps skipped when not applicable:
- **slice** — only java + rust (verified PDG params)
- **cpg mutations body lines** — C allows 0 hits (type not indexed as `ShoppingCart`)

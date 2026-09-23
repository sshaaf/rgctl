# Unreleased (post v0.4.16)

<!-- Add bullets here during development; move to docs/releases/v0.4.x.md at tag time. -->

## Fixes

- **TypeScript / JavaScript named arrows** — `const` / `let` / object-property / class-field arrows and function expressions are named from their binding (no longer collapsed to a single `anonymous` per file). Truly unnamed callbacks use `anonymous@L{line}`. **Rediscover** TS/JS repos after upgrade so Function identities and call edges refresh.
- **CFG skip diagnostics** — discover `--with-cfg` summarizes skips by `unsupported_language` / `missing_source` / `analysis_error`; `-v` / profile logs each skip (path, symbol, reason).
- **Community flat-graph UX** — warn (soften `[✓] Detected N communities`) when Calls/Uses are sparse or communities ≥ ~90% of nodes.
- **Release checksums** — workflow refuses an empty `SHA256SUMS.txt` and flattens nested artifact dirs.
- **Discover `--exclude`** — repeatable `-e a -e b` while still accepting comma-separated values.

## Docs

- Installation: glibc / Ubuntu 22.04 caveat for gnu Linux releases, Rust **1.88+**, `--no-default-features` when ONNX/`ort` fails to link. Musl/manylinux prebuilt asset tracked in #97.

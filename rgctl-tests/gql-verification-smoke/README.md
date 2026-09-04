# GQL verification smoke tests

Per-language shell scripts that verify extraction-depth GQL probes and core rgctl commands (`blast-radius`, `metrics`, `inspect`, `slice`, `cpg`, `semantic`, `export`, `check`) against each `ecommerce-*` fixture.

See [rgctl-tests README — Extraction-depth GQL](../README.md#extraction-depth-gql--rgctl-command-verification) for the full command matrix and example corpora.

Published language guides (implementation + GQL queries): [docs/languages/](../../docs/languages/README.md).

## Usage

```bash
# from monorepo root
cargo build --release --bin rgctl

# all languages (fixture + example when cloned)
./rgctl-tests/gql-verification-smoke/run-all-extraction-gql.sh

# one language
RGCTL=target/release/rgctl ./rgctl-tests/gql-verification-smoke/verify-extraction-gql-python.sh

# fixture only (~seconds per language)
RGCTL_SKIP_EXAMPLE=1 ./rgctl-tests/gql-verification-smoke/run-all-extraction-gql.sh
```

## Layout

| File | Role |
|------|------|
| `verify-extraction-gql-<lang>.sh` | Per-language GQL + command smoke |
| `run-all-extraction-gql.sh` | Run all languages |
| `extraction-gql-common.sh` | GQL helpers |
| `rgctl-commands-common.sh` | Command-suite runner |
| `rgctl-commands-config.sh` | Per-language symbols and discover flags |

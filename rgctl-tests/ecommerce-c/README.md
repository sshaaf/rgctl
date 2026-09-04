# ecommerce-c

C reference fixture for rgctl Tier 1 language support.

Layered REST-style ecommerce API (SQLite + service/repository pattern) used by
`rgctl discover --with-cfg -l c` dashboard gates (add `--with-dashboard` when exercising the UI).

## Layout

- `include/ecommerce/` — headers (models, repositories, services, handlers)
- `src/` — implementations

## rgctl

See [summary report](../rgctl-reports/REPORT.md) · [language report](../rgctl-reports/languages/c.md) · [HTML](../rgctl-reports/languages/c.html) (2026-07-22).

```bash
rgctl -f json discover . --with-cfg -e build,cmake-build-debug,.rgctl
rgctl -f json blast-radius 'src/coolstore/services/shopping_cart_service.c::price_shopping_cart'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgctl-policy.json
```

### Extraction-depth GQL

`verify-extraction-gql.sh` — `CALLS`, include `Import` graph, `file::symbol` FQN. Example: [`example/linux`](../../example/linux).

```bash
RGCTL=../../target/release/rgctl ../gql-verification-smoke/verify-extraction-gql-c.sh
```

Details: [rgctl-tests README](../README.md#extraction-depth-gql-verification).

| Metric | Value |
|--------|------:|
| Files indexed | 84 |
| Nodes | 486 |
| Edges | 838 |
| Discover ms | 309 |
| Cache MB | 0.92 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | ✓ |
| metrics | ✓ |
| export | ✗ |
| check | ✓ |
| slice / taint | — / ✓ |

### Top symbols

| Symbol | Score | Callers | Impact |
|--------|------:|--------:|-------:|
| `seed` | 25.15 | 1 | 3 |
| `correctness_leaf` | 25.10 | 1 | 2 |
| `init_shopping_cart_for_pricing` | 25.10 | 1 | 2 |
| `round2` | 25.05 | 1 | 1 |
| `is_post` | 25.05 | 1 | 1 |

C fixture with CoolStore /services cart pricing mutations.

Raw: [`../rgctl-reports/c-summary.json`](../rgctl-reports/c-summary.json)

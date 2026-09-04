# ecommerce-javascript

E-commerce reference app.

## rgctl

See [summary report](../rgctl-reports/REPORT.md) · [language report](../rgctl-reports/languages/javascript.md) · [HTML](../rgctl-reports/languages/javascript.html) (2026-07-22).

```bash
rgctl -f json discover . --cfg -e node_modules
rgctl -f json blast-radius 'src/services/orderService.js::checkout'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgctl-policy.json
```

### Extraction-depth GQL

`verify-extraction-gql.sh` — `Import`, `EXTENDS`, `INSTANTIATES`, `CALLS`, class method FQN. Example: [`example/node/test`](../../example/node/test).

```bash
RGCTL=../../target/release/rgctl ../gql-verification-smoke/verify-extraction-gql-javascript.sh
```

Details: [rgctl-tests README](../README.md#extraction-depth-gql-verification).

| Metric | Value |
|--------|------:|
| Files indexed | 60 |
| Nodes | 1440 |
| Edges | 2843 |
| Discover ms | 303 |
| Cache MB | 1.74 |

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
| `getDb` | 40.80 | 16 | 16 |
| `asyncHandler` | 40.45 | 9 | 9 |
| `nowIso` | 40.35 | 6 | 7 |
| `createShoppingCartItem` | 40.10 | 2 | 2 |
| `correctnessLeaf` | 25.10 | 1 | 2 |

Mirror of TypeScript graph without types.

Raw: [`../rgctl-reports/javascript-summary.json`](../rgctl-reports/javascript-summary.json)

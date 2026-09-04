# ecommerce-typescript

E-commerce reference app.

## rgctl

See [summary report](../rgctl-reports/REPORT.md) · [language report](../rgctl-reports/languages/typescript.md) · [HTML](../rgctl-reports/languages/typescript.html) (2026-07-22).

```bash
rgctl -f json discover . --cfg -e node_modules,dist
rgctl -f json blast-radius 'src/services/orderService.ts::checkout'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgctl-policy.json
```

### Extraction-depth GQL

`verify-extraction-gql.sh` — `Import`, `EXTENDS`, `IMPLEMENTS`, `ANNOTATEDWITH`, `CALLS`. Example: [`example/vscode/src`](../../example/vscode/src).

```bash
RGCTL=../../target/release/rgctl ../gql-verification-smoke/verify-extraction-gql-typescript.sh
```

Details: [rgctl-tests README](../README.md#extraction-depth-gql-verification).

| Metric | Value |
|--------|------:|
| Files indexed | 61 |
| Nodes | 1607 |
| Edges | 3169 |
| Discover ms | 305 |
| Cache MB | 1.88 |

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
| `migrate` | 25.10 | 1 | 2 |

High AST node count; compare with JavaScript sibling.

Raw: [`../rgctl-reports/typescript-summary.json`](../rgctl-reports/typescript-summary.json)

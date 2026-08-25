# ecommerce-go

E-commerce reference app.

## rgBuilder

See [summary report](../rgbuilder-reports/REPORT.md) · [language report](../rgbuilder-reports/languages/go.md) · [HTML](../rgbuilder-reports/languages/go.html) (2026-07-22).

```bash
rgctl -f json discover . --cfg -e vendor
rgctl -f json blast-radius 'internal/service/order.go::Checkout'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 46 |
| Nodes | 495 |
| Edges | 1099 |
| Discover ms | 308 |
| Cache MB | 1.16 |

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
| `handleError` | 40.80 | 16 | 16 |
| `MapRepoError` | 40.40 | 8 | 8 |
| `toProductResponse` | 40.15 | 3 | 3 |
| `toCategoryResponse` | 40.15 | 3 | 3 |
| `NewUnauthorized` | 40.10 | 2 | 2 |

Partial Go indexing possible; verify file coverage in discover metrics.

Raw: [`../rgbuilder-reports/go-summary.json`](../rgbuilder-reports/go-summary.json)

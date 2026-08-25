# ecommerce-cpp

C++ reference fixture for rgBuilder Tier 1 language support.

Layered ecommerce API (SQLite + service/repository pattern) mirroring ecommerce-c.

## rgBuilder

See [summary report](../rgbuilder-reports/REPORT.md) · [language report](../rgbuilder-reports/languages/cpp.md) · [HTML](../rgbuilder-reports/languages/cpp.html) (2026-07-22).

```bash
rgctl -f json discover . --cfg -e build,cmake-build-debug,.rgbuilder
rgctl -f json blast-radius 'src/coolstore/services/shopping_cart_service.cpp::priceShoppingCart'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 81 |
| Nodes | 638 |
| Edges | 1224 |
| Discover ms | 308 |
| Cache MB | 0.96 |

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
| `correctnessLeaf` | 25.10 | 1 | 2 |
| `initShoppingCartForPricing` | 25.10 | 1 | 2 |
| `correctnessMid` | 25.05 | 1 | 1 |
| `cart_delete` | 25.05 | 1 | 1 |
| `getShoppingCart` | 25.05 | 1 | 1 |

C++ fixture with CoolStore /services cart pricing mutations.

Raw: [`../rgbuilder-reports/cpp-summary.json`](../rgbuilder-reports/cpp-summary.json)

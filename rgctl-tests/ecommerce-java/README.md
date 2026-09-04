# ecommerce-java

E-commerce reference app.

## rgctl

See [summary report](../rgctl-reports/REPORT.md) · [language report](../rgctl-reports/languages/java.md) · [HTML](../rgctl-reports/languages/java.html) (2026-07-22).

```bash
rgctl -f json discover . --cfg -e target,data
rgctl -f json blast-radius 'src/main/java/com/example/ecommerce/service/OrderService.java::checkout'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgctl-policy.json
```

### Extraction-depth GQL

`verify-extraction-gql.sh` — JF-01…JF-07 on [`tests/fixtures/java/langfeatures`](../../tests/fixtures/java/langfeatures); example smoke on [`example/metasfresh-4.9.8b`](../../example/metasfresh-4.9.8b).

```bash
RGCTL=../../target/release/rgctl ../gql-verification-smoke/verify-extraction-gql-java.sh
```

Details: [rgctl-tests README](../README.md#extraction-depth-gql-verification).

| Metric | Value |
|--------|------:|
| Files indexed | 66 |
| Nodes | 993 |
| Edges | 2211 |
| Discover ms | 307 |
| Cache MB | 1.89 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | ✓ |
| metrics | ✓ |
| export | ✗ |
| check | ✓ |
| slice / taint | ◐ / ✓ |

### Top symbols

| Symbol | Score | Callers | Impact |
|--------|------:|--------:|-------:|
| `findByEmail` | 40.85 | 3 | 17 |
| `currentUser` | 40.70 | 7 | 14 |
| `getProductByItemId` | 40.45 | 2 | 9 |
| `getRole` | 40.35 | 6 | 7 |
| `getUserCart` | 40.30 | 5 | 6 |

Strongest CALLS graph and community modularity in this suite.

Raw: [`../rgctl-reports/java-summary.json`](../rgctl-reports/java-summary.json)

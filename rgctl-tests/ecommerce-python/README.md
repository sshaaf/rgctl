# ecommerce-python

E-commerce reference app.

## rgctl

See [summary report](../rgctl-reports/REPORT.md) · [language report](../rgctl-reports/languages/python.md) · [HTML](../rgctl-reports/languages/python.html) (2026-07-22).

```bash
rgctl -f json discover . --cfg -e .venv,__pycache__
rgctl -f json blast-radius 'app/services/order.py::checkout'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgctl-policy.json
```

### Extraction-depth GQL

Verifies archived openspec Python extraction features (`Import`, `EXTENDS`, `ANNOTATEDWITH`, `INSTANTIATES`, `CALLS`, method FQN) **and** runs `blast-radius`, `metrics`, `inspect`, `slice`, `cpg`, `semantic`, `export`, `check`. Example scale corpus: [`example/home-assistant`](../../example/home-assistant).

```bash
RGCTL=../../target/release/rgctl ../gql-verification-smoke/verify-extraction-gql-python.sh
RGCTL_SKIP_EXAMPLE=1 ../gql-verification-smoke/verify-extraction-gql-python.sh   # fixture only
```

See [rgctl-tests README — Extraction-depth GQL](../README.md#extraction-depth-gql-verification).

| Metric | Value |
|--------|------:|
| Files indexed | 59 |
| Nodes | 571 |
| Edges | 1407 |
| Discover ms | 307 |
| Cache MB | 1.17 |

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
| `get_product_by_item_id` | 40.45 | 2 | 9 |
| `add` | 40.45 | 8 | 9 |
| `get_shopping_cart` | 40.25 | 4 | 5 |
| `price_shopping_cart` | 40.20 | 3 | 4 |
| `_cart_out` | 40.20 | 4 | 4 |

Second full CFG/PDG language; rich class nodes.

Raw: [`../rgctl-reports/python-summary.json`](../rgctl-reports/python-summary.json)

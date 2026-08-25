# ecommerce-rust

Axum + SQLx e-commerce API for rgBuilder test indexing.

## Run

```bash
cargo run
# http://localhost:8080/health
```

## Environment

| Variable | Default |
|----------|---------|
| `DATABASE_URL` | `sqlite:ecommerce.db` |
| `JWT_SECRET` | `dev-secret-change-me` |
| `BIND_ADDR` | `0.0.0.0:8080` |

## Demo data

Seeds **Electronics** category with **Wireless Headphones** and **USB-C Hub** on first run.

## rgBuilder

See [summary report](../rgbuilder-reports/REPORT.md) · [language report](../rgbuilder-reports/languages/rust.md) · [HTML](../rgbuilder-reports/languages/rust.html) (2026-07-22).

```bash
rgctl -f json discover . --cfg -e target
rgctl -f json blast-radius 'src/services/order.rs::checkout'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 68 |
| Nodes | 513 |
| Edges | 1111 |
| Discover ms | 307 |
| Cache MB | 1.21 |

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
| `now_iso` | 40.35 | 6 | 7 |
| `correctness_shared` | 40.15 | 2 | 3 |
| `find_by_email` | 40.10 | 2 | 2 |
| `items_for_order` | 40.10 | 2 | 2 |
| `to_response` | 40.10 | 2 | 2 |

Best deep-analysis reference (CFG/PDG/inspect/taint).

Raw: [`../rgbuilder-reports/rust-summary.json`](../rgbuilder-reports/rust-summary.json)

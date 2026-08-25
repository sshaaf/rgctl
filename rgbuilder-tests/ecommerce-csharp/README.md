# ecommerce-csharp

E-commerce reference app (ASP.NET Core 8 + EF Core SQLite + JWT).

## Run

```bash
cd src/Ecommerce
dotnet run
# or from repo root:
dotnet run --project src/Ecommerce
```

Listens on `http://localhost:5000` (or ports in `launchSettings.json`).

## rgBuilder

See [summary report](../rgbuilder-reports/REPORT.md) · [language report](../rgbuilder-reports/languages/csharp.md) · [HTML](../rgbuilder-reports/languages/csharp.html) (2026-07-22).

```bash
rgctl -f json discover . --cfg -e bin,obj,data
rgctl -f json blast-radius 'src/Ecommerce/Services/OrderService.cs::CheckoutAsync'
rgctl -f json metrics --communities --pagerank
rgctl -f json check --policy-file ../rgbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 62 |
| Nodes | 624 |
| Edges | 1321 |
| Discover ms | 309 |
| Cache MB | 1.31 |

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
| `GetUserCartAsync` | 40.25 | 5 | 5 |
| `GetProductAsync` | 25.10 | 1 | 2 |
| `InitShoppingCartForPricing` | 25.10 | 1 | 2 |
| `GetByProductIdAsync` | 25.10 | 1 | 2 |
| `CorrectnessLeaf` | 25.10 | 1 | 2 |

ASP.NET Core mirror of Java; Tier 1 CFG/taint/calls.

Raw: [`../rgbuilder-reports/csharp-summary.json`](../rgbuilder-reports/csharp-summary.json)

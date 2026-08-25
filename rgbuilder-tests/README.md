# rgBuilder Test Applications

In-tree Tier‑1 fixtures and graph **correctness** suite for [rgBuilder](https://github.com/sshaaf/rgBuilder) (copied into this monorepo; not a git submodule).

Reference **e-commerce store** applications for indexing, blast-radius, and migration tests.

Each Tier 1 language project implements the same domain:

| Feature | Description |
|---------|-------------|
| **Users** | Register, login, JWT/session auth |
| **Categories** | Product taxonomy |
| **Products** | Catalog with stock and pricing |
| **Cart** | Per-user shopping cart |
| **Orders** | Checkout and order history |
| **Reviews** | Product ratings and comments |
| **Inventory** | Stock adjustments on checkout |

## Projects

| Directory | Stack | Database | Run |
|-----------|-------|----------|-----|
| [`ecommerce-rust/`](ecommerce-rust/) | Axum + SQLx | SQLite | `cargo run` |
| [`ecommerce-python/`](ecommerce-python/) | FastAPI + SQLAlchemy | SQLite | `uvicorn app.main:app --reload` |
| [`ecommerce-go/`](ecommerce-go/) | Gin + GORM | SQLite | `go run ./cmd/server` |
| [`ecommerce-java/`](ecommerce-java/) | Spring Boot + JPA | H2 (file) | `./mvnw spring-boot:run` |
| [`ecommerce-csharp/`](ecommerce-csharp/) | ASP.NET Core + EF Core | SQLite | `dotnet run --project src/Ecommerce` |
| [`ecommerce-c/`](ecommerce-c/) | C + SQLite (layered services/repos) | SQLite | `make` (optional) |
| [`ecommerce-cpp/`](ecommerce-cpp/) | C++ + SQLite (classes/namespaces) | SQLite | `cmake --build build` (optional) |
| [`ecommerce-typescript/`](ecommerce-typescript/) | Express + better-sqlite3 | SQLite | `npm run build && npm start` |
| [`ecommerce-javascript/`](ecommerce-javascript/) | Express + better-sqlite3 | SQLite | `npm start` |

## Shared REST API (conceptual)

### Existing fixture API (`/api/*`)

```
GET    /health
POST   /api/auth/register
POST   /api/auth/login
GET    /api/categories
POST   /api/categories
GET    /api/products
GET    /api/products/:id
POST   /api/products
GET    /api/cart
POST   /api/cart/items
DELETE /api/cart/items/:productId
POST   /api/orders
GET    /api/orders
GET    /api/orders/:id
GET    /api/products/:id/reviews
POST   /api/products/:id/reviews
```

### CoolStore dual API (`/services/*`)

Same shape as [example/coolstore-weblogic](../example/coolstore-weblogic) (in-memory cart pricing; additive):

```
GET    /services/products
GET    /services/products/{itemId}
GET    /services/cart/{cartId}
POST   /services/cart/{cartId}/{itemId}/{quantity}
DELETE /services/cart/{cartId}/{itemId}/{quantity}
POST   /services/cart/checkout/{cartId}
GET    /services/orders
GET    /services/orders/{orderId}
```

`ShoppingCartService.priceShoppingCart` mutates cart totals (promo/shipping) — useful for hybrid CPG `cpg mutations --type ShoppingCart`.

Use these repos with `rgctl discover .` to compare graph structure across languages.

## Graph data correctness (expected-facts)

Beyond smoke reports, each `ecommerce-*` app ships hand-labeled facts under
`ecommerce-*/correctness/expected-facts.json` (schema: [`correctness/SCHEMA.md`](correctness/SCHEMA.md)).

Checked by the standard Rust test suite (from the rgBuilder repo root):

```bash
cargo test --test graph_correctness
cargo test --test graph_correctness java   # filter by project id
```

Required failures fail the test. Domain edges that extractors still miss (e.g. some checkout→clearCart paths) are `best_effort` warnings. See [rgBuilder#26](https://github.com/sshaaf/rgBuilder/issues/26).

Regenerate analysis reports:

```bash
./scripts/run_rgbuilder_report.sh
# or: RGBUILDER=/path/to/rgctl ./scripts/run_rgbuilder_report.py --update-readmes
```

See [`scripts/README.md`](scripts/README.md) for options.

## rgBuilder analysis results

Summary: **[rgbuilder-reports/REPORT.md](rgbuilder-reports/REPORT.md)** · [HTML](rgbuilder-reports/REPORT.html) (run 2026-07-22)

**Language reports:** [Rust](rgbuilder-reports/languages/rust.md) · [Python](rgbuilder-reports/languages/python.md) · [Go](rgbuilder-reports/languages/go.md) · [Java](rgbuilder-reports/languages/java.md) · [C#](rgbuilder-reports/languages/csharp.md) · [TypeScript](rgbuilder-reports/languages/typescript.md) · [JavaScript](rgbuilder-reports/languages/javascript.md) · [C](rgbuilder-reports/languages/c.md) · [C++](rgbuilder-reports/languages/cpp.md)

### Feature coverage (✓ ok · ◐ partial · — unsupported/n/a)

| Feature | Rust | Py | Go | Java | TS | JS |
|---------|:----:|:--:|:--:|:----:|:--:|:--:|
| discover (`--cfg`) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Dashboard | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| GQL queries | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Metrics (communities + PageRank) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Blast radius | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Export (JSON subgraph) | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| CI check (`--policy-file`) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Program slice | ◐ | ◐ | — | ◐ | — | — | — | — | — |
| Taint analysis | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Inspect CFG | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Inspect PDG | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Inspect dominators | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Serve daemon | — | — | — | — | — | — | — | — | — |

### Index size

| Project | Files | Nodes | Edges | Discover ms |
|---------|------:|------:|------:|------------:|
| Rust | 68 | 513 | 1111 | 307 |
| Python | 59 | 571 | 1407 | 307 |
| Go | 46 | 495 | 1099 | 308 |
| Java | 66 | 993 | 2211 | 307 |
| C# | 62 | 624 | 1321 | 309 |
| TypeScript | 61 | 1607 | 3169 | 305 |
| JavaScript | 60 | 1440 | 2843 | 303 |
| C | 84 | 486 | 838 | 309 |
| C++ | 81 | 638 | 1224 | 308 |

### Blast radius (max score per project)

Full function scan (`--blast-top N`); checkout leaf symbols often score 0.

| Project | Scanned | Score > 0 | Max score | Top symbol |
|---------|--------:|----------:|----------:|------------|
| Rust | 68 | 21 | 40.35 | `now_iso` |
| Python | 80 | 43 | 40.45 | `get_product_by_item_id` |
| Go | 96 | 13 | 40.80 | `handleError` |
| Java | 182 | 59 | 40.85 | `findByEmail` |
| C# | 105 | 14 | 40.25 | `GetUserCartAsync` |
| TypeScript | 88 | 19 | 40.80 | `getDb` |
| JavaScript | 90 | 19 | 40.80 | `getDb` |
| C | 165 | 10 | 25.15 | `seed` |
| C++ | 110 | 11 | 25.10 | `correctnessLeaf` |

Per-project details: [`rgbuilder-reports/languages/`](rgbuilder-reports/languages/) · each `ecommerce-*/README.md` § **rgBuilder**.

Regenerate: `./scripts/run_rgbuilder_report.py`

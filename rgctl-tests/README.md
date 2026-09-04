# rgctl Test Applications

In-tree Tier‑1 fixtures and graph **correctness** suite for [rgctl](https://github.com/sshaaf/rgctl) (copied into this monorepo; not a git submodule).

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

Checked by the standard Rust test suite (from the rgctl repo root):

```bash
cargo test --test graph_correctness
cargo test --test graph_correctness java   # filter by project id
```

Required failures fail the test. Domain edges that extractors still miss (e.g. some checkout→clearCart paths) are `best_effort` warnings. See [rgctl#26](https://github.com/sshaaf/rgctl/issues/26).

Regenerate analysis reports:

```bash
./scripts/run_rgctl_report.sh
# or: RGCTL=/path/to/rgctl ./scripts/run_rgctl_report.py --update-readmes
```

See [`scripts/README.md`](scripts/README.md) for options.

## Extraction-depth GQL + rgctl command verification

Shell scripts exercise **every archived `*-extraction-depth` capability** via GQL and run the **core rgctl analysis commands** on each fixture (with copy-paste examples printed during the run).

Each `gql-verification-smoke/verify-extraction-gql-<lang>.sh` runs:

1. **Fixture GQL** — extraction-depth probes (`Import`, `EXTENDS`, `CALLS`, …) on the in-tree `ecommerce-*` corpus (Java: `tests/fixtures/java/langfeatures` for GQL, `ecommerce-java` for commands).
2. **Fixture commands** — `blast-radius`, `metrics`, `communities list`, `inspect` (cfg/pdg/dom), `slice` (+ `--taint` when supported), `cpg status`/`mutations`, `semantic index`/`query`, `export`, `check`.
3. **Example smoke** — scale GQL on `example/` when cloned (`RGCTL_SKIP_EXAMPLE=1` skips).

```bash
# from monorepo root
cargo build --release --bin rgctl

# all languages (fixture + example when present)
./rgctl-tests/gql-verification-smoke/run-all-extraction-gql.sh

# one language (prints example commands as it runs)
RGCTL=target/release/rgctl ./rgctl-tests/gql-verification-smoke/verify-extraction-gql-python.sh

# fixture only (~seconds per language)
RGCTL_SKIP_EXAMPLE=1 ./rgctl-tests/gql-verification-smoke/run-all-extraction-gql.sh
```

| Command | What the script checks |
|---------|------------------------|
| `blast-radius` | Primary checkout/cart symbol + CoolStore pricing symbol |
| `metrics` | `--communities --pagerank` |
| `communities` | `list` |
| `inspect` | `cfg`, `pdg`, `dom` on checkout function |
| `slice` | Backward slice + `--taint` (skipped when unsupported) |
| `cpg` | `status`; `mutations --type ShoppingCart` (or `OrderDTO` for PHP) |
| `semantic` | `index --embedder vocab` then `query` |
| `export` | `--export-format json --query name:…` |
| `check` | `--policy-file rgctl-tests/rgctl-policy.json` |

Per-language symbols and paths: [`gql-verification-smoke/rgctl-commands-config.sh`](gql-verification-smoke/rgctl-commands-config.sh). Shared runners: [`gql-verification-smoke/extraction-gql-common.sh`](gql-verification-smoke/extraction-gql-common.sh), [`gql-verification-smoke/rgctl-commands-common.sh`](gql-verification-smoke/rgctl-commands-common.sh). See [`gql-verification-smoke/README.md`](gql-verification-smoke/README.md).

Fetch example corpora: `./scripts/fetch-profile-repos.sh` (see [`example/README.md`](../example/README.md)).

| Language | Script | Fixture probes | Example corpus | Discover | Openspec capabilities |
|----------|--------|----------------|----------------|----------|------------------------|
| **C** | [`gql-verification-smoke/verify-extraction-gql-c.sh`](gql-verification-smoke/verify-extraction-gql-c.sh) | CALLS, Import, `file::symbol` FQN | `example/linux` | default | `c-include-graph`, `c-call-resolution`, `c-qualified-symbols` |
| **C++** | [`gql-verification-smoke/verify-extraction-gql-cpp.sh`](gql-verification-smoke/verify-extraction-gql-cpp.sh) | EXTENDS, INSTANTIATES, CALLS | `example/llvm-project/clang` | `-l cpp` | `cpp-inheritance-edges`, `cpp-instantiation`, `cpp-call-resolution` |
| **C#** | [`gql-verification-smoke/verify-extraction-gql-csharp.sh`](gql-verification-smoke/verify-extraction-gql-csharp.sh) | ANNOTATEDWITH, INSTANTIATES, CALLS, namespace FQN | `example/roslyn/src` | `-l csharp` | `csharp-annotations`, `csharp-instantiation`, `csharp-call-binding`, `csharp-namespace-fqn` |
| **Go** | [`gql-verification-smoke/verify-extraction-gql-go.sh`](gql-verification-smoke/verify-extraction-gql-go.sh) | LF-05…LF-17 (`IMPLEMENTS`, `EXTENDS`, Import, generics) | `example/kubernetes` | `-l go -e vendor` | `docs/design/go-language-coverage.md` |
| **Java** | [`gql-verification-smoke/verify-extraction-gql-java.sh`](gql-verification-smoke/verify-extraction-gql-java.sh) | JF-01…JF-07 (INSTANTIATES, ANNOTATED_WITH, REFERENCES, JPMS, lambda, FQN) | `example/metasfresh-4.9.8b` | `--full` | Java extraction-depth / issue #49 |
| **JavaScript** | [`gql-verification-smoke/verify-extraction-gql-javascript.sh`](gql-verification-smoke/verify-extraction-gql-javascript.sh) | Import, EXTENDS, INSTANTIATES, CALLS, method FQN | `example/node/test` | `-l javascript` | `javascript-module-graph`, `javascript-heritage`, `javascript-call-resolution` |
| **PHP** | [`gql-verification-smoke/verify-extraction-gql-php.sh`](gql-verification-smoke/verify-extraction-gql-php.sh) | Import, CALLS, cross-file static call; USES/ANNOTATEDWITH/INSTANTIATES (soft) | `example/magento2` (`app lib setup`) | `-l php` | `php-trait-and-imports`, `php-framework-symbols`, `php-analysis-polish` |
| **Python** | [`gql-verification-smoke/verify-extraction-gql-python.sh`](gql-verification-smoke/verify-extraction-gql-python.sh) | Import, EXTENDS, ANNOTATEDWITH, INSTANTIATES, CALLS, method FQN | `example/home-assistant` | `-l python` | `python-module-graph`, `python-heritage`, `python-decorators`, `python-call-resolution` |
| **Rust** | [`gql-verification-smoke/verify-extraction-gql-rust.sh`](gql-verification-smoke/verify-extraction-gql-rust.sh) | Import, IMPLEMENTS, ANNOTATEDWITH, INSTANTIATES, CALLS | `example/rust` | `-l rust` | `rust-module-graph`, `rust-trait-heritage`, `rust-attributes`, `rust-call-resolution` |
| **TypeScript** | [`gql-verification-smoke/verify-extraction-gql-typescript.sh`](gql-verification-smoke/verify-extraction-gql-typescript.sh) | Import, EXTENDS, IMPLEMENTS, ANNOTATEDWITH, CALLS | `example/vscode/src` | `-l typescript` | `typescript-module-graph`, `typescript-heritage`, `typescript-decorators`, `typescript-call-resolution` |

Shared helpers: [`gql-verification-smoke/extraction-gql-common.sh`](gql-verification-smoke/extraction-gql-common.sh).

Environment:

| Variable | Purpose |
|----------|---------|
| `RGCTL` | Path to `rgctl` binary (default: `target/release/rgctl`) |
| `RGCTL_SKIP_EXAMPLE=1` | Skip example-corpus smoke phase |
| `RGCTL_REPO` | Override discover root |

## rgctl analysis results

Summary: **[rgctl-reports/REPORT.md](rgctl-reports/REPORT.md)** · [HTML](rgctl-reports/REPORT.html) (run 2026-07-22)

**Language reports:** [Rust](rgctl-reports/languages/rust.md) · [Python](rgctl-reports/languages/python.md) · [Go](rgctl-reports/languages/go.md) · [Java](rgctl-reports/languages/java.md) · [C#](rgctl-reports/languages/csharp.md) · [TypeScript](rgctl-reports/languages/typescript.md) · [JavaScript](rgctl-reports/languages/javascript.md) · [C](rgctl-reports/languages/c.md) · [C++](rgctl-reports/languages/cpp.md)

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

Per-project details: [`rgctl-reports/languages/`](rgctl-reports/languages/) · each `ecommerce-*/README.md` § **rgctl**.

Regenerate: `./scripts/run_rgctl_report.py`

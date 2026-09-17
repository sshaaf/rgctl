# ecommerce-ruby

Minimal Ruby MVC fixture for Tier 1 gates: `require` / `Import`, mixin `EXTENDS`/`USES`, `CALLS`, `INSTANTIATES`, CFG/taint/field-write on `OrderDTO`.

## Discover

```bash
cargo build --release --bin rgctl
cd rgctl-tests/ecommerce-ruby
../../target/release/rgctl discover . -l ruby -v
../../target/release/rgctl discover . -l ruby --with-cfg --with-security --with-taint
```

## Verification

| Check | Command / test |
|-------|------------------|
| GQL smoke | `RGCTL=../../target/release/rgctl ../gql-verification-smoke/verify-extraction-gql-ruby.sh` |
| Langfeatures | `cargo test --test ruby_langfeatures` |
| CFG discover | `cargo test --test ruby_cfg_analysis` |
| Dashboard bundle | `cargo test --test dashboard_ecommerce_ruby` (needs embedded dashboard dist) |

Language guide: [docs/languages/ruby.md](../../docs/languages/ruby.md) · honesty limits: [docs/ruby-extract-honesty.md](../../docs/ruby-extract-honesty.md).

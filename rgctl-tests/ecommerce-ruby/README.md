# ecommerce-ruby

Minimal Ruby MVC fixture for rgctl Tier 1 gates (calls, imports, mixins, taint, field writes).

```bash
cargo build --release -p rgctl
cd rgctl-tests/ecommerce-ruby && ../../target/release/rgctl discover . -l ruby -v
```

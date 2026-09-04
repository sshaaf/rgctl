# ecommerce-php

PHP reference fixture for rgctl Tier 1 language support.

Small MVC-style app: controller → service → repository, with auth flow, one taint path, and field-write mutation coverage.

```bash
rgctl discover . --with-cfg --with-taint -l php
```

### Extraction-depth GQL

`verify-extraction-gql.sh` — `Import`, `CALLS`, cross-file static calls; soft probes for `USES`, `ANNOTATEDWITH`, `INSTANTIATES`. Example: [`example/magento2`](../../example/magento2) (`app lib setup`).

```bash
RGCTL=../../target/release/rgctl ../gql-verification-smoke/verify-extraction-gql-php.sh
```

Details: [rgctl-tests README](../README.md#extraction-depth-gql-verification).

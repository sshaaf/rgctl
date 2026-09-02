# ecommerce-php

PHP reference fixture for rgctl Tier 1 language support.

Small MVC-style app: controller → service → repository, with auth flow, one taint path, and field-write mutation coverage.

```bash
rgctl discover . --with-cfg --with-taint -l php
```

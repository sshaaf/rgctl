---
qualified_name: "/Users/sraghuna/local_dev/petprojects/rBuilder/tests/fixtures/markdown-context/./README.md#try-it"
level: "2"
---

From the rgBuilder repo root (build the CLI first: `cargo build --bin rgctl`):

```bash
export REPO="$(pwd)/tests/fixtures/markdown-context"
RGB="$(pwd)/target/debug/rgctl"

# Docs only (Phase 2a)
"$RGB" -r "$REPO" discover . -l markdown

# Docs + code (Phase 2b)
"$RGB" -r "$REPO" discover . -l markdown,java

# Example: find checkout-related headings
"$RGB" -r "$REPO" -f json gql \
  "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n LIMIT 10"

# Example: guide links to the payments ADR section
"$RGB" -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:REFERENCES]->(t:Module) WHERE h.kind = 'heading' AND t.name = 'Payments' RETURN h, t"

# Example: doc → Java class (needs markdown,java discover)
"$RGB" -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' RETURN h, f, c"
```

Artifacts appear under `$REPO/.rgbuilder/` after discover.

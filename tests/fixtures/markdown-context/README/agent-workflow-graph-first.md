---
qualified_name: "/Users/sraghuna/local_dev/petprojects/rBuilder/tests/fixtures/markdown-context/./README.md#agent-workflow-graph-first"
level: "2"
---

Before editing checkout code in this example, query the graph:

```bash
rgctl -r "$REPO" -f json gql \
  "MATCH (h:Module)-[:CONTAINS*1..3]->(n) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' RETURN h, n LIMIT 20"
```

Then read [Checkout Flow](docs/guide.md#checkout-flow) and [Payments ADR](docs/adr.md#payments) only if you need prose detail.
[[/Users/sraghuna/local_dev/petprojects/rBuilder/tests/fixtures/markdown-context/./docs/guide.md#checkout-flow]]
[[/Users/sraghuna/local_dev/petprojects/rBuilder/tests/fixtures/markdown-context/./docs/adr.md#payments]]

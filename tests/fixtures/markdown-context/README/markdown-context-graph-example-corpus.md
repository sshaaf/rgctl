---
qualified_name: "/Users/sraghuna/local_dev/petprojects/rBuilder/tests/fixtures/markdown-context/./README.md#markdown-context-graph-example-corpus"
level: "1"
---

This directory is a **minimal but realistic** repo used to demo and test rgctl’s markdown context graph ([issue #56](https://github.com/sshaaf/rgctl/issues/56)).

It is not production software. It models how **docs, ADRs, and code** land in the same `graph.snapshot.bin` so agents can query structure instead of reading every file.

**Note:** This is isolated from the parent rgctl repo. When you set `REPO` to this folder, the root [AGENTS.md](https://github.com/sshaaf/rgctl/blob/main/AGENTS.md) of rgctl is **not** indexed — only files under this tree.

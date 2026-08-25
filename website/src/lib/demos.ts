export type DemoScenario = {
  id: string;
  flow: string;
  title: string;
  prompt: string;
  commands: string[];
  note?: string;
  output: string;
  reasoning: string;
};

export const demoFlows = [
  "Modernization audit",
  "Intent discovery",
  "Pre-refactor safety",
  "Policy verification",
] as const;

export const demos: DemoScenario[] = [
  {
    id: "migration",
    flow: "Modernization audit",
    title: "Migration plan",
    prompt: "Generate a complete migration plan to help us modernize this repository.",
    commands: [
      "discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints",
    ],
    note: "Plan is written to .rgctl/migration_plan.json (opt-in flags). Dashboard needs --with-dashboard.",
    output: `{
  "schema_version": 2,
  "preset": "hybrid_default",
  "order": "scheduled",
  "packages": [
    {"name": "com.example.ecommerce.payment", "priority": 0.91, "step": 1},
    {"name": "com.example.ecommerce.cart", "priority": 0.72, "step": 2}
  ]
}`,
    reasoning:
      "rgctl wrote a migration roadmap. Open highest-priority packages first and run blast-radius before editing shared symbols.",
  },
  {
    id: "pagerank",
    flow: "Modernization audit",
    title: "Architectural hotspots",
    prompt: "Which core functions are major system bottlenecks or central dependencies?",
    commands: ["-f json metrics --pagerank"],
    output: `{
  "schema_version": 1,
  "pagerank": {
    "top": [
      {"node": "execute", "pagerank": 0.142},
      {"node": "getConnection", "pagerank": 0.098}
    ],
    "converged": true,
    "iterations": 20
  }
}`,
    reasoning:
      "High PageRank nodes are risky to change directly — check blast-radius and wrap or adapt callers first.",
  },
  {
    id: "inventory",
    flow: "Modernization audit",
    title: "Function inventory",
    prompt: "Give me an inventory of functions so we can spot shrink candidates.",
    commands: ["-f json gql --macro-name all_functions unused"],
    note: "all_functions lists every function. The word unused is only a required QUERY placeholder — not a dead-code filter.",
    output: `{
  "schema_version": 1,
  "count": 292,
  "rows": [
    [{"binding": "f", "node": "formatLegacyCsvExport", "type": "Function"}]
  ]
}`,
    reasoning:
      "Use the inventory, then verify callers with blast-radius / CALL queries before deleting anything.",
  },
  {
    id: "communities",
    flow: "Modernization audit",
    title: "Named communities",
    prompt: "What architectural communities does the graph see?",
    commands: ["-f json gql --macro-name all_communities unused"],
    note: "Lists label-propagation communities — not orphaned modules. Prefer communities list for labels + modularity.",
    output: `{
  "schema_version": 1,
  "count": 40,
  "rows": [
    [{"binding": "c", "type": "Community", "community_id": 19, "label": "legacy_v1_reports", "member_count": 14}]
  ]
}`,
    reasoning:
      "Inspect members and call edges into/out of a community before proposing a prune.",
  },
  {
    id: "semantic",
    flow: "Intent discovery",
    title: "Natural-language search",
    prompt: "Where is the code that handles our checkout flow?",
    commands: [
      "semantic index",
      '-f json semantic query "checkout flow" --limit 10',
    ],
    note: "semantic index is opt-in after discover. Use --embedder vocab|hash offline.",
    output: `{
  "schema_version": 3,
  "query": "checkout flow",
  "dimensions": 256,
  "hits": [
    {"name": "processCart", "score": 0.92, "file_path": "src/checkout/service.ts"},
    {"name": "chargeToken", "score": 0.87, "file_path": "src/payment/gateway.ts"}
  ]
}`,
    reasoning:
      "Semantic hits point at entrypoints. Follow with blast-radius / GQL before editing.",
  },
  {
    id: "community-search",
    flow: "Intent discovery",
    title: "Community-scoped search",
    prompt: "Which subsystem owns checkout?",
    commands: [
      '-f json semantic query "checkout" --scope community --limit 10',
    ],
    output: `{
  "schema_version": 3,
  "query": "checkout",
  "hits": [
    {"node_id": "12", "name": "Order Management & Checkout", "score": 0.88}
  ]
}`,
    reasoning:
      "Treat the matched community as a refactor boundary; list members next.",
  },
  {
    id: "blast",
    flow: "Pre-refactor safety",
    title: "Blast radius",
    prompt: "What's the impact if I change updateQuantity?",
    commands: ['-f json blast-radius "updateQuantity" --depth 2'],
    output: `{
  "schema_version": 2,
  "target": {"symbol": "updateQuantity"},
  "metrics": {
    "score": 12.5,
    "direct_callers_count": 2,
    "impact_zone_size": 5,
    "caller_depth_limit": 2
  },
  "topology": {
    "direct_callers": [
      {"name": "putItem"},
      {"name": "run"}
    ]
  }
}`,
    reasoning:
      "Update direct callers in the same change set before altering the signature.",
  },
  {
    id: "mutations",
    flow: "Pre-refactor safety",
    title: "Field mutations",
    prompt: "Where are ShoppingCart fields mutated?",
    commands: [
      "-f json cpg mutations --type ShoppingCart --exclude-ctors",
    ],
    note: "Requires discover --with-cfg (field-write index).",
    output: `{
  "schema_version": 1,
  "type_name": "ShoppingCart",
  "exclude_ctors": true,
  "mutations": [
    {"file": "src/cart/CartService.ts", "line": 54, "member": "items", "function": "updateQuantity"}
  ]
}`,
    reasoning:
      "Review each write site for encapsulation before a domain refactor.",
  },
  {
    id: "flows",
    flow: "Pre-refactor safety",
    title: "Variable flows",
    prompt: "Trace how quantity flows from CartService.ts.",
    commands: [
      "-f json cpg flows src/cart/CartService.ts --line 50 --variable quantity --function updateQuantity --direction forward",
    ],
    output: `{
  "schema_version": 1,
  "steps": [
    {"line": 50, "code": "quantity = …"},
    {"line": 78, "code": "repo.updateQuantity(…, quantity)"}
  ]
}`,
    reasoning:
      "Add validation at the service boundary before the repository call.",
  },
  {
    id: "check",
    flow: "Policy verification",
    title: "CI policy check",
    prompt: "Validate recent changes against project policies.",
    commands: ["-f json check --policy-file policy.json"],
    note: "Policies use max_impact_nodes / forbidden_crossings — see docs/policy-format.md.",
    output: `{
  "schema_version": 1,
  "passed": true,
  "violations": []
}`,
    reasoning:
      "Zero violations on touched symbols — safe to open a PR after human review.",
  },
];

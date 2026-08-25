import type { TabId } from "./tabDocs";

export interface CliCommandBlock {
  comment?: string;
  commands: string[];
}

export interface CliWorkflowSection {
  /** Dashboard tab this mirrors (guide uses multiple). */
  tabId: TabId;
  tabLabel: string;
  summary: string;
  prerequisite?: string;
  blocks: CliCommandBlock[];
  notes?: string[];
}

/** Shown at the top of the Query Guide. Prefer `--with-*` (there is no `--all`). */
export const GUIDE_PREREQUISITES = `export REPO="$PWD"   # repository root after discover

# Fast index (graph, blast scores, metrics)
rgctl discover .

# Dashboard UI + CFG/PDG (Dataflow, CFG, Slice overlays, CPG mutations)
rgctl discover . --with-cfg --with-dashboard

# Richer pass used by the feature demo / migration tab
rgctl discover . -l java -e target \\
  --with-cfg --with-security --with-taint --with-dashboard --with-harmonic \\
  --export-migration-hints

# Optional: semantic Search tab (after discover)
rgctl semantic index --embedder vocab --dimensions 256`;

/**
 * Examples target rgctl-tests/ecommerce-java (JWT /api/* + CoolStore /services/*).
 * Substitute symbols/paths for other repos.
 */
export const CLI_WORKFLOWS: CliWorkflowSection[] = [
  {
    tabId: "graph",
    tabLabel: "Graph visualization",
    summary:
      "Explore package/community structure, drill into call neighborhoods, and export subgraphs for external tools.",
    prerequisite: "rgctl discover .",
    blocks: [
      {
        comment: "List functions and orient by name",
        commands: [
          'rgctl -r "$REPO" gql --macro-name all_functions unused',
          'rgctl -r "$REPO" gql "MATCH (n:Function) WHERE n.name LIKE \'*Service*\' RETURN n LIMIT 20"',
        ],
      },
      {
        comment: "Named communities (dashboard metagraph labels)",
        commands: [
          'rgctl -r "$REPO" -f json gql --macro-name all_communities unused | jq ".rows[:5]"',
          'rgctl -r "$REPO" communities list | head -15',
        ],
      },
      {
        comment: "Call chains (1–3 hops) — same edges the metagraph summarizes",
        commands: [
          'rgctl -r "$REPO" gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) RETURN a,b LIMIT 50"',
          'rgctl -r "$REPO" gql --macro-name call_chain unused',
          'rgctl -r "$REPO" gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = \'clearCart\' RETURN a,b"',
        ],
      },
      {
        comment: "Export a subgraph (GraphML, Mermaid) for offline layout",
        commands: [
          'rgctl -r "$REPO" export --export-format graphml --export-output subgraph.graphml \\',
          '  --query "name:priceShoppingCart"',
          'rgctl -r "$REPO" export --export-format mermaid --export-output clearCart.mmd \\',
          "  --query 'name:clearCart'",
        ],
      },
      {
        comment: "Many queries in one session — HTTP dashboard + GQL API",
        commands: [
          'rgctl -r "$REPO" serve --open',
          'curl -sS -X POST http://127.0.0.1:8080/api/query \\',
          "  -H 'Content-Type: application/json' \\",
          "  -d '{\"macro\":\"all_functions\"}' | jq '.count'",
        ],
      },
    ],
    notes: [
      "Dashboard metagraph drill-down has no single CLI command; combine GQL + export, or use `serve` for repeated queries.",
      "`export --query` uses filter syntax (`name:`, `type:`, `functions`, `all`) — not full GQL MATCH.",
      "Add `-f json` on any command for scripts and CI.",
    ],
  },
  {
    tabId: "search",
    tabLabel: "Semantic search",
    summary:
      "Natural-language function search (and community-scoped search) — same as the Search tab after `semantic index`.",
    prerequisite: "rgctl discover . && rgctl semantic index --embedder vocab",
    blocks: [
      {
        comment: "Build the Hamming index (offline vocab embedder)",
        commands: ['rgctl -r "$REPO" semantic index --embedder vocab --dimensions 256'],
      },
      {
        comment: "Query functions",
        commands: [
          'rgctl -r "$REPO" -f json semantic query "shopping cart checkout" --limit 5 \\',
          "  | jq '.hits[:3] | map({name, score})'",
        ],
      },
      {
        comment: "Scope to communities (Search tab community mode)",
        commands: [
          'rgctl -r "$REPO" -f json semantic query "shopping cart" --scope community --limit 3 \\',
          "  | jq '.hits | map({name, ranking, score})'",
        ],
      },
    ],
    notes: [
      "With `rgctl serve`, the Search tab hits `POST /api/semantic/query` on the same index.",
    ],
  },
  {
    tabId: "functions",
    tabLabel: "Function inventory",
    summary:
      "Browse symbols with PageRank, betweenness, harmonic centrality, and blast scores — same columns as the Functions table.",
    prerequisite: "rgctl discover .",
    blocks: [
      {
        comment: "Inventory all functions",
        commands: [
          'rgctl -r "$REPO" gql --macro-name all_functions unused',
          'rgctl -r "$REPO" -f json gql --macro-name all_functions unused | jq ".count"',
        ],
      },
      {
        comment: "Centrality reports (Functions table PR / BC columns)",
        commands: [
          'rgctl -r "$REPO" metrics --pagerank',
          'rgctl -r "$REPO" metrics --betweenness',
          'rgctl -r "$REPO" -f json metrics --pagerank | jq ".pagerank.top[:10]"',
        ],
      },
      {
        comment: "Filter by type or file path",
        commands: [
          'rgctl -r "$REPO" gql "MATCH (n:Class) RETURN n LIMIT 30"',
          'rgctl -r "$REPO" gql "MATCH (n:Function) WHERE n.file_path LIKE \'*coolstore*\' RETURN n"',
        ],
      },
    ],
    notes: [
      "Blast scores in the table come from discover; re-run `blast-radius <symbol>` for caller lists at a chosen depth.",
      "JSON shape is `.pagerank.top` / `.betweenness.top` — not `.rows`.",
    ],
  },
  {
    tabId: "cfg",
    tabLabel: "CFG / PDG analysis",
    summary:
      "Inspect control-flow blocks, branches, and dominance inside one function — equivalent to the CFG graph + dominance panel.",
    prerequisite: "rgctl discover . --with-cfg",
    blocks: [
      {
        comment: "CFG for CoolStore pricing (ecommerce-java)",
        commands: [
          'rgctl -r "$REPO" inspect priceShoppingCart cfg',
          'rgctl -r "$REPO" -f mermaid inspect priceShoppingCart cfg',
          'rgctl -r "$REPO" inspect priceShoppingCart cfg --prune',
        ],
      },
      {
        comment: "JWT /api checkout method",
        commands: [
          'rgctl -r "$REPO" inspect checkout cfg',
          'rgctl -r "$REPO" -f json inspect checkout cfg | jq "{blocks: (.nodes|length), edges: (.edges|length)}"',
        ],
      },
      {
        comment: "Dominator tree + frontiers",
        commands: ['rgctl -r "$REPO" inspect priceShoppingCart dom --frontiers'],
      },
    ],
    notes: [
      "`inspect` takes a **function** symbol only (no `--class`). Prefer unique names (`priceShoppingCart`) or `Class::method`.",
      "Large repos: dashboard loads one function on demand from the CFG archive; CLI `inspect` reads `.rgctl/analysis/`.",
    ],
  },
  {
    tabId: "dataflow",
    tabLabel: "Dataflow",
    summary:
      "Statement-level PDG / dominator views, plus the Field mutations (CPG) panel for typed writes such as ShoppingCart.",
    prerequisite: "rgctl discover . --with-cfg --with-dashboard",
    blocks: [
      {
        comment: "CPG status + field mutations (Dataflow → Field mutations panel)",
        commands: [
          'rgctl -r "$REPO" cpg status',
          'rgctl -r "$REPO" cpg mutations --type ShoppingCart --exclude-ctors',
          'rgctl -r "$REPO" -f json cpg mutations --type ShoppingCart --exclude-ctors',
        ],
      },
      {
        comment: "PDG / dataflow edges for pricing",
        commands: [
          'rgctl -r "$REPO" inspect priceShoppingCart pdg --edge-layer data',
          'rgctl -r "$REPO" inspect priceShoppingCart pdg --def-use',
          'rgctl -r "$REPO" -f mermaid inspect priceShoppingCart pdg --edge-layer data',
        ],
      },
      {
        comment: "Dominator tree (Dataflow → Dominator Tree view)",
        commands: ['rgctl -r "$REPO" inspect priceShoppingCart dom --frontiers'],
      },
      {
        comment: "CALL neighborhood via CPG façade",
        commands: [
          'rgctl -r "$REPO" cpg function priceShoppingCart',
          'rgctl -r "$REPO" cpg calls \'ShoppingCartService::priceShoppingCart\'',
        ],
      },
    ],
    notes: [
      "C fixtures: query the struct typedef (`shopping_cart_t`), not `ShoppingCart`.",
      "Empty mutations ⇒ no typed non-ctor writes recovered (try `--include-unresolved`).",
    ],
  },
  {
    tabId: "slice",
    tabLabel: "Program slicing",
    summary:
      "Backward or forward line-level slice for a variable at a line — same as Compute slice in the dashboard.",
    prerequisite: "rgctl discover . --with-cfg",
    blocks: [
      {
        comment: "Backward slice — JWT cart addItem (ecommerce-java)",
        commands: [
          'rgctl -r "$REPO" slice \\',
          "  src/main/java/com/example/ecommerce/service/CartService.java \\",
          "  --line 53 --variable item --function addItem",
        ],
      },
      {
        comment: "Forward slice",
        commands: [
          'rgctl -r "$REPO" slice \\',
          "  src/main/java/com/example/ecommerce/service/CartService.java \\",
          "  --line 53 --variable item --function addItem --direction forward",
        ],
      },
      {
        comment: "JSON for automation",
        commands: [
          'rgctl -r "$REPO" -f json slice \\',
          "  src/main/java/com/example/ecommerce/service/CartService.java \\",
          "  --line 53 --variable item --function addItem | jq .",
        ],
      },
    ],
    notes: [
      "`--function` is the **method** name (`addItem`), not the class name.",
    ],
  },
  {
    tabId: "blast",
    tabLabel: "Blast radius",
    summary:
      "Upstream impact if you change a symbol — impact score, direct callers, and impact zone (dashboard table is depth-limited).",
    prerequisite: "rgctl discover .",
    blocks: [
      {
        comment: "JWT /api cart clear",
        commands: [
          'rgctl -r "$REPO" blast-radius \'CartService::clearCart\'',
          'rgctl -r "$REPO" -f json blast-radius \'CartService::clearCart\' | jq "{score: .metrics.score, callers: .topology.direct_callers}"',
        ],
      },
      {
        comment: "CoolStore /services pricing",
        commands: [
          'rgctl -r "$REPO" blast-radius \'ShoppingCartService::priceShoppingCart\'',
          'rgctl -r "$REPO" -f json blast-radius \'ShoppingCartService::priceShoppingCart\' | jq ".metrics"',
        ],
      },
      {
        comment: "Limit caller depth (matches dashboard depth slider)",
        commands: [
          'rgctl -r "$REPO" blast-radius \'CartService::clearCart\' --depth 1',
          'rgctl -r "$REPO" blast-radius \'CartService::clearCart\' --depth 5',
        ],
      },
      {
        comment: "CI policy gate on changed functions",
        commands: [
          'rgctl -r "$REPO" -f json check --policy-file "$REPO/../rgctl-policy.json" \\',
          "  | jq '{schema_version, violations: (.violations|length)}'",
        ],
      },
    ],
    notes: [
      "Sidebar scores are full-graph metrics from discover; the caller table respects the depth slider.",
      "Prefer `Class::method` when simple names collide.",
    ],
  },
  {
    tabId: "taint",
    tabLabel: "Taint analysis",
    summary:
      "Source-to-sink flows and sanitizer checks per function — requires CFG/PDG from discover.",
    prerequisite: "rgctl discover . --with-cfg --with-taint --with-dashboard",
    blocks: [
      {
        comment: "On-demand taint at a program point",
        commands: [
          'rgctl -r "$REPO" slice \\',
          "  src/main/java/com/example/ecommerce/service/CartService.java \\",
          "  --line 53 --variable item --function addItem --taint",
        ],
      },
      {
        comment: "Find CoolStore / JWT endpoints, then trace",
        commands: [
          'rgctl -r "$REPO" gql "MATCH (n:Function) WHERE n.name LIKE \'*checkout*\' OR n.name LIKE \'*Endpoint*\' RETURN n LIMIT 20"',
          'rgctl -r "$REPO" slice <file> --line <N> --variable <VAR> --function <method> --taint',
        ],
      },
    ],
    notes: [
      "Dashboard Taint tab lists flows exported at discover time (`--with-taint`); CLI `slice --taint` re-runs analysis on demand.",
    ],
  },
  {
    tabId: "migration",
    tabLabel: "Migration planner",
    summary:
      "Package-level extraction roadmap from communities, centrality, and blast — export plan JSON for agents or CI.",
    prerequisite:
      "rgctl discover . --with-cfg --with-dashboard --with-harmonic --export-migration-hints",
    blocks: [
      {
        comment: "Default hybrid strategy",
        commands: [
          'rgctl discover . --with-cfg --with-dashboard --with-harmonic --export-migration-hints',
          'jq ".packages[:5]" .rgctl/dashboard/migration_plan.json',
        ],
      },
      {
        comment: "Strategy presets (dashboard α/β/γ presets)",
        commands: [
          'rgctl discover . --with-cfg --with-harmonic --export-migration-hints \\',
          "  --migration-preset risk_mitigation",
          'rgctl discover . --with-cfg --with-harmonic --export-migration-hints \\',
          "  --migration-preset hotspot_first",
          'rgctl discover . --with-cfg --with-harmonic --export-migration-hints \\',
          "  --migration-order priority",
        ],
      },
    ],
    notes: [
      "There is no `discover --all` — compose `--with-cfg`, `--with-taint`, `--with-dashboard`, `--with-harmonic`, etc.",
      "`--export-migration-plan` remains a deprecated alias of `--export-migration-hints`.",
      "Interactive weight tuning is dashboard-only; re-run discover with presets to refresh CLI exports.",
    ],
  },
  {
    tabId: "guide",
    tabLabel: "GQL reference",
    summary: "Core graph queries used across tabs — patterns, macros, and JSON output.",
    prerequisite: "rgctl discover .",
    blocks: [
      {
        comment: "Macros (shortcuts)",
        commands: [
          'rgctl -r "$REPO" gql --macro-name all_functions unused',
          'rgctl -r "$REPO" gql --macro-name all_communities unused',
          'rgctl -r "$REPO" gql --macro-name direct_calls unused',
          'rgctl -r "$REPO" gql --macro-name call_chain unused',
        ],
      },
      {
        comment: "Patterns",
        commands: [
          'rgctl -r "$REPO" -f json gql "MATCH (n:Function) RETURN n LIMIT 5" | jq ".count"',
          'rgctl -r "$REPO" gql "MATCH (a:Function)-[:CALLS]->(b:Function) RETURN a,b LIMIT 25"',
          'rgctl -r "$REPO" gql --explain "MATCH (n:Function) WHERE n.name = \'clearCart\' RETURN n"',
        ],
      },
      {
        comment: "HTTP (same as serve /api/query — GraphQL alias at /graphql)",
        commands: [
          'curl -sS -X POST http://127.0.0.1:8080/api/query \\',
          "  -H 'Content-Type: application/json' \\",
          "  -d '{\"query\":\"MATCH (n:Function) WHERE n.name LIKE \\\"*Cart*\\\" RETURN n LIMIT 10\"}' | jq .",
        ],
      },
    ],
    notes: [
      "`POST /graphql` is an alias of `/api/query`; body is JSON (`query` or `macro`), not a GraphQL schema document.",
    ],
  },
];

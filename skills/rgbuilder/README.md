# rgBuilder Skill

A skill for answering structural questions about codebases using the rgBuilder CLI graph.

## Quick Stats

- **Main skill:** 352 lines (57% reduction from original 814 lines)
- **Total documentation:** 1,505 lines (85% more comprehensive coverage)
- **Files:** 5 reference files + main skill
- **Workflow families:** 6 (organized by user intent)
- **NL routing examples:** 20+ common user utterances

## Structure

```
skills/rgbuilder/
├── SKILL.md                              # Main skill (352 lines)
├── README.md                             # This file
└── references/
    ├── command-encyclopedia.md           # All commands with JSON samples (19KB)
    ├── workflows.md                      # Migration, refactor, audit scenarios (8.6KB)
    ├── gql-reference.md                  # GQL patterns & limitations (4.7KB)
    └── communities-and-policy.md         # Community detection + CI policy (13KB)
```

## What's Covered

### Main SKILL.md (Always Loaded)

- When to use rgBuilder
- **MCP vs CLI decision** (7 MCP tools table)
- **6 workflow families:**
  1. Discovery & Indexing
  2. Query & Search (includes communities)
  3. Impact & Safety (includes policy checks)
  4. Metrics & Analysis
  5. Code Analysis (CFG/PDG/slicing)
  6. Export & Visualization
- **NL routing table** (20+ user utterances → commands)
- Common scenarios (migration, pre-refactor safety)
- Failure playbook

### References (Loaded On-Demand)

#### command-encyclopedia.md
- All 15+ commands with full details
- JSON sample responses
- Prerequisites and pitfalls
- "What to report" guidelines

#### workflows.md
- Migration & audit workflows
- Intent discovery & subsystem mapping
- Pre-refactor safety analysis
- CI gates & policy
- Advanced patterns

#### gql-reference.md
- Cypher subset capabilities
- Macros (all_functions, all_communities)
- Valid edge types
- LIKE pattern matching limitations
- Common patterns & troubleshooting

#### communities-and-policy.md
- **Community Detection:**
  - What communities are (implicit architecture)
  - Commands (list, query, label, semantic scope)
  - Use cases (microservice extraction, ownership)
  - 5 complete workflows
- **CI Policy Checks:**
  - Policy schema (max_impact_nodes, centrality, forbidden_crossings)
  - CI integration (GitHub Actions, GitLab)
  - Crafting policies (calibration, gradual tightening)
  - 4 complete workflows
- Combined workflows using both features

## Design Principles

✅ **Progressive disclosure** - Main skill <500 lines, details in references
✅ **Workflow-centric** - Organized by user intent, not commands
✅ **MCP alignment** - Follows same pattern as MCP guide
✅ **Clear routing** - Natural language → tool mapping
✅ **Comprehensive** - All features documented with examples
✅ **Integration** - Shows how features work together

## Installation

From another repo:

```bash
rgctl install --skill
```

This writes `.claude/skills/rgbuilder/` and `.cursor/skills/rgbuilder/` from the embedded skill.

## See Also

- [User Guide](../../docs/user-guide.md) - Complete CLI tutorial
- [MCP Server Guide](../../docs/guides/mcp-server.md) - MCP setup
- [JSON API](../../docs/json-api.md) - Schema specifications
- [All Guides](../../docs/guides/README.md) - Feature-specific guides

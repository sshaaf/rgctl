# rgctl Skill

A skill for answering structural questions about codebases using the rgctl CLI graph.

## Quick Stats

- **Main skill:** 352 lines (57% reduction from original 814 lines)
- **Total documentation:** 1,505 lines (85% more comprehensive coverage)
- **Files:** 5 reference files + main skill
- **Workflow families:** 6 + Kantra rules
- **NL routing examples:** 25+ common user utterances

## Structure

```
skills/rgctl/
├── SKILL.md                              # Main skill (352 lines)
├── README.md                             # This file
├── workflows/                            # Source for slash skills + references/workflows.md (assembled at build)
└── references/
    ├── command-encyclopedia.md           # All commands with JSON samples (19KB)
    ├── workflows.md                      # Generated from workflows/ at build (do not edit by hand)
    ├── gql-reference.md                  # GQL patterns & limitations (4.7KB)
    └── communities-and-policy.md         # Community detection + CI policy (13KB)
```

## What's Covered

### Main SKILL.md (Always Loaded)

- When to use rgctl
- **CLI subprocess workflow** — spawn `rgctl -f json` for agents
- **6 workflow families:**
  1. Discovery & Indexing
  1b. Konveyor Kantra rules (`--with-kantra`)
  2. Query & Search (includes communities + KantraRule GQL)
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

#### workflows.md (generated)
- Assembled from `workflows/*.md` when the agent pack is built (`cargo build`)
- Edit fragments under `workflows/` (e.g. `migrate.md`, `kantra.md`); order comes from `agent-pack/manifest.yaml`

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
✅ **CLI-first** - Agents use `rgctl -f json` subprocesses (optional `serve` HTTP)
✅ **Clear routing** - Natural language → tool mapping
✅ **Comprehensive** - All features documented with examples
✅ **Integration** - Shows how features work together

## Installation

From a target repository (not the rgctl source tree unless you are dogfooding):

```bash
rgctl install --skill --with-commands --tools cursor,claude,codex,agents
```

Installs meta skill `rgctl`, workflow skills (`rgctl-discover`, …), and optional slash commands per adapter. See [Agent commands guide](../../docs/guides/agent-commands.md).

**Maintainers:** edit workflow bodies under `workflows/`; run `cargo build` to refresh `references/workflows.md`.

## See Also

- [User Guide](../../docs/user-guide.md) - Complete CLI tutorial
- [Agent recipes](../../docs/agent-recipes.md) - Copy-paste CLI workflows
- [HTTP API](../../docs/http-api.md) - Optional `rgctl serve` for repeated queries
- [JSON API](../../docs/json-api.md) - Schema specifications
- [All Guides](../../docs/guides/README.md) - Feature-specific guides

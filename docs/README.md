# rgBuilder documentation

Agent-first docs: index once, query with `-f json`, deepen in the User Guide when a human needs the full walkthrough.

## Primary (start here)

| Goal | Canon |
|------|--------|
| Step-by-step feature how-tos (CoolStore) | **[Guides](guides/README.md)** |
| LLM / agent workflows | [AGENTS.md](../AGENTS.md) · [Agent recipes](agent-recipes.md) |
| JSON shapes (`schema_version`, fields) | [JSON API](json-api.md) |
| Install + CLI walkthrough (ecommerce-java) | [User Guide](user-guide.md) |
| Concepts (what / why) | [Introduction](Introduction.md) |

**Agent loop:** [AGENTS.md](../AGENTS.md) → `discover` once → `gql` / `blast-radius` / `cpg` with `-f json`.  
**First hour (human):** User Guide §1–4 on [ecommerce-java](user-guide.md#3-example-project-ecommerce-java), then a [Guide](guides/README.md) for the feature you need.

## Secondary

| Goal | Doc |
|------|-----|
| Supported languages | [Languages](languages.md) |
| Markdown / doc context graph | [Markdown context](markdown-context.md) — `.md` / `.mdx`, GQL, Obsidian vault export, doc semantic index |
| FAQ / glossary | [FAQ](faq.md) · [Glossary](glossary.md) |
| HTTP `serve` query API | [HTTP API](http-api.md) |
| CI blast-radius policy | [Policy format](policy-format.md) |
| Monolith migration (how-to) | [Building a migration plan](building-migration-plan.md) |
| Research map | [Further reading](further-reading.md) |

Optional browser UI (nice-to-have, not required for agents): [Dashboard user guide](dashboard-user-guide.md) after `discover --with-dashboard`.

## For contributors

Internals and contribution bars — not the default agent reading path.

| Document | Topic |
|----------|--------|
| [Contributor checklist](contributor-checklist.md) | End-to-end workflow: add language, update feature, tests, PR |
| [Feature designs](design/README.md) | Per-capability engineering notes |
| [Tier 1 language support](tier-1-language-support.md) | Layer A–F bar for Tier 1 languages |
| [Code structure](Code_structure.md) | Crate layout |
| [Analysis architecture](analysis-architecture.md) | CFG / PDG / taint |
| [Graph storage architecture](graph-storage-architecture.md) | Snapshots, blast cache |
| [CLI I/O sanity QE](cli-io-sanity-qe.md) | Golden-path test contract |
| [Dashboard design](dashboard-design.md) | WASM export pipeline |
| [Migration planner design](design/migration-planner-design.md) · [Migration algorithms](migration-algorithms.md) · [Harmonic centrality](harmonic-centrality.md) | Migration internals |
| [Releasing](releasing.md) | Versioned binaries |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Dev setup |

## Terminology

| Term | Meaning |
|------|---------|
| Tier 1 languages | Nine always-linked plugins (see [languages.md](languages.md)) |
| `--with-cfg` | CFG/PDG archive (prefer over legacy `--cfg`) |
| Communities | Label propagation (Raghavan 2007); `louvain_community_id` is historical |
| Dashboard / migration JSON | Opt-in (`--with-dashboard` / `--export-migration-hints`) |
| First-hour fixture | In-tree **ecommerce-java** |

## Redirects

- [cli-getting-started.md](cli-getting-started.md) → User Guide  
- [cli-output-schemas.md](cli-output-schemas.md) → [JSON API](json-api.md)  
- [LANGUAGE_GUIDE.md](LANGUAGE_GUIDE.md) → [languages.md](languages.md)

Docs match the CLI in this repository — verify with `rgctl --version`.

Maintainer scratch: [`internal/`](internal/) (not public).

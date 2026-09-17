# Agent pack install

Multi-tool agent pack: workflow skills, optional slash commands, optional structural policy, and a registry of ~40 agent adapters.

## Summary

| Area | Change |
|------|--------|
| **Install** | `rgctl install --skill` installs meta skill `rgctl` plus workflow skills `rgctl-discover`, `rgctl-impact`, `rgctl-flow`, `rgctl-search`, `rgctl-gql`, `rgctl-migrate`, `rgctl-kantra`, `rgctl-gate`. |
| **Commands** | `--with-commands` adds per-adapter slash/prompt files (Cursor `/rgctl-gql`, Claude `/rgctl:gql`, Pi `.pi/prompts/`, …). |
| **Targeting** | `--tools cursor,claude` or `--tools all`. **`--host` is deprecated** (warning only; use `--tools`). |
| **Scope** | `-g` / `--global` for user-level agent dirs. |
| **Policy** | `--with-policy` installs Cursor structural rule snippet. |
| **Discovery** | `install --list-agents` prints `agent-pack/agents/registry.toml` entries. |
| **JSON** | `install -f json` uses **`schema_version`: 2** (`agent`, `workflow`, `kind`, `scope`, …). |
| **Workflow docs** | Single source: `skills/rgctl/workflows/*.md`; `references/workflows.md` generated at build. |
| **Embed** | Agent pack generated in `build.rs`, zipped into the binary (replaces single-tree `include_dir` for install). |

## Upgrade

```bash
# Install newer rgctl binary, then in each repo:
rgctl install --skill --with-commands --tools cursor,claude --force
```

## Documentation

- [Agent commands guide](../guides/agent-commands.md) — full install flag reference
- [Agent skill guide](../guides/agent-skill.md) — walkthrough
- [JSON API §18](../json-api.md#18-install)

Tracks GitHub [#84](https://github.com/sshaaf/rgctl/issues/84) and OpenSpec change `agent-pack-install`.

# Agent pack install

Multi-tool agent pack: single skill `rgctl`, optional structural policy, and a registry of ~40 agent adapters.

## Summary

| Area | Change |
|------|--------|
| **Install** | `rgctl install --skill` installs one skill named `rgctl` (workflow playbooks live under `references/workflows.md`). |
| **Targeting** | `--tools cursor,claude` or `--tools all`. **`--host` is deprecated** (warning only; use `--tools`). |
| **Scope** | `-g` / `--global` for user-level agent dirs. |
| **Policy** | `--with-policy` installs Cursor structural rule snippet. |
| **Discovery** | `install --list-agents` prints `agent-pack/agents/registry.toml` entries. |
| **JSON** | `install -f json` uses **`schema_version`: 3** (`agent`, `workflow`, `kind`, `scope`, …). |
| **Workflow docs** | `skills/rgctl/references/workflows.md` is copied into the pack at build. |
| **Embed** | Agent pack generated in `build.rs`, zipped into the binary (replaces single-tree `include_dir` for install). |

## Upgrade

```bash
rgctl install --skill --tools cursor,claude --force
```

## Related

- [Agent pack walkthrough](../guides/agent-skill.md)
- GitHub [#84](https://github.com/sshaaf/rgctl/issues/84) · OpenSpec change `agent-pack-install`.

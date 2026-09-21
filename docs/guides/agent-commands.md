# rgctl agent pack (install & chat commands)

rgctl has two command surfaces:

| Layer | What it is | Where it lives |
|--------|------------|----------------|
| **Engine** | `rgctl discover`, `rgctl -f json gql`, … | Your PATH; graph artifacts in `{repo}/.rgctl/` |
| **Agent pack** | Skills, optional slash commands, optional policy | Repo-local agent dirs (or home with `-g`) |

The pack is **embedded in the `rgctl` binary** (no separate download). Install the CLI first: [Installation](../installation.md).

Workflow text is authored under **`skills/rgctl/workflows/`**; `references/workflows.md` in the installed meta skill is **assembled at build time** from those fragments (single source of truth).

---

## Command shape

```bash
rgctl [-r REPO] install [FLAGS]
```

- **`-r REPO`** — Repository root where agent directories are written (default: current working directory). Use the same root as `discover`.
- **`install`** — Copies files from the embedded agent pack.

You must pass at least one of **`--skill`** or **`--with-policy`**.

### Flags

| Flag | Effect |
|------|--------|
| **`--skill`** | Meta skill **`rgctl`** (router + `references/`) and eight workflow skills: `rgctl-discover`, `rgctl-impact`, `rgctl-flow`, `rgctl-search`, `rgctl-gql`, `rgctl-migrate`, `rgctl-kantra`, `rgctl-gate`. |
| **`--with-commands`** | Chat slash commands / prompts per adapter (e.g. Cursor `/rgctl-gql`, Claude `/rgctl:gql`). Use with **`--skill`** for the full experience. |
| **`--with-policy`** | Structural bias snippet (e.g. `.cursor/rules/rgctl-structural.mdc`). Optional; does not replace skills. |
| **`--tools id1,id2`** or **`--tools all`** | Which **registry adapters** receive files. **Default (omit flag):** `cursor`, `claude`, `codex`, `agents`, `antigravity`. **`all`** = full registry (~40 products). Unknown ids: stderr warning; if none valid, exit **1**. |
| **`-g` / `--global`** | Install under your **home** (e.g. `~/.cursor/skills/…`) instead of repo-local paths. Only agents with `supports_global: true` in the registry (see `--list-agents`). |
| **`--list-agents`** | Print the registry table and exit (no install). |
| **`--force`** | Overwrite rgctl-managed files that differ from the bundled version. |
| **`--host`** | **Deprecated** — use **`--tools`**. |

### Typical installs

```bash
cd /path/to/your-app

# Skills + slash commands for common IDEs (repo-local)
rgctl install --skill --with-commands --tools cursor,claude,codex,antigravity,agents

# Cursor only
rgctl install --skill --with-commands --tools cursor

# Antigravity only
rgctl install --skill --with-commands --tools antigravity

# Full registry (many dot-directories)
rgctl install --skill --with-commands --tools all

# Optional Cursor rule when .rgctl/ exists
rgctl install --skill --with-commands --tools cursor --with-policy

# Global install (user-level agent dirs)
rgctl install --skill --with-commands -g --tools cursor

# Inspect adapters before installing
rgctl install --list-agents
```

### After install

Install does **not** run `discover`. Index the codebase separately:

```bash
export REPO=/path/to/your-app
cd "$REPO"
rgctl discover .
# Chat: /rgctl-gql …   OR   shell: rgctl -f json gql '…'
```

Add `.rgctl/` and agent skill dirs to `.gitignore` if you want them local-only.

### Upgrades

1. Install a newer **`rgctl`** binary.
2. Re-run install with **`--force`** if managed files already exist and differ.

---

## What gets written

Paths come from **`agent-pack/agents/registry.toml`** (per-product `agent_dir`, `skills_subdir`, `commands_subdir`).

| Kind | Example (Cursor, repo-local) |
|------|------------------------------|
| Meta skill | `.cursor/skills/rgctl/SKILL.md` + `references/` |
| Workflow skill | `.cursor/skills/rgctl-gql/SKILL.md`, … |
| Command | `.cursor/commands/rgctl-gql.md` |
| Policy | `.cursor/rules/rgctl-structural.mdc` (with `--with-policy`) |

**Shared dedup:** **`codex`**, **`agents`**, and **`zed`** use **`.agents/skills/`**; install writes each destination once.

### Adapter examples

| Agent | Skills | Commands / prompts |
|-------|--------|-------------------|
| Cursor | `.cursor/skills/rgctl-*` | `.cursor/commands/rgctl-*.md` |
| Claude | `.claude/skills/rgctl-*` | `.claude/commands/` (`rgctl:gql` style) |
| OpenCode | `.opencode/skills/rgctl-*` | `.opencode/commands/rgctl-*.md` |
| Pi | `.pi/skills/rgctl-*` | `.pi/prompts/rgctl-*.md` |
| GitHub Copilot | `.github/skills/` | `.github/prompts/*.prompt.md` |

Run **`rgctl install --list-agents`** for the full table.

---

## Workflows (chat vs CLI)

Chat commands are **steering wheels**; the engine remains the terminal CLI.

| Workflow | Cursor (example) | Claude (example) | Primary CLI |
|----------|------------------|------------------|-------------|
| Index | `/rgctl-discover` | `/rgctl:discover` | `discover` |
| Impact | `/rgctl-impact` | `/rgctl:impact` | `blast-radius` |
| Data flow | `/rgctl-flow` | `/rgctl:flow` | `slice`, `cpg flows`, … |
| Search | `/rgctl-search` | `/rgctl:search` | `semantic query` |
| GQL | `/rgctl-gql` | `/rgctl:gql` | `gql` |
| Migration roadmap | `/rgctl-migrate` | `/rgctl:migrate` | discover + `migration_plan.json` |
| Kantra rules | `/rgctl-kantra` | `/rgctl:kantra` | `discover --with-kantra` |
| CI gate | `/rgctl-gate` | `/rgctl:gate` | `check`, `pr-check` |

**Migrate** (roadmap / `migration_plan.json`) and **Kantra** (Konveyor / `kantra_findings.json`) are **separate** workflows — do not conflate them in prompts or reports.

---

## JSON output

```bash
rgctl -r "$REPO" -f json install --skill --with-commands --tools cursor
```

Uses **`schema_version`: 2** (`scope`, `agents`, `with_commands`, per-write `agent`, `workflow`, `kind`, `status`). See [JSON API §18](../json-api.md#18-install). If any write is `skipped_exists`, JSON is still printed and the process exits **1**.

---

## Structural policy (optional)

```bash
rgctl install --with-policy --tools cursor
```

**Cursor-only today:** writes `.cursor/rules/rgctl-structural.mdc` regardless of `--tools` (other agents have no policy adapter yet). Best-effort nudge toward `rgctl -f json` when `.rgctl/` exists; agents cannot hard-block grep.

### AGENTS.md for *your* repo

Prefer **`rgctl install --skill`**. If you skip `--with-policy` / skills, paste either:

- the short nudge below into your repo’s root **`AGENTS.md`**, or
- the full playbook from [USER_AGENTS_TEMPLATE.md](../agents/USER_AGENTS_TEMPLATE.md)

```markdown
When `.rgctl/` exists, answer structural questions via `rgctl -f json` before ripgrep or bulk file reads.
```

The **rgctl source tree** root [`AGENTS.md`](../../AGENTS.md) is for **contributing to rgctl** (not a consumer CLI cookbook).

---

## Related

- [Agent skill](agent-skill.md) — use cases and agent loop
- [Installation](../installation.md) — binary setup
- [JSON API §18](../json-api.md#18-install) — install payload types
- [USER_AGENTS_TEMPLATE.md](../agents/USER_AGENTS_TEMPLATE.md) — paste into consumer repos
- [AGENTS.md](../../AGENTS.md) — contributor agent README for this repository
- Release notes: [agent-pack-install](../releases/agent-pack-install.md) · GitHub [#84](https://github.com/sshaaf/rgctl/issues/84)

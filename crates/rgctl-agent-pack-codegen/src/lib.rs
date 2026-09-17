//! Generate the embedded agent pack from manifest, `skills/rgctl/workflows/`, and agent adapters.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Agent adapter definition (`agent-pack/agents/*.toml`).
#[derive(Debug, Clone, Deserialize)]
pub struct AgentDef {
    pub id: String,
    pub agent_dir: String,
    pub skills_subdir: String,
    pub commands_subdir: String,
    pub command_style: String,
    #[serde(default)]
    pub command_extension: String,
    pub invoke_prefix: String,
    pub supports_global: bool,
    pub global_skills: String,
    pub global_commands: String,
    pub commands_enabled: bool,
}

/// Installed pack manifest written to `out/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub profile: String,
    pub version: u32,
    pub rgctl_version: String,
    pub workflows: Vec<WorkflowEntry>,
    pub agents: Vec<AgentManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEntry {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifestEntry {
    pub id: String,
    pub agent_dir: String,
    pub skills_subdir: String,
    pub commands_subdir: Option<String>,
    pub skills_path: String,
    pub commands_path: Option<String>,
    pub invoke_prefix: String,
    pub supports_global: bool,
}

#[derive(Debug, Deserialize)]
struct RootManifest {
    profile: String,
    version: u32,
    workflows: Vec<WorkflowEntry>,
    meta_skills: Vec<WorkflowEntry>,
}

/// Generate the full agent pack under `out_dir`.
pub fn generate(pack_root: &Path, out_dir: &Path, rgctl_version: &str) -> Result<(), String> {
    if out_dir.exists() {
        fs::remove_dir_all(out_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    let manifest_yaml = fs::read_to_string(pack_root.join("manifest.yaml")).map_err(|e| e.to_string())?;
    let root: RootManifest = serde_yaml::from_str(&manifest_yaml).map_err(|e| e.to_string())?;
    if !root.meta_skills.iter().any(|m| m.id == "rgctl") {
        return Err("manifest meta_skills must include id rgctl".into());
    }

    let repo_root = repo_root_from_pack(pack_root);
    let workflows_dir = skills_workflows_dir(&repo_root);
    let agents = load_agents(&pack_root.join("agents"))?;
    let footer = read_workflow_fragment(&workflows_dir, "_shared-footer")?;
    let workflows_reference =
        assemble_workflows_reference(&root.workflows, &workflows_dir)?;

    for agent in &agents {
        for wf in &root.workflows {
            let body = workflow_body(&workflows_dir, wf.id.as_str(), &footer)?;
            let skill_name = format!("rgctl-{}", wf.id);
            let description = format!(
                "{}. Use for rgctl {} workflow. Spawn rgctl -f json; parse schema_version from stdout.",
                wf.title, wf.id
            );
            let skill_md = render_skill(&skill_name, &description, &body, rgctl_version);
            let skill_rel = format!(
                "{}/rgctl-{}/SKILL.md",
                agent.skills_subdir,
                wf.id
            );
            write_agent_file(out_dir, agent, &skill_rel, skill_md)?;

            if agent.commands_enabled && !agent.commands_subdir.is_empty() {
                let cmd = render_command(agent, &wf.id, &wf.title, rgctl_version);
                let cmd_rel = command_rel_path(agent, &wf.id);
                write_agent_file(out_dir, agent, &cmd_rel, cmd)?;
            }
        }

        // Meta router skill: copy tree from skills/rgctl if present
        let meta_src = pack_root
            .parent()
            .unwrap_or(pack_root)
            .join("skills/rgctl");
        if meta_src.is_dir() {
            let meta_dest = agent_out_root(out_dir, &agent.id)
                .join(&agent.skills_subdir)
                .join("rgctl");
            copy_meta_skill_tree(&meta_src, &meta_dest)?;
            let ref_dest = meta_dest.join("references/workflows.md");
            if let Some(parent) = ref_dest.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&ref_dest, &workflows_reference).map_err(|e| e.to_string())?;
            prepend_router_note(
                &meta_dest.join("SKILL.md"),
                agent,
                &root.workflows,
                rgctl_version,
            )?;
        }
    }

    sync_repo_workflows_reference(&repo_root, &root.workflows, &workflows_dir)?;

    let pack_manifest = PackManifest {
        profile: root.profile.clone(),
        version: root.version,
        rgctl_version: rgctl_version.to_string(),
        workflows: root.workflows.clone(),
        agents: agents
            .iter()
            .map(|a| AgentManifestEntry {
                id: a.id.clone(),
                agent_dir: a.agent_dir.clone(),
                skills_subdir: a.skills_subdir.clone(),
                commands_subdir: if a.commands_enabled && !a.commands_subdir.is_empty() {
                    Some(a.commands_subdir.clone())
                } else {
                    None
                },
                skills_path: format!("{}/{}/", a.agent_dir, a.skills_subdir),
                commands_path: if a.commands_enabled && !a.commands_subdir.is_empty() {
                    Some(format!("{}/{}/", a.agent_dir, a.commands_subdir))
                } else {
                    None
                },
                invoke_prefix: a.invoke_prefix.clone(),
                supports_global: a.supports_global,
            })
            .collect(),
    };
    let json = serde_json::to_string_pretty(&pack_manifest).map_err(|e| e.to_string())?;
    fs::write(out_dir.join("manifest.json"), json).map_err(|e| e.to_string())?;

    // Policy snippet (cursor rules format)
    let policy = render_policy(rgctl_version);
    let policy_path = out_dir.join("policy/rgctl-structural.mdc");
    if let Some(parent) = policy_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(policy_path, policy).map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct AgentRegistryFile {
    agent: Vec<AgentDef>,
}

fn load_agents(agents_dir: &Path) -> Result<Vec<AgentDef>, String> {
    let registry = agents_dir.join("registry.toml");
    let s = fs::read_to_string(&registry)
        .map_err(|e| format!("read {}: {}", registry.display(), e))?;
    let file: AgentRegistryFile = toml::from_str(&s).map_err(|e| e.to_string())?;
    let mut out = file.agent;
    for a in &mut out {
        if a.command_extension.is_empty() {
            a.command_extension = "md".to_string();
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn repo_root_from_pack(pack_root: &Path) -> PathBuf {
    pack_root
        .parent()
        .unwrap_or(pack_root)
        .to_path_buf()
}

fn skills_workflows_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("skills/rgctl/workflows")
}

fn read_workflow_fragment(workflows_dir: &Path, id: &str) -> Result<String, String> {
    let path = workflows_dir.join(format!("{id}.md"));
    fs::read_to_string(&path).map_err(|e| format!("read {}: {}", path.display(), e))
}

fn workflow_body(workflows_dir: &Path, id: &str, footer: &str) -> Result<String, String> {
    let main = read_workflow_fragment(workflows_dir, id)?;
    Ok(format!("{main}\n\n{footer}"))
}

/// Assemble the meta-skill `references/workflows.md` from workflow fragments.
pub fn assemble_workflows_reference(
    workflows: &[WorkflowEntry],
    workflows_dir: &Path,
) -> Result<String, String> {
    let mut out = String::from(
        "# Workflow Scenarios\n\n\
Worked NL scenarios showing the discover → query → reason → act pattern for common tasks.\n\n\
## Table of Contents\n\n",
    );
    for wf in workflows {
        let anchor = workflow_anchor(&wf.id);
        out.push_str(&format!("- [{}](#{})\n", wf.title, anchor));
    }
    out.push_str("- [Advanced patterns](#advanced-patterns)\n\n---\n\n");
    for wf in workflows {
        let body = read_workflow_fragment(workflows_dir, &wf.id)?;
        out.push_str(&body);
        out.push_str("\n\n---\n\n");
    }
    let advanced = workflows_dir.join("_advanced.md");
    if advanced.is_file() {
        out.push_str(&fs::read_to_string(&advanced).map_err(|e| e.to_string())?);
        out.push_str("\n\n---\n\n");
    }
    let see_also = workflows_dir.join("_see-also.md");
    if see_also.is_file() {
        out.push_str(&fs::read_to_string(&see_also).map_err(|e| e.to_string())?);
        out.push('\n');
    }
    Ok(out)
}

fn workflow_anchor(id: &str) -> String {
    format!("{id}-workflow")
}

fn sync_repo_workflows_reference(
    repo_root: &Path,
    workflows: &[WorkflowEntry],
    workflows_dir: &Path,
) -> Result<(), String> {
    let assembled = assemble_workflows_reference(workflows, workflows_dir)?;
    let ref_path = repo_root.join("skills/rgctl/references/workflows.md");
    let existing = fs::read_to_string(&ref_path).ok();
    if existing.as_deref() != Some(assembled.as_str()) {
        fs::write(&ref_path, assembled).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn render_skill(name: &str, description: &str, body: &str, version: &str) -> String {
    format!(
        r#"---
name: {name}
description: "{description}"
rgctl-managed: true
metadata:
  generatedBy: "rgctl {version}"
---

{body}
"#
    )
}

fn invoke_for(agent: &AgentDef, workflow_id: &str) -> String {
    match agent.command_style.as_str() {
        "colon" => format!("{}:{}", agent.invoke_prefix.trim_start_matches('/'), workflow_id),
        "hyphen" => format!("{}-{}", agent.invoke_prefix, workflow_id),
        "dollar" => format!("{}-{}", agent.invoke_prefix, workflow_id),
        "slash" => format!("{}-{}", agent.invoke_prefix, workflow_id),
        _ => format!("{}-{}", agent.invoke_prefix, workflow_id),
    }
}

fn render_command(agent: &AgentDef, workflow_id: &str, title: &str, version: &str) -> String {
    let invoke = invoke_for(agent, workflow_id);
    let name = if agent.command_style == "colon" {
        format!("/{}", invoke)
    } else {
        invoke.clone()
    };
    format!(
        r#"---
name: {name}
id: rgctl-{workflow_id}
category: Analysis
description: "{title} (rgctl workflow)"
rgctl-managed: true
metadata:
  generatedBy: "rgctl {version}"
---

Run the **rgctl {workflow_id}** workflow. Load skill `rgctl-{workflow_id}` if needed.

User input after the command is natural-language intent; translate to `rgctl -f json` subprocesses.

Invoke: `{invoke}`
"#
    )
}

fn command_ext(agent: &AgentDef) -> &str {
    if agent.command_extension.is_empty() {
        "md"
    } else {
        &agent.command_extension
    }
}

fn command_rel_path(agent: &AgentDef, workflow_id: &str) -> String {
    let ext = command_ext(agent);
    let file = if agent.command_style == "colon" {
        format!("{workflow_id}.{ext}")
    } else {
        format!("rgctl-{workflow_id}.{ext}")
    };
    format!("{}/{}", agent.commands_subdir, file)
}

/// Embed directory name (avoid `.cursor`/`.claude` gitignore collisions on case-insensitive FS).
fn embed_agent_dir(agent_id: &str) -> String {
    format!("host-{agent_id}")
}

fn agent_out_root(out_dir: &Path, agent_id: &str) -> PathBuf {
    out_dir.join("agents").join(embed_agent_dir(agent_id))
}

fn write_agent_file(
    out_dir: &Path,
    agent: &AgentDef,
    rel: &str,
    contents: String,
) -> Result<(), String> {
    let dest = agent_out_root(out_dir, &agent.id).join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&dest, contents).map_err(|e| e.to_string())?;
    Ok(())
}

/// Copy `skills/rgctl` for install, excluding build-only `workflows/` and generated `references/workflows.md`.
fn copy_meta_skill_tree(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for ent in walkdir::WalkDir::new(src) {
        let ent = ent.map_err(|e| e.to_string())?;
        let rel = ent
            .path()
            .strip_prefix(src)
            .map_err(|e| e.to_string())?;
        if rel.components().next().is_some_and(|c| c.as_os_str() == "workflows") {
            continue;
        }
        if rel.as_os_str() == "references/workflows.md" {
            continue;
        }
        let target = dest.join(rel);
        if ent.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = target.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            fs::copy(ent.path(), &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn prepend_router_note(
    skill_path: &Path,
    agent: &AgentDef,
    workflows: &[WorkflowEntry],
    version: &str,
) -> Result<(), String> {
    let existing = fs::read_to_string(skill_path).map_err(|e| e.to_string())?;
    let mut lines = vec![
        "## Workflow slash commands (generated)".to_string(),
        String::new(),
        "| Intent | Command |".to_string(),
        "|--------|---------|".to_string(),
    ];
    for wf in workflows {
        let inv = invoke_for(agent, &wf.id);
        lines.push(format!("| {} | `{}` |", wf.title, inv));
    }
    lines.push(String::new());
    lines.push("**Migrate** (roadmap / `migration_plan.json`) and **Kantra** (rules / `kantra_findings.json`) are separate workflows — do not conflate.".to_string());
    lines.push(format!("rgctl-managed router note generatedBy rgctl {version}"));
    lines.push(String::new());
    let note = lines.join("\n");
    if !existing.contains("Workflow slash commands (generated)") {
        let updated = format!("{existing}\n\n{note}");
        fs::write(skill_path, updated).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod codegen_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn workflows_reference_matches_fragments() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let pack = repo.join("agent-pack");
        let manifest_yaml =
            fs::read_to_string(pack.join("manifest.yaml")).expect("manifest");
        let root: RootManifest = serde_yaml::from_str(&manifest_yaml).expect("yaml");
        let workflows_dir = skills_workflows_dir(&repo);
        let assembled =
            assemble_workflows_reference(&root.workflows, &workflows_dir).expect("assemble");
        let on_disk = repo.join("skills/rgctl/references/workflows.md");
        let existing = fs::read_to_string(&on_disk).expect("workflows.md");
        assert_eq!(
            assembled,
            existing,
            "skills/rgctl/references/workflows.md is stale; run `cargo build`"
        );
    }

    #[test]
    fn generate_is_deterministic_for_gql_cursor() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let pack = repo.join("agent-pack");
        let a = repo.join("target/agent-pack-test-a");
        let b = repo.join("target/agent-pack-test-b");
        generate(&pack, &a, "test").expect("gen a");
        generate(&pack, &b, "test").expect("gen b");
        let fa = a.join("agents/host-cursor/skills/rgctl-gql/SKILL.md");
        let fb = b.join("agents/host-cursor/skills/rgctl-gql/SKILL.md");
        assert_eq!(
            fs::read(&fa).expect("read a"),
            fs::read(&fb).expect("read b")
        );
    }
}

fn render_policy(version: &str) -> String {
    format!(
        r#"---
description: Prefer rgctl for structural codebase questions when .rgctl/ exists
globs:
alwaysApply: true
rgctl-managed: true
metadata:
  generatedBy: "rgctl {version}"
---

# rgctl structural policy (best-effort)

Agent hosts do **not** guarantee blocking grep or read tools. This rule biases behavior only.

When the repository contains `.rgctl/` (after `discover`):

- **MUST** use `rgctl -f json` first for **structural** questions: callers/callees, blast radius, communities, data flow, migration **roadmap** (`migrate` workflow), Konveyor **Kantra** violations (`kantra` workflow), semantic/graph search.
- **MAY** use ripgrep/grep for **lexical** search (fixed strings, comments, logs) and to open files **after** rgctl cites paths.
- **MUST NOT** use bulk grep-as-call-graph (e.g. searching for `foo(` to find callers) when rgctl can answer.

Parse `schema_version` from stdout; never redirect stderr to `/dev/null`.
"#
    )
}

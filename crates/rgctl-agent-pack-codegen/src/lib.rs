//! Generate the embedded agent pack from `skills/rgctl/` and agent adapters.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Agent adapter definition (`agent-pack/agents/*.toml`).
#[derive(Debug, Clone, Deserialize)]
pub struct AgentDef {
    pub id: String,
    pub agent_dir: String,
    pub skills_subdir: String,
    pub invoke_prefix: String,
    pub supports_global: bool,
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
    pub skills_path: String,
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

    let agents = load_agents(&pack_root.join("agents"))?;
    let meta_src = pack_root
        .parent()
        .unwrap_or(pack_root)
        .join("skills/rgctl");
    if !meta_src.join("references/workflows.md").is_file() {
        return Err("skills/rgctl/references/workflows.md missing".into());
    }

    for agent in &agents {
        // Single skill `rgctl`: copy `skills/rgctl` (including references/workflows.md).
        if meta_src.is_dir() {
            let meta_dest = agent_out_root(out_dir, &agent.id)
                .join(&agent.skills_subdir)
                .join("rgctl");
            copy_meta_skill_tree(&meta_src, &meta_dest)?;
            ensure_rgctl_managed_frontmatter(&meta_dest.join("SKILL.md"), rgctl_version)?;
        }
    }

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
                skills_path: format!("{}/{}/", a.agent_dir, a.skills_subdir),
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
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Embed directory name (avoid `.cursor`/`.claude` gitignore collisions on case-insensitive FS).
fn embed_agent_dir(agent_id: &str) -> String {
    format!("host-{agent_id}")
}

fn agent_out_root(out_dir: &Path, agent_id: &str) -> PathBuf {
    out_dir.join("agents").join(embed_agent_dir(agent_id))
}

/// Copy `skills/rgctl` for install. Skip a leftover `workflows/` dir if present.
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

/// Ensure SKILL.md frontmatter notes this file is rgctl-managed (for install --force).
fn ensure_rgctl_managed_frontmatter(skill_path: &Path, version: &str) -> Result<(), String> {
    let existing = fs::read_to_string(skill_path).map_err(|e| e.to_string())?;
    if existing.contains("rgctl-managed: true") {
        return Ok(());
    }
    if existing.starts_with("---\n") {
        let rest = &existing[4..];
        if let Some(end) = rest.find("\n---\n") {
            let front = &rest[..end];
            let body = &rest[end + 5..];
            let updated = format!(
                "---\n{front}\nrgctl-managed: true\nmetadata:\n  generatedBy: \"rgctl {version}\"\n---\n{body}"
            );
            fs::write(skill_path, updated).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    let updated = format!(
        "---\nname: rgctl\nrgctl-managed: true\nmetadata:\n  generatedBy: \"rgctl {version}\"\n---\n\n{existing}"
    );
    fs::write(skill_path, updated).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod codegen_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn generate_copies_workflows_reference() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let pack = repo.join("agent-pack");
        let out = repo.join("target/agent-pack-test-workflows-ref");
        generate(&pack, &out, "test").expect("gen");
        let src = fs::read(repo.join("skills/rgctl/references/workflows.md")).expect("src");
        let dest = fs::read(
            out.join("agents/host-cursor/skills/rgctl/references/workflows.md"),
        )
        .expect("dest");
        assert_eq!(src, dest);
        assert!(!out
            .join("agents/host-cursor/skills/rgctl/workflows")
            .exists());
    }

    #[test]
    fn generate_emits_only_rgctl_skill_for_cursor() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let pack = repo.join("agent-pack");
        let out = repo.join("target/agent-pack-test-single");
        generate(&pack, &out, "test").expect("gen");
        let skills = out.join("agents/host-cursor/skills");
        assert!(skills.join("rgctl/SKILL.md").is_file());
        assert!(skills.join("rgctl/references/workflows.md").is_file());
        assert!(!skills.join("rgctl-search").exists());
        assert!(!skills.join("rgctl-discover").exists());
        assert!(!skills.join("rgctl-migrate").exists());
        // Deterministic: regenerate and compare
        let out2 = repo.join("target/agent-pack-test-single-b");
        generate(&pack, &out2, "test").expect("gen b");
        assert_eq!(
            fs::read(skills.join("rgctl/SKILL.md")).expect("a"),
            fs::read(out2.join("agents/host-cursor/skills/rgctl/SKILL.md")).expect("b")
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

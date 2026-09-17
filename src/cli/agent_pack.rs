//! Embedded agent pack manifest and path helpers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Parsed embedded `manifest.json`.
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

static PACK_ZIP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/agent_pack.zip"));

static PACK_FILES: OnceLock<HashMap<PathBuf, Vec<u8>>> = OnceLock::new();

fn pack_files() -> &'static HashMap<PathBuf, Vec<u8>> {
    PACK_FILES.get_or_init(|| {
        let mut map = HashMap::new();
        let cursor = Cursor::new(PACK_ZIP);
        let mut archive = zip::ZipArchive::new(cursor).expect("agent_pack.zip");
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).expect("zip entry");
            let name = PathBuf::from(file.name());
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut buf).expect("read zip entry");
            map.insert(name, buf);
        }
        map
    })
}

pub fn load_manifest() -> Result<PackManifest, String> {
    let bytes = pack_files()
        .get(Path::new("manifest.json"))
        .ok_or("embedded agent pack missing manifest.json")?;
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}

/// One installable artifact from the embedded pack.
#[derive(Debug, Clone)]
pub struct PackArtifact {
    pub agent_id: String,
    pub workflow: Option<String>,
    pub kind: PackArtifactKind,
    pub repo_rel: PathBuf,
    pub bundle_rel: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackArtifactKind {
    Skill,
    Command,
    Policy,
    Meta,
}

fn agent_entry<'a>(
    manifest: &'a PackManifest,
    agent_id: &str,
) -> Result<&'a AgentManifestEntry, String> {
    manifest
        .agents
        .iter()
        .find(|a| a.id == agent_id)
        .ok_or_else(|| format!("agent {agent_id} missing from manifest"))
}

/// Collect artifacts to install for the given agents and flags.
pub fn collect_artifacts(
    manifest: &PackManifest,
    agent_ids: &[String],
    skills: bool,
    with_commands: bool,
    with_policy: bool,
) -> Result<Vec<PackArtifact>, String> {
    let mut out = Vec::new();
    let mut seen_dest: HashMap<PathBuf, ()> = HashMap::new();

    for agent_id in agent_ids {
        let entry = agent_entry(manifest, agent_id)?;
        let embed_prefix = format!("agents/host-{}/", agent_id);

        if skills {
            let skills_prefix = format!("{}{}/", embed_prefix, entry.skills_subdir);
            collect_skills(
                &skills_prefix,
                embed_prefix.as_str(),
                agent_id,
                &entry.agent_dir,
                &entry.skills_subdir,
                &mut out,
                &mut seen_dest,
            )?;
        }

        if with_commands {
            if let Some(cmd_sub) = &entry.commands_subdir {
                let cmd_prefix = format!("{}{}/", embed_prefix, cmd_sub);
                collect_commands(
                    &cmd_prefix,
                    embed_prefix.as_str(),
                    agent_id,
                    &entry.agent_dir,
                    cmd_sub,
                    &mut out,
                    &mut seen_dest,
                )?;
            }
        }
    }

    if with_policy {
        let policy_bundle = PathBuf::from("policy/rgctl-structural.mdc");
        let policy_dest = PathBuf::from(".cursor/rules/rgctl-structural.mdc");
        if pack_files().contains_key(&policy_bundle) && !seen_dest.contains_key(&policy_dest) {
            seen_dest.insert(policy_dest.clone(), ());
            out.push(PackArtifact {
                agent_id: "cursor".to_string(),
                workflow: None,
                kind: PackArtifactKind::Policy,
                repo_rel: policy_dest,
                bundle_rel: policy_bundle,
            });
        }
    }

    Ok(out)
}

fn collect_skills(
    skills_prefix: &str,
    embed_prefix: &str,
    agent_id: &str,
    agent_dir: &str,
    skills_subdir: &str,
    out: &mut Vec<PackArtifact>,
    seen_dest: &mut HashMap<PathBuf, ()>,
) -> Result<(), String> {
    for (path, _) in pack_files() {
        let path = path.to_string_lossy();
        if !path.starts_with(skills_prefix) {
            continue;
        }
        let after_embed = path.strip_prefix(embed_prefix).unwrap_or(&path);
        let repo_rel = PathBuf::from(agent_dir).join(after_embed);
        if seen_dest.contains_key(&repo_rel) {
            continue;
        }
        seen_dest.insert(repo_rel.clone(), ());
        let workflow = workflow_from_skill_path(after_embed, skills_subdir);
        let kind = if after_embed.contains("/rgctl/") {
            PackArtifactKind::Meta
        } else {
            PackArtifactKind::Skill
        };
        out.push(PackArtifact {
            agent_id: agent_id.to_string(),
            workflow,
            kind,
            repo_rel,
            bundle_rel: PathBuf::from(path.as_ref()),
        });
    }
    Ok(())
}

fn collect_commands(
    cmd_prefix: &str,
    embed_prefix: &str,
    agent_id: &str,
    agent_dir: &str,
    commands_subdir: &str,
    out: &mut Vec<PackArtifact>,
    seen_dest: &mut HashMap<PathBuf, ()>,
) -> Result<(), String> {
    for (path, _) in pack_files() {
        let path = path.to_string_lossy();
        if !path.starts_with(cmd_prefix) {
            continue;
        }
        let after_embed = path.strip_prefix(embed_prefix).unwrap_or(&path);
        let repo_rel = PathBuf::from(agent_dir).join(after_embed);
        if seen_dest.contains_key(&repo_rel) {
            continue;
        }
        seen_dest.insert(repo_rel.clone(), ());
        let workflow = workflow_from_command_path(after_embed, commands_subdir);
        out.push(PackArtifact {
            agent_id: agent_id.to_string(),
            workflow,
            kind: PackArtifactKind::Command,
            repo_rel,
            bundle_rel: PathBuf::from(path.as_ref()),
        });
    }
    Ok(())
}

fn workflow_from_skill_path(after_embed: &str, skills_subdir: &str) -> Option<String> {
    let prefix = format!("{skills_subdir}/");
    let rest = after_embed.strip_prefix(&prefix)?;
    let name = rest.split('/').next()?;
    name.strip_prefix("rgctl-").map(str::to_string)
}

fn workflow_from_command_path(after_embed: &str, commands_subdir: &str) -> Option<String> {
    let prefix = format!("{commands_subdir}/");
    let file = after_embed.strip_prefix(&prefix)?;
    let file = Path::new(file).file_name()?.to_string_lossy();
    let stem = file
        .strip_prefix("rgctl-")
        .unwrap_or(file.as_ref());
    for suffix in [".prompt.md", ".prompt", ".toml", ".md"] {
        if let Some(s) = stem.strip_suffix(suffix) {
            return Some(s.to_string());
        }
    }
    if commands_subdir.contains("/rgctl") {
        return Some(stem.to_string());
    }
    None
}

pub fn bundle_bytes(bundle_rel: &Path) -> Option<&[u8]> {
    pack_files()
        .get(bundle_rel)
        .map(|v| v.as_slice())
}

pub fn default_agent_ids(manifest: &PackManifest) -> Vec<String> {
    manifest.agents.iter().map(|a| a.id.clone()).collect()
}

pub fn resolve_tools(
    manifest: &PackManifest,
    tools: Option<Vec<String>>,
    host: Option<super::args::SkillHost>,
) -> Vec<String> {
    if let Some(list) = tools {
        let ids = normalize_tool_ids(manifest, list);
        if ids.is_empty() {
            return default_agent_ids(manifest);
        }
        return ids;
    }
    if let Some(h) = host {
        return match h {
            super::args::SkillHost::All => default_agent_ids(manifest),
            super::args::SkillHost::Claude => vec!["claude".to_string()],
            super::args::SkillHost::Codex => vec!["codex".to_string()],
            super::args::SkillHost::Cursor => vec!["cursor".to_string()],
        };
    }
    default_agent_ids(manifest)
}

/// Expand `all` and drop unknown ids.
pub fn normalize_tool_ids(manifest: &PackManifest, tools: Vec<String>) -> Vec<String> {
    let known: HashMap<&str, ()> = manifest.agents.iter().map(|a| (a.id.as_str(), ())).collect();
    if tools.iter().any(|t| t == "all") {
        return default_agent_ids(manifest);
    }
    tools
        .into_iter()
        .filter(|t| known.contains_key(t.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_pack_lists_cursor_skills() {
        let n = pack_files()
            .keys()
            .filter(|p| p.to_string_lossy().starts_with("agents/host-cursor/skills/"))
            .count();
        assert!(n > 5, "expected cursor skills in zip, got {n}");
    }

    #[test]
    fn embedded_pack_includes_opencode_and_pi() {
        let m = load_manifest().expect("manifest");
        assert!(m.agents.iter().any(|a| a.id == "opencode"));
        assert!(m.agents.iter().any(|a| a.id == "pi"));
        assert!(
            pack_files()
                .keys()
                .any(|p| p.to_string_lossy().contains("host-opencode/"))
        );
        assert!(
            pack_files()
                .keys()
                .any(|p| p.to_string_lossy().contains("host-pi/"))
        );
    }
}

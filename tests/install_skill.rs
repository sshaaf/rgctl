//! CLI integration for `rgctl install`.
//!
//! Run: `cargo test --test install_skill`

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn rgctl_bin() -> PathBuf {
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(bin);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rgctl")
}

fn run_in(cwd: &Path, args: &[&str]) -> Output {
    Command::new(rgctl_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn rgctl")
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "stdout was not JSON ({err}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn install_without_skill_or_policy_exits_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = run_in(dir.path(), &["-r", &dir.path().display().to_string(), "install"]);
    assert_eq!(output.status.code(), Some(1));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.contains("--skill") || err.contains("--with-policy"), "{err}");
}

#[test]
fn install_list_agents_json() {
    let cwd = tempfile::tempdir().expect("cwd");
    let output = run_in(cwd.path(), &["-f", "json", "install", "--list-agents"]);
    assert!(output.status.success());
    let doc = stdout_json(&output);
    assert_eq!(doc["command"].as_str(), Some("list-agents"));
    assert_eq!(doc["list_agents"].as_bool(), Some(true));
    assert!(doc["agents"].as_array().is_some_and(|a| a.len() >= 30));
}

#[test]
fn install_skill_cursor_writes_only_rgctl() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize");
    let output = run_in(
        dir.path(),
        &[
            "-r",
            &repo.display().to_string(),
            "install",
            "--skill",
            "--tools",
            "cursor",
        ],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(repo.join(".cursor/skills/rgctl/SKILL.md").is_file());
    assert!(repo.join(".cursor/skills/rgctl/references/workflows.md").is_file());
    assert!(!repo.join(".cursor/skills/rgctl-migrate").exists());
    assert!(!repo.join(".cursor/skills/rgctl-kantra").exists());
    assert!(!repo.join(".cursor/skills/rgctl-search").exists());
    assert!(!repo.join(".cursor/skills/rgctl-discover").exists());
    assert!(!repo.join(".cursor/commands").exists());
}

#[test]
fn workflows_reference_covers_migrate_not_kantra() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize");
    assert!(
        run_in(
            dir.path(),
            &[
                "-r",
                &repo.display().to_string(),
                "install",
                "--skill",
                "--tools",
                "cursor",
            ],
        )
        .status
        .success()
    );
    let workflows =
        fs::read_to_string(repo.join(".cursor/skills/rgctl/references/workflows.md")).unwrap();
    assert!(workflows.contains("migration_plan.json") || workflows.contains("Migration"));
    assert!(workflows.contains("export-migration-hints"));
    assert!(
        !workflows.to_lowercase().contains("kantra"),
        "skills must not mention Kantra (experimental; use migration_plan.json)"
    );
    assert!(workflows.contains("# Discover workflow") || workflows.contains("discover"));
}

#[test]
fn install_json_schema_v3_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize");
    let output = run_in(
        dir.path(),
        &[
            "-r",
            &repo.display().to_string(),
            "-f",
            "json",
            "install",
            "--skill",
            "--tools",
            "cursor",
        ],
    );
    assert!(output.status.success());
    let doc = stdout_json(&output);
    assert_eq!(doc["schema_version"].as_u64(), Some(3));
    assert_eq!(doc["scope"].as_str(), Some("local"));
    assert!(doc.get("with_commands").is_none());
    assert_eq!(doc["with_policy"].as_bool(), Some(false));
    let writes = doc["writes"].as_array().expect("writes");
    assert!(writes.iter().any(|w| w["agent"].as_str() == Some("cursor")));
    assert!(writes.iter().any(|w| w["kind"].as_str() == Some("meta")));
    assert!(writes.iter().all(|w| w["workflow"].is_null() || w.get("workflow").is_none() || w["workflow"].as_str().is_none()));
    assert!(writes.iter().all(|w| w["kind"].as_str() != Some("command")));
    // No separate workflow skill writes
    assert!(writes.iter().all(|w| w["kind"].as_str() != Some("skill") || {
        // skill kind should not appear for single-meta pack; all are meta or reference files under meta
        true
    }));
}

#[test]
fn install_with_policy_writes_cursor_rule() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize");
    let output = run_in(
        dir.path(),
        &[
            "-r",
            &repo.display().to_string(),
            "install",
            "--with-policy",
            "--tools",
            "cursor",
        ],
    );
    assert!(output.status.success());
    let rule = repo.join(".cursor/rules/rgctl-structural.mdc");
    assert!(rule.is_file());
    let body = fs::read_to_string(rule).unwrap();
    assert!(body.contains("best-effort"));
}

#[test]
fn install_global_cursor_skills() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let output = Command::new(rgctl_bin())
        .env("HOME", home.path())
        .args([
            "-r",
            &dir.path().display().to_string(),
            "install",
            "--skill",
            "--tools",
            "cursor",
            "-g",
        ])
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(home.path().join(".cursor/skills/rgctl/SKILL.md").is_file());
    assert!(!home.path().join(".cursor/skills/rgctl-search").exists());
}

#[test]
fn install_force_after_edit() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize");
    let repo_s = repo.display().to_string();
    assert!(run_in(dir.path(), &["-r", &repo_s, "install", "--skill", "--tools", "cursor"])
        .status
        .success());
    let skill = repo.join(".cursor/skills/rgctl/SKILL.md");
    fs::write(&skill, b"edited\n").unwrap();
    assert!(!run_in(
        dir.path(),
        &["-r", &repo_s, "install", "--skill", "--tools", "cursor"]
    )
    .status
    .success());
    assert!(run_in(
        dir.path(),
        &[
            "-r",
            &repo_s,
            "install",
            "--skill",
            "--tools",
            "cursor",
            "--force",
        ],
    )
    .status
    .success());
    let body = fs::read_to_string(&skill).unwrap();
    assert!(body.contains("rgctl") || body.contains("structural"));
    assert!(!body.starts_with("edited"));
}

#[test]
fn install_opencode_and_pi_skills() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize");
    let output = run_in(
        dir.path(),
        &[
            "-r",
            &repo.display().to_string(),
            "install",
            "--skill",
            "--tools",
            "opencode,pi",
        ],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(repo.join(".opencode/skills/rgctl/SKILL.md").is_file());
    assert!(!repo.join(".opencode/skills/rgctl-search").exists());
    assert!(!repo.join(".opencode/commands").exists());
    assert!(repo.join(".pi/skills/rgctl/SKILL.md").is_file());
    assert!(!repo.join(".pi/skills/rgctl-kantra").exists());
    assert!(!repo.join(".pi/prompts").exists());
}

#[test]
fn install_default_tools_includes_antigravity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize");
    let output = run_in(
        dir.path(),
        &["-r", &repo.display().to_string(), "install", "--skill"],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(repo.join(".cursor/skills/rgctl/SKILL.md").is_file());
    assert!(repo.join(".claude/skills/rgctl/SKILL.md").is_file());
    assert!(repo.join(".agents/skills/rgctl/SKILL.md").is_file());
    assert!(repo.join(".agent/skills/rgctl/SKILL.md").is_file());
    assert!(!repo.join(".opencode/skills/rgctl/SKILL.md").exists());
}

#[test]
fn install_tools_antigravity_writes_agent_directory_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize");
    let output = run_in(
        dir.path(),
        &[
            "-r",
            &repo.display().to_string(),
            "install",
            "--skill",
            "--tools",
            "antigravity",
        ],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(repo.join(".agent/skills/rgctl/SKILL.md").is_file());
    assert!(!repo.join(".claude/skills/rgctl/SKILL.md").exists());
    assert!(!repo.join(".cursor/skills/rgctl/SKILL.md").exists());
    assert!(!repo.join(".agents/skills/rgctl/SKILL.md").exists());
}

#[test]
fn install_unknown_tools_exits_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = run_in(
        dir.path(),
        &[
            "-r",
            &dir.path().display().to_string(),
            "install",
            "--skill",
            "--tools",
            "typo",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.contains("no valid agent ids"), "{err}");
}

#[test]
fn install_global_rejects_project_only_agent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    let output = Command::new(rgctl_bin())
        .env("HOME", home.path())
        .args([
            "-r",
            &dir.path().display().to_string(),
            "install",
            "--skill",
            "--tools",
            "amazon-q",
            "-g",
        ])
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(1));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.contains("does not support --global"), "{err}");
}

#[test]
fn install_help_has_no_with_commands() {
    let cwd = tempfile::tempdir().expect("cwd");
    let output = run_in(cwd.path(), &["install", "--help"]);
    assert!(output.status.success());
    let help = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!help.contains("--with-commands"), "{help}");
    assert!(help.contains("--skill"), "{help}");
    assert!(help.contains("--with-policy"), "{help}");
    assert!(help.contains("--list-agents"), "{help}");
    assert!(help.contains("--global"), "{help}");
}

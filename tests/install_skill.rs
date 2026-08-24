//! CLI integration for `rg-build install --skill`.
//!
//! Run: `cargo test --test install_skill`

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn rgbuilder_bin() -> PathBuf {
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rg_ctl") {
        return PathBuf::from(bin);
    }
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rg_ctl") {
        return PathBuf::from(bin);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rg_ctl")
}

fn bundled_skill_md() -> Vec<u8> {
    fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skills/rgbuilder/SKILL.md"))
        .expect("read in-tree skills/rgbuilder/SKILL.md")
}

fn skill_path(repo: &Path, host: &str) -> PathBuf {
    repo.join(format!(".{host}/skills/rgbuilder/SKILL.md"))
}

fn run_in(cwd: &Path, args: &[&str]) -> Output {
    Command::new(rgbuilder_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn rg-build")
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
fn install_without_skill_exits_one_and_writes_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cwd = tempfile::tempdir().expect("cwd");
    let output = run_in(
        cwd.path(),
        &["-r", &dir.path().display().to_string(), "install"],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("--skill"),
        "error should mention --skill: {err}"
    );
    assert!(!skill_path(dir.path(), "claude").exists());
    assert!(!skill_path(dir.path(), "cursor").exists());
}

#[test]
fn install_skill_writes_both_hosts_matching_bundle() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize repo");
    let output = run_in(
        dir.path(),
        &["-r", &repo.display().to_string(), "install", "--skill"],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected = bundled_skill_md();
    for host in ["claude", "cursor"] {
        let path = skill_path(&repo, host);
        let got = fs::read(&path).unwrap_or_else(|_| panic!("missing {}", path.display()));
        assert_eq!(got, expected, "{host} SKILL.md must match bundle");
    }
}

#[test]
fn install_host_claude_does_not_create_cursor_and_repo_flag_ignores_cwd() {
    let repo_dir = tempfile::tempdir().expect("repo");
    let cwd_dir = tempfile::tempdir().expect("cwd");
    let repo = fs::canonicalize(repo_dir.path()).expect("canonicalize repo");
    let output = run_in(
        cwd_dir.path(),
        &[
            "-r",
            &repo.display().to_string(),
            "install",
            "--skill",
            "--host",
            "claude",
        ],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(skill_path(&repo, "claude").is_file());
    assert!(!cwd_dir.path().join(".claude").exists());
    assert!(!skill_path(&repo, "cursor").exists());
    assert!(!repo.join(".cursor").exists());
    assert!(!cwd_dir.path().join(".cursor").exists());
}

#[test]
fn install_second_run_unchanged_conflict_then_force() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize repo");
    let repo_s = repo.display().to_string();
    let first = run_in(dir.path(), &["-r", &repo_s, "install", "--skill"]);
    assert!(first.status.success());

    let second = run_in(
        dir.path(),
        &["-r", &repo_s, "-f", "json", "install", "--skill"],
    );
    assert!(
        second.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );
    let doc = stdout_json(&second);
    let writes = doc["writes"].as_array().expect("writes");
    assert!(
        writes
            .iter()
            .all(|w| w["status"].as_str() == Some("unchanged"))
    );

    let claude = skill_path(&repo, "claude");
    fs::write(&claude, b"local edits\n").expect("dirty skill");
    let refused = run_in(
        dir.path(),
        &["-r", &repo_s, "-f", "json", "install", "--skill"],
    );
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(fs::read(&claude).expect("read dirty"), b"local edits\n");
    let refused_doc = stdout_json(&refused);
    let skipped = refused_doc["writes"]
        .as_array()
        .expect("writes")
        .iter()
        .find(|w| w["host"].as_str() == Some("claude"))
        .expect("claude write");
    assert_eq!(skipped["status"].as_str(), Some("skipped_exists"));

    let forced = run_in(
        dir.path(),
        &["-r", &repo_s, "-f", "json", "install", "--skill", "--force"],
    );
    assert!(
        forced.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&forced.stderr)
    );
    assert_eq!(
        fs::read(&claude).expect("read after force"),
        bundled_skill_md()
    );
    let forced_doc = stdout_json(&forced);
    let overwritten = forced_doc["writes"]
        .as_array()
        .expect("writes")
        .iter()
        .find(|w| w["host"].as_str() == Some("claude"))
        .expect("claude write");
    assert_eq!(overwritten["status"].as_str(), Some("overwritten"));
}

#[test]
fn install_json_created_shape() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize repo");
    let output = run_in(
        dir.path(),
        &[
            "-r",
            &repo.display().to_string(),
            "-f",
            "json",
            "install",
            "--skill",
        ],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doc = stdout_json(&output);
    assert_eq!(doc["schema_version"].as_u64(), Some(1));
    assert_eq!(doc["command"].as_str(), Some("install"));
    assert_eq!(doc["skill"].as_str(), Some("rgbuilder"));
    assert_eq!(doc["force"].as_bool(), Some(false));
    let repo_json = doc["repo"].as_str().expect("repo");
    assert!(
        Path::new(repo_json).is_absolute(),
        "repo should be absolute: {repo_json}"
    );
    let writes = doc["writes"].as_array().expect("writes");
    assert_eq!(writes.len(), 2);
    let mut hosts: Vec<_> = writes
        .iter()
        .map(|w| w["host"].as_str().unwrap_or_default())
        .collect();
    hosts.sort_unstable();
    assert_eq!(hosts, ["claude", "cursor"]);
    for write in writes {
        assert_eq!(write["status"].as_str(), Some("created"));
        let path = write["path"].as_str().expect("path");
        assert!(Path::new(path).is_absolute());
        assert!(Path::new(path).ends_with("SKILL.md"));
    }
}

#[test]
fn install_help_mentions_flags() {
    let cwd = tempfile::tempdir().expect("cwd");
    let output = run_in(cwd.path(), &["install", "--help"]);
    assert!(output.status.success());
    let help = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(help.contains("--skill"), "{help}");
    assert!(help.contains("--host"), "{help}");
    assert!(help.contains("--force"), "{help}");
}

#[test]
fn install_uses_embedded_bundle_when_cwd_has_no_skills_tree() {
    let repo_dir = tempfile::tempdir().expect("repo");
    let cwd_dir = tempfile::tempdir().expect("cwd without skills");
    let repo = fs::canonicalize(repo_dir.path()).expect("canonicalize repo");
    assert!(!cwd_dir.path().join("skills/rgbuilder").exists());
    let output = run_in(
        cwd_dir.path(),
        &["-r", &repo.display().to_string(), "install", "--skill"],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(skill_path(&repo, "claude")).expect("claude skill"),
        bundled_skill_md()
    );
}

#[cfg(unix)]
#[test]
fn install_replaces_symlink_with_regular_file() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize repo");
    let dest_dir = repo.join(".claude/skills/rgbuilder");
    fs::create_dir_all(&dest_dir).expect("mkdir dest");
    let sidecar = repo.join("sidecar.md");
    fs::write(&sidecar, bundled_skill_md()).expect("sidecar");
    let dest = dest_dir.join("SKILL.md");
    symlink(&sidecar, &dest).expect("symlink");
    assert!(
        dest.symlink_metadata()
            .expect("meta")
            .file_type()
            .is_symlink()
    );

    let output = run_in(
        dir.path(),
        &[
            "-r",
            &repo.display().to_string(),
            "install",
            "--skill",
            "--host",
            "claude",
        ],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let meta = dest.symlink_metadata().expect("meta after");
    assert!(meta.file_type().is_file());
    assert!(!meta.file_type().is_symlink());
    assert_eq!(fs::read(&dest).expect("read dest"), bundled_skill_md());
}

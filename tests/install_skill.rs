//! CLI integration for `rgctl install --skill`.
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

fn bundled_skill_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("skills/rgctl")
}

fn collect_bundled_files() -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(base: &Path, rel: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for ent in fs::read_dir(base.join(rel)).unwrap_or_else(|err| {
            panic!("read bundled skill dir {}: {err}", base.join(rel).display())
        }) {
            let ent = ent.expect("dir entry");
            let rel_path = rel.join(ent.file_name());
            if ent.path().is_dir() {
                walk(base, &rel_path, out);
            } else {
                let bytes = fs::read(base.join(&rel_path))
                    .unwrap_or_else(|err| panic!("read {}: {err}", rel_path.display()));
                out.push((rel_path, bytes));
            }
        }
    }
    let root = bundled_skill_root();
    let mut out = Vec::new();
    walk(&root, Path::new(""), &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn bundled_file_count() -> usize {
    collect_bundled_files().len()
}

fn skill_dest(repo: &Path, host: &str, rel: &Path) -> PathBuf {
    repo.join(format!(".{host}/skills/rgctl")).join(rel)
}

fn assert_host_matches_bundle(repo: &Path, host: &str) {
    let files = collect_bundled_files();
    assert!(
        !files.is_empty(),
        "expected non-empty skills/rgctl bundle in tree"
    );
    for (rel, expected) in files {
        let path = skill_dest(repo, host, &rel);
        assert!(
            path.is_file(),
            "missing bundled file for {host}: {}",
            path.display()
        );
        let got = fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        assert_eq!(
            got, expected,
            "{host} {} must match bundle",
            rel.display()
        );
    }
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
    assert!(!dir.path().join(".claude/skills/rgctl").exists());
    assert!(!dir.path().join(".cursor/skills/rgctl").exists());
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
    for host in ["claude", "cursor"] {
        assert_host_matches_bundle(&repo, host);
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
    assert_host_matches_bundle(&repo, "claude");
    assert!(!cwd_dir.path().join(".claude").exists());
    assert!(!repo.join(".cursor/skills/rgctl").exists());
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
    assert_eq!(writes.len(), bundled_file_count() * 2);
    assert!(
        writes
            .iter()
            .all(|w| w["status"].as_str() == Some("unchanged"))
    );

    let claude_skill = skill_dest(&repo, "claude", Path::new("SKILL.md"));
    fs::write(&claude_skill, b"local edits\n").expect("dirty skill");
    let refused = run_in(
        dir.path(),
        &["-r", &repo_s, "-f", "json", "install", "--skill"],
    );
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(
        fs::read(&claude_skill).expect("read dirty"),
        b"local edits\n"
    );
    let refused_doc = stdout_json(&refused);
    let skipped = refused_doc["writes"]
        .as_array()
        .expect("writes")
        .iter()
        .find(|w| {
            w["host"].as_str() == Some("claude")
                && w["path"]
                    .as_str()
                    .is_some_and(|p| p.ends_with("SKILL.md"))
        })
        .expect("claude SKILL.md write");
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
    assert_host_matches_bundle(&repo, "claude");
    let forced_doc = stdout_json(&forced);
    let overwritten = forced_doc["writes"]
        .as_array()
        .expect("writes")
        .iter()
        .find(|w| {
            w["host"].as_str() == Some("claude")
                && w["path"]
                    .as_str()
                    .is_some_and(|p| p.ends_with("SKILL.md"))
        })
        .expect("claude SKILL.md write");
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
    assert_eq!(doc["skill"].as_str(), Some("rgctl"));
    assert_eq!(doc["force"].as_bool(), Some(false));
    let repo_json = doc["repo"].as_str().expect("repo");
    assert!(
        Path::new(repo_json).is_absolute(),
        "repo should be absolute: {repo_json}"
    );

    let bundled = collect_bundled_files();
    let per_host = bundled.len();
    assert!(per_host >= 2, "bundle should include SKILL.md and references");

    let writes = doc["writes"].as_array().expect("writes");
    assert_eq!(writes.len(), per_host * 2);

    for host in ["claude", "cursor"] {
        let host_writes: Vec<_> = writes
            .iter()
            .filter(|w| w["host"].as_str() == Some(host))
            .collect();
        assert_eq!(host_writes.len(), per_host, "{host} write count");
        for (rel, _) in &bundled {
            let found = host_writes.iter().any(|w| {
                w["status"].as_str() == Some("created")
                    && w["path"]
                        .as_str()
                        .is_some_and(|p| Path::new(p).ends_with(rel))
            });
            assert!(found, "{host} missing created write for {}", rel.display());
        }
    }

    assert!(
        skill_dest(&repo, "claude", Path::new("references/gql-reference.md")).is_file(),
        "references/ should be installed"
    );
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
    assert!(!cwd_dir.path().join("skills/rgctl").exists());
    let output = run_in(
        cwd_dir.path(),
        &["-r", &repo.display().to_string(), "install", "--skill"],
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_host_matches_bundle(&repo, "claude");
}

#[cfg(unix)]
#[test]
fn install_replaces_symlink_with_regular_file() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = fs::canonicalize(dir.path()).expect("canonicalize repo");
    let dest_dir = repo.join(".claude/skills/rgctl");
    fs::create_dir_all(&dest_dir).expect("mkdir dest");
    let sidecar = repo.join("sidecar.md");
    let skill_md = bundled_skill_root().join("SKILL.md");
    fs::write(&sidecar, fs::read(&skill_md).expect("bundle SKILL.md")).expect("sidecar");
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
    assert_eq!(
        fs::read(&dest).expect("read dest"),
        fs::read(&skill_md).expect("bundle")
    );
}

//! Shared helpers for rgctl integration tests.

#![allow(dead_code)]

use serde_json::Value;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

pub fn rgctl() -> PathBuf {
    if let Some(p) = option_env!("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(p);
    }
    if let Ok(out) = Command::new("sh")
        .args(["-c", "command -v rgctl"])
        .output()
    {
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && !p.is_empty() && Path::new(&p).is_file() {
            return PathBuf::from(p);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in ["target/release/rgctl", "target/debug/rgctl"] {
        let p = manifest.join(rel);
        if p.is_file() {
            return p;
        }
    }
    manifest.join("target/debug/rgctl")
}

pub fn fixture_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo")
}

pub fn linux_repo_path() -> PathBuf {
    std::env::var("RGCTL_LINUX_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("example/linux"))
}

pub fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for ent in fs::read_dir(src).unwrap() {
        let ent = ent.unwrap();
        let name = ent.file_name();
        if name == ".rgctl" || name == ".rbuilder" {
            continue;
        }
        let from = ent.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}

/// Copy tiny_polyglot into a temp dir; returns `(guard, repo_path)`.
pub fn materialize_fixture() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    copy_tree(&fixture_src(), &repo);
    let _ = fs::remove_dir_all(repo.join(".rgctl"));
    let _ = fs::remove_dir_all(repo.join(".rbuilder"));
    (tmp, repo)
}

pub fn assert_ok(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed status={:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Optional cwd so `discover .` indexes the intended repo (not the test runner cwd).
pub fn apply_test_isolation(cmd: &mut Command, repo: Option<&Path>) {
    if let Some(repo) = repo {
        cmd.current_dir(repo);
    }
}

/// Run rgctl with cwd = `repo`.
pub fn run_in_repo(repo: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(rgctl());
    apply_test_isolation(&mut cmd, Some(repo));
    cmd.args(args);
    cmd.output().expect("spawn rgctl")
}

pub fn run_in_repo_json(repo: &Path, args: &[&str]) -> Output {
    let mut full = vec!["-f", "json"];
    full.extend_from_slice(args);
    run_in_repo(repo, &full)
}

pub fn cli_json(repo: &Path, args: &[&str]) -> Value {
    let output = run_in_repo_json(repo, args);
    assert_ok(&output, &format!("cli {}", args.join(" ")));
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "CLI JSON parse failed ({err}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

pub fn discover_fixture(repo: &Path) {
    assert_ok(
        &run_in_repo(repo, &["discover", ".", "--languages", "java,rust"]),
        "discover",
    );
}

pub fn assert_no_rgctl_under(path: &Path) {
    assert!(
        !path.join(".rgctl").exists(),
        "unexpected .rgctl under {}",
        path.display()
    );
}

pub fn assert_rgctl_snapshot(repo: &Path) {
    assert!(
        repo.join(".rgctl/graph.snapshot.bin").is_file(),
        "expected snapshot under {}",
        repo.join(".rgctl").display()
    );
}

pub fn remove_rgctl(repo: &Path) {
    let rb = repo.join(".rgctl");
    if rb.exists() {
        fs::remove_dir_all(&rb).expect("remove .rgctl");
    }
}

pub fn wait_http(url: &str, timeout: Duration) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if client
            .get(url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

pub fn reserve_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

// Back-compat aliases while tests are updated.
pub fn run_no_daemon_in_repo(repo: &Path, args: &[&str]) -> Output {
    run_in_repo(repo, args)
}

pub fn run_no_daemon_json(repo: &Path, args: &[&str]) -> Output {
    run_in_repo_json(repo, args)
}

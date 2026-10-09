//! Integration tests for `rgctl review paths` and `review check` / `pr-check` alias.

use rgctl_graph::backend::GraphBackend;
use rgctl_graph::schema::{Edge, EdgeType, Node, NodeType};
use rgctl_graph::write_columnar_from_nodes_edges;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn rgctl_bin() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_rgctl")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rgctl")
        })
}

fn init_git_repo(dir: &Path) {
    for args in [
        ["init", "-b", "main"],
        ["config", "user.email", "test@example.com"],
        ["config", "user.name", "test"],
    ] {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(out.status.success(), "git {:?} failed", args);
    }
}

fn git_commit_all(dir: &Path, message: &str) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false", "commit", "-m", message])
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write_rewired_snapshot(path: &Path, head: bool) {
    let handle = Node::new(NodeType::Function, "CheckoutController.handle")
        .with_file_path("src/Checkout.java");
    let submit = Node::new(NodeType::Function, "submitOrder").with_file_path("src/Order.java");
    let leaf = if head {
        Node::new(NodeType::Function, "authorizeThenCapture").with_file_path("src/Pay.java")
    } else {
        Node::new(NodeType::Function, "chargeCard").with_file_path("src/Pay.java")
    };
    let h = handle.id;
    let s = submit.id;
    let l = leaf.id;
    let mut backend = rgctl_graph::backend::MemoryBackend::new();
    backend.insert_node(handle).unwrap();
    backend.insert_node(submit).unwrap();
    backend.insert_node(leaf).unwrap();
    backend
        .insert_edge(Edge::new(h, s, EdgeType::Calls))
        .unwrap();
    backend
        .insert_edge(Edge::new(s, l, EdgeType::Calls))
        .unwrap();
    write_columnar_from_nodes_edges(
        backend.all_nodes().unwrap(),
        backend.all_edges().unwrap(),
        path,
    )
    .unwrap();
}

fn setup_rewired_repo() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    init_git_repo(repo);
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/Order.java"), "void submitOrder() {}\n").unwrap();
    fs::create_dir_all(repo.join(".rgctl")).unwrap();
    write_rewired_snapshot(&repo.join(".rgctl/graph.snapshot.bin"), false);
    git_commit_all(repo, "base");

    fs::write(
        repo.join("src/Order.java"),
        "void submitOrder() { /* rewired */ }\n",
    )
    .unwrap();
    write_rewired_snapshot(&repo.join(".rgctl/graph.snapshot.bin"), true);
    git_commit_all(repo, "head");

    fs::create_dir_all(repo.join(".rgctl-base/.rgctl")).unwrap();
    write_rewired_snapshot(
        &repo.join(".rgctl-base/.rgctl/graph.snapshot.bin"),
        false,
    );
    tmp
}

#[test]
fn review_paths_json_rewired_callee() {
    let tmp = setup_rewired_repo();
    let repo = tmp.path();
    let out = Command::new(rgctl_bin())
        .current_dir(repo)
        .args([
            "-r",
            repo.to_str().unwrap(),
            "-f",
            "json",
            "review",
            "paths",
            "--full-snapshots",
            "--base-ref",
            "HEAD~1",
            "--head-ref",
            "HEAD",
        ])
        .output()
        .expect("spawn review paths");
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["command"], "review paths");
    let symbols = v["symbols"].as_array().expect("symbols");
    assert!(!symbols.is_empty());
    let submit = symbols
        .iter()
        .find(|s| s["name"] == "submitOrder")
        .expect("submitOrder");
    let delta = submit["path_delta"].as_array().unwrap();
    assert!(
        delta.iter().any(|d| {
            d["kind"] == "retargeted"
                && d["to_before"] == "chargeCard"
                && d["to_after"] == "authorizeThenCapture"
        }),
        "delta={delta:?}"
    );
}

fn write_target_snapshot(path: &Path, caller_count: usize) {
    let mut backend = rgctl_graph::backend::MemoryBackend::new();
    let target = Node::new(NodeType::Function, "target_fn").with_file_path("src/pkg.rs");
    let target_id = target.id;
    backend.insert_node(target).unwrap();
    for i in 0..caller_count {
        let caller =
            Node::new(NodeType::Function, format!("caller{i}")).with_file_path("src/pkg.rs");
        let caller_id = caller.id;
        backend.insert_node(caller).unwrap();
        backend
            .insert_edge(Edge::new(caller_id, target_id, EdgeType::Calls))
            .unwrap();
    }
    write_columnar_from_nodes_edges(
        backend.all_nodes().unwrap(),
        backend.all_edges().unwrap(),
        path,
    )
    .unwrap();
}

fn setup_pr_policy_repo() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    init_git_repo(repo);
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(repo.join("src/pkg.rs"), "fn target_fn() {}\n").unwrap();
    fs::create_dir_all(repo.join(".rgctl")).unwrap();
    write_target_snapshot(&repo.join(".rgctl/graph.snapshot.bin"), 2);
    git_commit_all(repo, "base");
    fs::write(repo.join("src/pkg.rs"), "fn target_fn() { /* c */ }\n").unwrap();
    write_target_snapshot(&repo.join(".rgctl/graph.snapshot.bin"), 2);
    git_commit_all(repo, "head");
    fs::create_dir_all(repo.join(".rgctl-base/.rgctl")).unwrap();
    write_target_snapshot(&repo.join(".rgctl-base/.rgctl/graph.snapshot.bin"), 2);

    let policy = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/rgctl-pr-policy.json");
    (tmp, policy)
}

#[test]
fn pr_check_and_review_check_equivalent_json() {
    let (tmp, policy) = setup_pr_policy_repo();
    let repo = tmp.path();
    let run = |cmd: &[&str]| {
        Command::new(rgctl_bin())
            .current_dir(repo)
            .args(["-r", repo.to_str().unwrap(), "-f", "json"])
            .args(cmd)
            .arg("--policy-file")
            .arg(&policy)
            .arg("--full-snapshots")
            .arg("--base-ref")
            .arg("HEAD~1")
            .arg("--head-ref")
            .arg("HEAD")
            .output()
            .expect("spawn")
    };
    let a = run(&["pr-check"]);
    let b = run(&["review", "check"]);
    assert_eq!(a.status.code(), b.status.code());
    let va: Value = serde_json::from_slice(&a.stdout).expect("pr-check json");
    let vb: Value = serde_json::from_slice(&b.stdout).expect("review check json");
    assert_eq!(va["schema_version"], vb["schema_version"]);
    assert_eq!(va["passed"], vb["passed"]);
    assert_eq!(va["violations_summary"], vb["violations_summary"]);
}

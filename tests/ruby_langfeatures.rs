//! Ruby extraction GQL gates on `rgctl-tests/ecommerce-ruby`.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-ruby")
}

fn bin() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/rgctl")
}

fn ensure_discovered() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let repo = repo();
        assert!(repo.is_dir(), "missing fixture {}", repo.display());
        let _ = std::fs::remove_dir_all(repo.join(".rgctl"));
        let out = Command::new(bin())
            .args(["discover", ".", "-l", "ruby"])
            .current_dir(&repo)
            .output()
            .expect("run discover");
        assert!(
            out.status.success(),
            "discover failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

fn gql(repo: &Path, query: &str) -> Value {
    let out = Command::new(bin())
        .args(["-f", "json", "gql", query])
        .current_dir(repo)
        .output()
        .expect("gql");
    assert!(
        out.status.success(),
        "gql failed: {}\n{}",
        query,
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json")
}

fn node_count(repo: &Path, label: &str) -> usize {
    let q = format!("MATCH (n:{label}) RETURN n LIMIT 10000");
    gql(repo, &q)
        .get("count")
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as usize
}

fn edge_count(repo: &Path, rel: &str) -> usize {
    let q = format!("MATCH (a)-[:{rel}]->(b) RETURN a,b LIMIT 10000");
    gql(repo, &q)
        .get("count")
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as usize
}

#[test]
fn ruby_ecommerce_import_nonzero() {
    ensure_discovered();
    let n = node_count(&repo(), "Import");
    assert!(n > 0, "expected Import nodes, got {n}");
}

#[test]
fn ruby_ecommerce_calls_nonzero() {
    ensure_discovered();
    let n = edge_count(&repo(), "CALLS");
    assert!(n > 0, "expected Calls edges, got {n}");
}

#[test]
fn ruby_ecommerce_extends_nonzero() {
    ensure_discovered();
    let n = edge_count(&repo(), "EXTENDS");
    assert!(n > 0, "expected Extends mixin edges, got {n}");
}

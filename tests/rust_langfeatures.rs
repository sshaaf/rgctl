//! Rust extraction-depth GQL gates on `rgctl-tests/ecommerce-rust`.
//!
//! ```bash
//! cargo build --release -p rgctl
//! cargo test --test rust_langfeatures -- --nocapture
//! ```

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-rust")
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
            .args(["discover", ".", "-l", "rust"])
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

fn edge_count(repo: &Path, rel: &str) -> usize {
    let q = format!("MATCH (a)-[:{rel}]->(b) RETURN a,b LIMIT 10000");
    gql(repo, &q)
        .get("count")
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as usize
}

fn import_count(repo: &Path) -> usize {
    gql(repo, "MATCH (n:Import) RETURN n LIMIT 10000")
        .get("count")
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as usize
}

#[test]
fn rust_ecommerce_import_graph_nonzero() {
    ensure_discovered();
    let imports = import_count(&repo());
    assert!(
        imports > 0,
        "expected Import nodes on ecommerce-rust, got {imports}"
    );
}

#[test]
fn rust_ecommerce_implements_nonzero() {
    ensure_discovered();
    let n = edge_count(&repo(), "IMPLEMENTS");
    assert!(n > 0, "expected Implements edges on ecommerce-rust, got {n}");
}

#[test]
fn rust_ecommerce_annotated_with_nonzero() {
    ensure_discovered();
    let n = edge_count(&repo(), "ANNOTATEDWITH");
    assert!(n > 0, "expected AnnotatedWith edges on ecommerce-rust, got {n}");
}

#[test]
fn rust_ecommerce_instantiates_nonzero() {
    ensure_discovered();
    let n = edge_count(&repo(), "INSTANTIATES");
    assert!(n > 0, "expected Instantiates edges on ecommerce-rust, got {n}");
}

#[test]
fn rust_ecommerce_calls_nonzero() {
    ensure_discovered();
    let n = edge_count(&repo(), "CALLS");
    assert!(n > 50, "expected substantial CALLS on ecommerce-rust, got {n}");
}

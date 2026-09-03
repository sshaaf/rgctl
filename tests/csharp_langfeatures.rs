//! C# extraction-depth GQL gates on `rgctl-tests/ecommerce-csharp`.
//!
//! ```bash
//! cargo build --release -p rgctl
//! cargo test --test csharp_langfeatures -- --nocapture
//! ```

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-csharp")
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
            .args(["discover", ".", "-l", "csharp"])
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

#[test]
fn csharp_ecommerce_annotated_with_nonzero() {
    ensure_discovered();
    let n = edge_count(&repo(), "ANNOTATEDWITH");
    assert!(n > 0, "expected AnnotatedWith edges on ecommerce-csharp, got {n}");
}

#[test]
fn csharp_ecommerce_instantiates_nonzero() {
    ensure_discovered();
    let n = edge_count(&repo(), "INSTANTIATES");
    assert!(n > 0, "expected Instantiates edges on ecommerce-csharp, got {n}");
}

#[test]
fn csharp_ecommerce_calls_gate() {
    ensure_discovered();
    let n = edge_count(&repo(), "CALLS");
    assert!(n >= 60, "expected >= 60 CALLS on ecommerce-csharp, got {n}");
}

#[test]
fn csharp_ecommerce_namespace_fqn() {
    ensure_discovered();
    let q = r#"MATCH (n:Function) WHERE n.qualified_name LIKE 'Ecommerce.*' RETURN n LIMIT 20"#;
    let result = gql(&repo(), q);
    let count = result.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    assert!(
        count > 0,
        "expected functions with Ecommerce namespace FQN, got {count}"
    );
}

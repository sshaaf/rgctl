//! C extraction-depth GQL gates on `rgctl-tests/ecommerce-c`.
//!
//! ```bash
//! cargo build --release -p rgctl
//! cargo test --test c_langfeatures -- --nocapture
//! ```

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-c")
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
            .args(["discover", ".", "-l", "c"])
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
fn c_ecommerce_calls_at_least_40() {
    ensure_discovered();
    let n = edge_count(&repo(), "CALLS");
    assert!(
        n >= 40,
        "expected >= 40 CALLS on ecommerce-c (baseline ~25), got {n}"
    );
}

#[test]
fn c_ecommerce_import_graph_normalized() {
    ensure_discovered();
    let data = gql(
        &repo(),
        "MATCH (n:Import) WHERE n.file_path LIKE '*.c' RETURN n LIMIT 10",
    );
    let count = data.get("count").and_then(|c| c.as_u64()).unwrap_or(0);
    assert!(count > 0, "expected Import nodes from .c files, got {count}");
    let rows = data.get("rows").and_then(|r| r.as_array()).cloned().unwrap_or_default();
    for row in rows {
        let name = row
            .get("name")
            .or_else(|| row.get("properties"))
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        assert!(
            !name.contains('#') && !name.contains('<') && !name.contains('>'),
            "include path should be normalized, got {name:?}"
        );
    }
}

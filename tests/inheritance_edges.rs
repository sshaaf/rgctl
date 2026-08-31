//! Inheritance edge correctness (EXTENDS / IMPLEMENTS / PERMITS) on ecommerce fixtures.
//!
//! GQL examples (after discover):
//!   MATCH (a:Class)-[:EXTENDS]->(b) WHERE a.name = 'JwtAuthenticationFilter' RETURN a,b
//!   MATCH (a)-[:IMPLEMENTS]->(b) WHERE a.name = 'CustomUserDetailsService' RETURN a,b
//!   MATCH (a:Class)-[:EXTENDS]->(b) WHERE a.name = 'ResourceNotFoundException' RETURN a,b

mod rgctl_harness;

use rgctl_harness::rgctl;
use std::process::Command;

fn rgctl_bin() -> std::path::PathBuf {
    std::env::var("RGCTL_BIN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| rgctl())
}

fn discover(repo: &std::path::Path) {
    let out = Command::new(rgctl_bin())
        .args([
            "discover",
            ".",
            "-q",
            "--languages",
            "java",
        ])
        .current_dir(repo)
        .output()
        .expect("discover");
    assert!(
        out.status.success(),
        "discover failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn gql(repo: &std::path::Path, query: &str) -> serde_json::Value {
    let out = Command::new(rgctl_bin())
        .args(["-r", &repo.to_string_lossy(), "-f", "json", "gql", query])
        .output()
        .expect("gql");
    assert!(
        out.status.success(),
        "gql failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("gql json")
}

fn row_count(v: &serde_json::Value) -> usize {
    v.get("count").and_then(|c| c.as_u64()).unwrap_or(0) as usize
}

#[test]
fn java_class_extends_external_filter() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-java");
    if !repo.is_dir() {
        return;
    }
    discover(&repo);
    let v = gql(
        &repo,
        "MATCH (a:Class)-[:EXTENDS]->(b) WHERE a.name = 'JwtAuthenticationFilter' RETURN a,b",
    );
    assert!(
        row_count(&v) >= 1,
        "expected JwtAuthenticationFilter EXTENDS OncePerRequestFilter, got {v}"
    );
}

#[test]
fn java_class_extends_lang_exception_without_import() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-java");
    if !repo.is_dir() {
        return;
    }
    discover(&repo);
    let v = gql(
        &repo,
        "MATCH (a:Class)-[:EXTENDS]->(b) WHERE a.name = 'ResourceNotFoundException' RETURN a,b",
    );
    assert!(
        row_count(&v) >= 1,
        "expected ResourceNotFoundException EXTENDS RuntimeException, got {v}"
    );
}

#[test]
fn java_class_implements_external_interface() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-java");
    if !repo.is_dir() {
        return;
    }
    discover(&repo);
    let v = gql(
        &repo,
        "MATCH (a:Class)-[:IMPLEMENTS]->(b) WHERE a.name = 'CustomUserDetailsService' RETURN a,b",
    );
    assert!(
        row_count(&v) >= 1,
        "expected CustomUserDetailsService IMPLEMENTS UserDetailsService, got {v}"
    );
}

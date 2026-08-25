//! Java language-feature GQL gates (remainder Instantiates / AnnotatedWith / JPMS / lambda).
//!
//! Fixture: `tests/fixtures/java/langfeatures`
//!
//! ```bash
//! cargo build --release -p rgctl
//! cargo test --test java_langfeatures -- --nocapture
//! ```

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/java/langfeatures")
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
            .args(["discover", ".", "-l", "java", "--with-cfg"])
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

fn row_count(v: &Value) -> usize {
    v.get("rows")
        .and_then(|r| r.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
}

fn binding_props(v: &Value, binding: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let Some(rows) = v.get("rows").and_then(|r| r.as_array()) else {
        return out;
    };
    for row in rows {
        let cells = row.as_array().cloned().unwrap_or_default();
        for cell in cells {
            if cell.get("binding").and_then(|b| b.as_str()) == Some(binding) {
                if let Some(props) = cell.get("properties") {
                    out.push(props.clone());
                }
            }
        }
    }
    out
}

fn target_names(v: &Value, binding: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Some(rows) = v.get("rows").and_then(|r| r.as_array()) else {
        return out;
    };
    for row in rows {
        let cells = row.as_array().cloned().unwrap_or_default();
        for cell in cells {
            if cell.get("binding").and_then(|b| b.as_str()) == Some(binding) {
                if let Some(n) = cell.get("node").and_then(|n| n.as_str()) {
                    out.push(n.to_string());
                }
            }
        }
    }
    out
}

fn binding_qualified_names(v: &Value, binding: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Some(rows) = v.get("rows").and_then(|r| r.as_array()) else {
        return out;
    };
    for row in rows {
        let cells = row.as_array().cloned().unwrap_or_default();
        for cell in cells {
            if cell.get("binding").and_then(|b| b.as_str()) == Some(binding) {
                if let Some(qn) = cell.get("qualified_name").and_then(|n| n.as_str()) {
                    out.push(qn.to_string());
                }
            }
        }
    }
    out
}

#[test]
fn jf01_instantiates_string() {
    let repo = repo();
    ensure_discovered();
    let v = gql(
        &repo,
        "MATCH (a:Function)-[:INSTANTIATES]->(b) WHERE a.name = 'instantiates' RETURN a,b",
    );
    assert!(
        row_count(&v) >= 1,
        "expected Instantiates from instantiates, got {v}"
    );
    let names = target_names(&v, "b");
    assert!(
        names.iter().any(|n| n == "String"),
        "expected String stub/target, got {names:?}"
    );
}

#[test]
fn jf02_type_use_annotated_with() {
    let repo = repo();
    ensure_discovered();
    let v = gql(
        &repo,
        "MATCH (a:Function)-[:ANNOTATED_WITH]->(b) WHERE a.name = 'typeUse' RETURN a,b",
    );
    assert!(
        row_count(&v) >= 1,
        "expected AnnotatedWith from typeUse, got {v}"
    );
    let names = target_names(&v, "b");
    assert!(
        names.iter().any(|n| n == "NonNull"),
        "expected NonNull, got {names:?}"
    );
}

#[test]
fn jf03_field_and_class_literal_references() {
    let repo = repo();
    ensure_discovered();
    let v = gql(
        &repo,
        "MATCH (a:Function)-[:REFERENCES]->(b) WHERE a.name = 'fieldAndClassLiteral' RETURN a,b",
    );
    assert!(
        row_count(&v) >= 1,
        "expected References from fieldAndClassLiteral, got {v}"
    );
}

#[test]
fn jf04_module_depends_on() {
    let repo = repo();
    ensure_discovered();
    let v = gql(&repo, "MATCH (m:Module)-[:DEPENDSON]->(t) RETURN m,t");
    assert!(
        row_count(&v) >= 1,
        "expected Module DependsOn edge, got {v}"
    );
    let names = target_names(&v, "t");
    assert!(
        names.iter().any(|n| n.contains("java.base") || n == "base"),
        "expected java.base stub, got {names:?}"
    );
    let modules = target_names(&v, "m");
    assert!(
        modules
            .iter()
            .any(|n| n.contains("langfeatures") || n.contains("demo")),
        "expected demo.langfeatures module, got {modules:?}"
    );
}

#[test]
fn jf05_lambda_is_lambda_property() {
    let repo = repo();
    ensure_discovered();
    let v = gql(
        &repo,
        "MATCH (f:Function) WHERE f.is_lambda = 'true' RETURN f LIMIT 20",
    );
    assert!(
        row_count(&v) >= 1,
        "expected is_lambda Function via WHERE, got {v}"
    );
    let props = binding_props(&v, "f");
    assert!(
        props
            .iter()
            .any(|p| p.get("is_lambda").and_then(|x| x.as_str()) == Some("true")),
        "expected projected is_lambda property, got {props:?} in {v}"
    );
}

#[test]
fn jf06_generic_throws_properties() {
    let repo = repo();
    ensure_discovered();
    let v = gql(
        &repo,
        "MATCH (f:Function) WHERE f.name = 'genericThrows' RETURN f",
    );
    assert!(row_count(&v) >= 1, "expected genericThrows, got {v}");
    let props = binding_props(&v, "f");
    assert!(
        props
            .iter()
            .any(|p| p.get("type_params").is_some() || p.get("throws").is_some()),
        "expected type_params and/or throws in projected properties, got {props:?} in {v}"
    );
}

/// Issue #49: Java class FQN queries via `qualified_name` (not `name`).
#[test]
fn jf07_class_fqn_qualified_name() {
    let repo = repo();
    ensure_discovered();

    let wrong = gql(
        &repo,
        "MATCH (n:Class) WHERE n.name = 'demo.LangFeatures' RETURN n",
    );
    assert_eq!(
        row_count(&wrong),
        0,
        "FQN on n.name must be empty (simple name only), got {wrong}"
    );

    let by_simple = gql(
        &repo,
        "MATCH (n:Class) WHERE n.name = 'LangFeatures' RETURN n",
    );
    assert!(
        row_count(&by_simple) >= 1,
        "simple name LangFeatures should match, got {by_simple}"
    );
    let names = target_names(&by_simple, "n");
    assert!(
        names.iter().any(|n| n == "LangFeatures"),
        "expected LangFeatures node name, got {names:?}"
    );
    let qns = binding_qualified_names(&by_simple, "n");
    assert!(
        qns.iter().any(|q| q == "demo.LangFeatures"),
        "JSON row should project qualified_name, got {qns:?} in {by_simple}"
    );

    let by_qn = gql(
        &repo,
        "MATCH (n:Class) WHERE n.qualified_name = 'demo.LangFeatures' RETURN n",
    );
    assert!(
        row_count(&by_qn) >= 1,
        "expected Class via n.qualified_name = 'demo.LangFeatures' (issue #49), got {by_qn}"
    );
    assert!(
        binding_qualified_names(&by_qn, "n")
            .iter()
            .any(|q| q == "demo.LangFeatures"),
        "FQN filter row should include qualified_name, got {by_qn}"
    );

    let by_like = gql(
        &repo,
        "MATCH (n:Class) WHERE n.qualified_name LIKE 'demo.*' RETURN n",
    );
    assert!(
        row_count(&by_like) >= 1,
        "expected LIKE on qualified_name, got {by_like}"
    );
}

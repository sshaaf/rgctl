//! Integration tests for `discover --with-kantra`.

use std::process::Command;
use std::sync::Mutex;

/// Both ecommerce-java tests write `.rgctl/` in the same fixture tree.
static ECOMMERCE_LOCK: Mutex<()> = Mutex::new(());
/// tiny_polyglot_repo tests share one `.rgctl/` artifact directory.
static TINY_REPO_LOCK: Mutex<()> = Mutex::new(());

fn rgctl_bin() -> String {
    std::env::var("RGCTL_BIN").unwrap_or_else(|_| {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("target/debug/rgctl")
            .to_string_lossy()
            .into_owned()
    })
}

fn rules_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kantra-rules")
}

#[test]
fn kantra_rules_indexed_in_graph_gql() {
    let _guard = TINY_REPO_LOCK.lock().unwrap();
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    if !repo.is_dir() {
        return;
    }
    let gql = Command::new(rgctl_bin())
        .args([
            "discover",
            ".",
            "--no-daemon",
            "-l",
            "go",
            "--with-kantra",
            "--kantra-index-only",
        ])
        .current_dir(&repo)
        .output()
        .expect("discover");
    assert!(
        gql.status.success(),
        "discover failed: {}",
        String::from_utf8_lossy(&gql.stderr)
    );
    let gql = Command::new(rgctl_bin())
        .args([
            "--no-daemon",
            "-f",
            "json",
            "gql",
            "MATCH (r:KantraRule) RETURN r",
        ])
        .current_dir(&repo)
        .output()
        .expect("gql");
    assert!(
        gql.status.success(),
        "gql failed: {}",
        String::from_utf8_lossy(&gql.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&gql.stdout).unwrap();
    let rows = doc.get("rows").and_then(|v| v.as_array()).unwrap();
    assert!(
        !rows.is_empty(),
        "expected KantraRule nodes in graph after kantra index"
    );
}

#[test]
fn kantra_embedded_discover_writes_findings_json() {
    let _guard = ECOMMERCE_LOCK.lock().unwrap();
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-java");
    if !repo.is_dir() {
        return;
    }
    let out = Command::new(rgctl_bin())
        .args([
            "discover",
            ".",
            "--no-daemon",
            "--languages",
            "java",
            "--with-kantra",
        ])
        .current_dir(&repo)
        .output()
        .expect("discover");
    assert!(
        out.status.success(),
        "discover failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let findings_path = repo.join(".rgctl/kantra_findings.json");
    assert!(findings_path.is_file(), "kantra_findings.json missing");
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&findings_path).unwrap()).unwrap();
    assert_eq!(doc.get("command").and_then(|v| v.as_str()), Some("kantra_findings"));
    assert!(
        doc.get("catalog_id")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty()),
        "expected catalog_id from embedded catalog"
    );
}

#[test]
fn kantra_rules_and_catalog_mutually_exclusive() {
    let _guard = TINY_REPO_LOCK.lock().unwrap();
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    if !repo.is_dir() {
        return;
    }
    let rules = rules_dir();
    let out = Command::new(rgctl_bin())
        .args([
            "discover",
            ".",
            "--no-daemon",
            "--with-kantra",
            "--kantra-rules",
            rules.to_str().unwrap(),
            "--kantra-catalog",
            rules.to_str().unwrap(),
        ])
        .current_dir(&repo)
        .output()
        .expect("discover");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--kantra-rules") || stderr.contains("--kantra-catalog"));
}

#[test]
fn kantra_discover_writes_findings_json() {
    let _guard = ECOMMERCE_LOCK.lock().unwrap();
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-java");
    if !repo.is_dir() {
        return;
    }
    let rules = rules_dir();
    let out = Command::new(rgctl_bin())
        .args([
            "discover",
            ".",
            "--no-daemon",
            "--languages",
            "java",
            "--with-kantra",
            "--kantra-rules",
            rules.to_str().unwrap(),
        ])
        .current_dir(&repo)
        .output()
        .expect("discover");
    assert!(
        out.status.success(),
        "discover failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let findings_path = repo.join(".rgctl/kantra_findings.json");
    assert!(findings_path.is_file(), "kantra_findings.json missing");
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&findings_path).unwrap()).unwrap();
    assert_eq!(doc.get("command").and_then(|v| v.as_str()), Some("kantra_findings"));
    assert_eq!(doc.get("schema_version").and_then(|v| v.as_u64()), Some(1));
    let violations = doc.get("violations").and_then(|v| v.as_array()).unwrap();
    assert!(!violations.is_empty(), "expected at least one violation");
    assert!(
        violations[0].get("rule_id").is_some()
            && violations[0].get("file").is_some()
            && violations[0].get("line").is_some()
    );
    let skipped = doc.get("skipped_rules").and_then(|v| v.as_array()).unwrap();
    assert!(
        skipped.iter().any(|s| {
            s.get("rule_id")
                .and_then(|v| v.as_str())
                == Some("unsupported-dependency")
        }),
        "expected unsupported rule in skipped_rules"
    );
}

#[test]
fn kantra_markdown_imports_not_matched_by_go_referenced() {
    let _guard = TINY_REPO_LOCK.lock().unwrap();
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    if !repo.is_dir() {
        return;
    }
    let rules = rules_dir();
    let out = Command::new(rgctl_bin())
        .args([
            "discover",
            ".",
            "--no-daemon",
            "-l",
            "markdown,go",
            "--with-kantra",
            "--kantra-rules",
            rules.to_str().unwrap(),
        ])
        .current_dir(&repo)
        .output()
        .expect("discover");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let findings_path = repo.join(".rgctl/kantra_findings.json");
    let doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(findings_path).unwrap()).unwrap();
    let violations = doc.get("violations").and_then(|v| v.as_array()).unwrap();
    assert!(
        !violations.iter().any(|v| {
            v.get("matched_by").and_then(|m| m.as_str()) == Some("go.referenced")
                && v.get("file")
                    .and_then(|f| f.as_str())
                    .is_some_and(|f| f.ends_with(".md"))
        }),
        "markdown imports must not match go.referenced"
    );
}

#[test]
fn kantra_rule_target_property_gql() {
    let _guard = TINY_REPO_LOCK.lock().unwrap();
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    if !repo.is_dir() {
        return;
    }
    let out = Command::new(rgctl_bin())
        .args([
            "discover",
            ".",
            "--no-daemon",
            "-l",
            "go",
            "--with-kantra",
            "--kantra-index-only",
        ])
        .current_dir(&repo)
        .output()
        .expect("discover");
    assert!(
        out.status.success(),
        "discover failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let gql = Command::new(rgctl_bin())
        .args([
            "--no-daemon",
            "-f",
            "json",
            "gql",
            "MATCH (r:KantraRule) WHERE r.`konveyor.io/target` = 'quarkus' RETURN r",
        ])
        .current_dir(&repo)
        .output()
        .expect("gql");
    assert!(
        gql.status.success(),
        "gql failed: {}",
        String::from_utf8_lossy(&gql.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&gql.stdout).unwrap();
    let rows = doc.get("rows").and_then(|v| v.as_array()).unwrap();
    assert!(
        !rows.is_empty(),
        "expected KantraRule nodes with konveyor.io/target=quarkus"
    );
}

//! Go language-feature probes (LF-*) against ecommerce-go `internal/langfeatures`.
//!
//! See `docs/design/go-language-coverage.md` and issue #46.
//!
//! ```bash
//! cargo test --test go_langfeatures -- --nocapture
//! ```

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

fn repo() -> PathBuf {
    if let Ok(p) = std::env::var("RGBUILDER_GO_REPO") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgbuilder-tests/ecommerce-go")
}

fn bin() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_rg_ctl") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/rg_ctl")
}

fn ensure_discovered() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let repo = repo();
        if !repo.is_dir() {
            return;
        }
        let _ = std::fs::remove_dir_all(repo.join(".rgbuilder"));
        let out = Command::new(bin())
            .args(["discover", ".", "-l", "go", "-e", "vendor", "--with-cfg"])
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

fn callees_named(repo: &Path, caller: &str) -> Vec<(String, String)> {
    let q =
        format!("MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = '{caller}' RETURN a,b");
    let v = gql(repo, &q);
    let mut out = Vec::new();
    if let Some(rows) = v.get("rows").and_then(|r| r.as_array()) {
        for row in rows {
            let cells = row.as_array().cloned().unwrap_or_default();
            let mut file = String::new();
            let mut name = String::new();
            for cell in cells {
                if cell.get("binding").and_then(|b| b.as_str()) == Some("b") {
                    name = cell
                        .get("node")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    file = cell
                        .get("file")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
            if !name.is_empty() {
                out.push((name, file));
            }
        }
    }
    out
}

#[test]
fn lf01_package_call_edge() {
    let repo = repo();
    if !repo.is_dir() {
        eprintln!("skip: missing {}", repo.display());
        return;
    }
    ensure_discovered();
    let callees = callees_named(&repo, "LfPkgCaller");
    assert!(
        callees.iter().any(|(n, _)| n == "LfPkgCallee"),
        "LF-01 expected LfPkgCaller → LfPkgCallee, got {callees:?}"
    );
}

#[test]
fn lf02_same_type_method_call() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let callees = callees_named(&repo, "Checkout");
    assert!(
        callees
            .iter()
            .any(|(n, f)| n == "validate" && f.contains("methods.go")),
        "LF-02 expected Checkout → validate in methods.go, got {callees:?}"
    );
}

#[test]
fn lf03_cross_type_same_name_resolves_to_beta() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let callees = callees_named(&repo, "Run");
    assert!(
        callees.iter().any(|(n, _)| n == "ListItems"),
        "LF-03 expected Run → ListItems, got {callees:?}"
    );
    assert!(
        !callees.iter().any(|(n, _)| n == "Run"),
        "LF-03 must not self-loop Run → Run, got {callees:?}"
    );
}

#[test]
fn lf04_interface_method_call() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let callees = callees_named(&repo, "Start");
    assert!(
        callees.iter().any(|(n, _)| n == "RunSandbox"),
        "LF-04 expected Start → RunSandbox, got {callees:?}"
    );
}

#[test]
fn lf07_embed_promoted_method_call() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let callees = callees_named(&repo, "UseBase");
    assert!(
        callees.iter().any(|(n, _)| n == "BaseMethod"),
        "LF-07 expected UseBase → BaseMethod, got {callees:?}"
    );
}

#[test]
fn lf18_constructor_qualified_name() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let out = Command::new(bin())
        .args(["-f", "json", "blast-radius", "NewLfCart", "--depth", "1"])
        .current_dir(&repo)
        .output()
        .expect("blast");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let blast: Value = serde_json::from_slice(&out.stdout).unwrap();
    let fqn = blast["target"]["canonical_fqn"].as_str().unwrap_or("");
    assert!(
        fqn == "LfCart.<init>" || fqn == "LfCart::<init>",
        "LF-18 ctor FQN, got {fqn}"
    );
    let file = blast["target"]["file_path"].as_str().unwrap_or("");
    assert!(
        file.contains("methods.go"),
        "LF-18 expected methods.go, got {file}"
    );
}

fn node_names(v: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(rows) = v.get("rows").and_then(|r| r.as_array()) {
        for row in rows {
            let cells = row.as_array().cloned().unwrap_or_default();
            for cell in cells {
                if let Some(n) = cell.get("node").and_then(|n| n.as_str()) {
                    out.push(n.to_string());
                } else if let Some(n) = cell.as_str() {
                    out.push(n.to_string());
                }
            }
        }
    }
    out
}

#[test]
fn lf05_implements_edges() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let v = gql(
        &repo,
        "MATCH (a:Struct)-[:IMPLEMENTS]->(b:Interface) WHERE a.name = 'LfRemoteRuntime' RETURN a,b",
    );
    let names = node_names(&v);
    assert!(
        names.iter().any(|n| n == "LfRuntime"),
        "LF-05 expected LfRemoteRuntime IMPLEMENTS LfRuntime, got {names:?} raw={v}"
    );
    let v2 = gql(
        &repo,
        "MATCH (a:Struct)-[:IMPLEMENTS]->(b:Interface) WHERE a.name = 'LfFakeRuntime' RETURN a,b",
    );
    let names2 = node_names(&v2);
    assert!(
        names2.iter().any(|n| n == "LfRuntime"),
        "LF-05 expected LfFakeRuntime IMPLEMENTS LfRuntime, got {names2:?}"
    );
}

#[test]
fn lf06_embed_extends() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let v = gql(
        &repo,
        "MATCH (a:Struct)-[:EXTENDS]->(b:Struct) WHERE a.name = 'LfDerived' RETURN a,b",
    );
    let names = node_names(&v);
    assert!(
        names.iter().any(|n| n == "LfBase"),
        "LF-06 expected LfDerived EXTENDS LfBase, got {names:?}"
    );
}

#[test]
fn lf10_const_and_typealias() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let consts = gql(
        &repo,
        "MATCH (n:Variable) WHERE n.name = 'LfStatusPending' RETURN n",
    );
    assert!(
        !node_names(&consts).is_empty(),
        "LF-10 expected Variable LfStatusPending"
    );
    let alias = gql(
        &repo,
        "MATCH (n:TypeAlias) WHERE n.name = 'LfUserID' RETURN n",
    );
    assert!(
        !node_names(&alias).is_empty(),
        "LF-10 expected TypeAlias LfUserID"
    );
}

#[test]
fn lf16_generics_symbols_present() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let id = gql(
        &repo,
        "MATCH (n:Function) WHERE n.name = 'LfIdentity' RETURN n",
    );
    assert!(
        !node_names(&id).is_empty(),
        "LF-16 expected Function LfIdentity"
    );
    let box_t = gql(&repo, "MATCH (n:Struct) WHERE n.name = 'LfBox' RETURN n");
    assert!(
        !node_names(&box_t).is_empty(),
        "LF-16 expected Struct LfBox"
    );
}

#[test]
fn lf17_import_symbols() {
    let repo = repo();
    if !repo.is_dir() {
        return;
    }
    ensure_discovered();
    let fmt = gql(&repo, "MATCH (n:Import) WHERE n.name = 'fmt' RETURN n");
    assert!(!node_names(&fmt).is_empty(), "LF-17 expected Import fmt");
    let tu = gql(&repo, "MATCH (n:Import) WHERE n.name = 'timeutil' RETURN n");
    assert!(
        !node_names(&tu).is_empty(),
        "LF-17 expected Import timeutil"
    );
}

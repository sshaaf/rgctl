//! Semantic search QE — index/query oracles (OpenSpec `qe-sanity-gates`).
//!
//! Distinct from `semantic_audit` / `semantic_boundary` (CFG/PDG). Policy: required red
//! until fixed — see `rgbuilder-tests/correctness/QE.md`.

use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str;

fn fixture_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo")
}

fn markdown_fixture_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/markdown-context")
}

fn oracles_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/semantic_oracles.json")
}

fn rgbuilder_bin() -> PathBuf {
    for key in ["CARGO_BIN_EXE_rg_ctl", "CARGO_BIN_EXE_rg_ctl"] {
        if let Ok(p) = std::env::var(key) {
            return PathBuf::from(p);
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let debug = root.join("target/debug/rg_ctl");
    if debug.is_file() {
        return debug;
    }
    panic!("rg-build binary not found; run via `cargo test` or `cargo build`");
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct OracleFile {
    pin: PinSpec,
    oracles: Vec<Oracle>,
}

#[derive(Debug, Deserialize)]
struct PinSpec {
    model_id_contains: String,
    dimensions: usize,
}

#[derive(Debug, Deserialize)]
struct Oracle {
    id: String,
    query: String,
    k: usize,
    expect_in_top_k: Vec<String>,
    #[serde(default)]
    expand: Option<String>,
    #[serde(default)]
    require_expansion: bool,
    #[serde(default = "required_sev")]
    severity: String,
}

fn required_sev() -> String {
    "required".into()
}

struct Sandbox {
    _dir: tempfile::TempDir,
    repo: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        Self::from_fixture(&fixture_repo())
    }

    fn from_fixture(fixture: &Path) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        copy_dir_all(fixture, dir.path()).expect("copy fixture");
        Self {
            repo: dir.path().to_path_buf(),
            _dir: dir,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut cmd = Command::new(rgbuilder_bin());
        cmd.arg("-r").arg(&self.repo);
        cmd.args(args);
        cmd.output().expect("spawn rg-build")
    }

    fn parse_json(&self, output: &Output) -> Value {
        let stdout = str::from_utf8(&output.stdout).expect("utf8");
        serde_json::from_str(stdout).unwrap_or_else(|e| {
            panic!(
                "JSON parse: {e}\nstdout:\n{stdout}\nstderr:\n{}",
                str::from_utf8(&output.stderr).unwrap_or("")
            )
        })
    }
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed (exit {:?}):\nstderr: {}\nstdout: {}",
        output.status.code(),
        str::from_utf8(&output.stderr).unwrap_or(""),
        str::from_utf8(&output.stdout).unwrap_or("")
    );
}

fn hit_names(doc: &Value) -> Vec<String> {
    doc["hits"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|h| h["name"].as_str().map(|s| s.to_string()))
        .collect()
}

#[test]
fn semantic_index_query_invariants_and_oracles() {
    let raw = fs::read_to_string(oracles_path()).expect("read semantic_oracles.json");
    let spec: OracleFile = serde_json::from_str(&raw).expect("parse oracles");

    let sandbox = Sandbox::new();
    let discover = sandbox.run(&["-f", "json", "discover", ".", "--languages", "java,rust"]);
    assert_success(&discover, "discover");

    let index = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "index",
        "--dimensions",
        &spec.pin.dimensions.to_string(),
    ]);
    assert_success(&index, "semantic index");
    let index_doc = sandbox.parse_json(&index);
    assert_eq!(index_doc["schema_version"].as_u64(), Some(2));
    assert!(
        index_doc["functions_indexed"].as_u64().unwrap_or(0) > 0,
        "functions_indexed must be > 0"
    );
    let model_id = index_doc["model_id"].as_str().unwrap_or("");
    assert!(
        model_id.contains(&spec.pin.model_id_contains),
        "model_id {model_id:?} must contain {:?}",
        spec.pin.model_id_contains
    );
    assert_eq!(
        index_doc["dimensions"].as_u64().unwrap_or(0) as usize,
        spec.pin.dimensions
    );

    let index2 = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "index",
        "--dimensions",
        &spec.pin.dimensions.to_string(),
    ]);
    assert_success(&index2, "semantic index incremental");
    let inc = sandbox.parse_json(&index2);
    let stats = inc["build_stats"].as_object().expect("build_stats");
    let reused = stats["reused"].as_u64().unwrap_or(0);
    let embedded = stats["embedded"].as_u64().unwrap_or(0);
    assert!(
        reused >= embedded,
        "second index must reuse embeddings: reused={reused} embedded={embedded}"
    );

    for oracle in &spec.oracles {
        if oracle.severity != "required" {
            continue;
        }
        let k = oracle.k.to_string();
        let mut args = vec![
            "-f",
            "json",
            "semantic",
            "query",
            oracle.query.as_str(),
            "--limit",
            k.as_str(),
        ];
        if let Some(mode) = &oracle.expand {
            args.push("--expand");
            args.push(mode.as_str());
        }
        let out = sandbox.run(&args);
        assert_success(&out, &format!("semantic query {}", oracle.id));
        let doc = sandbox.parse_json(&out);
        assert_eq!(doc["schema_version"].as_u64(), Some(3));
        assert_eq!(
            doc["dimensions"].as_u64().unwrap_or(0) as usize,
            spec.pin.dimensions
        );
        let names = hit_names(&doc);
        for expected in &oracle.expect_in_top_k {
            assert!(
                names.iter().any(|n| n == expected || n.ends_with(expected)),
                "oracle {}: expected {:?} in top-{} hits {:?}\nfull={}",
                oracle.id,
                expected,
                oracle.k,
                names,
                doc
            );
        }
        if oracle.require_expansion {
            let expansion = doc.get("expansion").expect("expansion object");
            let neighbors = expansion.get("neighbors");
            assert!(
                neighbors.is_some(),
                "oracle {}: expansion.neighbors required, got {expansion}",
                oracle.id
            );
            let arr = neighbors.and_then(|v| v.as_array());
            assert!(
                arr.is_some_and(|a| !a.is_empty()),
                "oracle {}: expansion.neighbors must be non-empty for checkout fixture, got {expansion}",
                oracle.id
            );
        }
    }
}

#[test]
fn semantic_vocab_and_diffuse_find_checkout() {
    let sandbox = Sandbox::new();
    let discover = sandbox.run(&["-f", "json", "discover", ".", "--languages", "java,rust"]);
    assert_success(&discover, "discover");

    let index_hash = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "index",
        "--embedder",
        "hash",
        "--dimensions",
        "256",
    ]);
    assert_success(&index_hash, "semantic index hash");

    let index_vocab = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "index",
        "--embedder",
        "vocab",
        "--dimensions",
        "256",
    ]);
    assert_success(&index_vocab, "semantic index vocab");
    let vocab_doc = sandbox.parse_json(&index_vocab);
    assert!(
        vocab_doc["model_id"]
            .as_str()
            .unwrap_or("")
            .contains("vocab-accumulate"),
        "expected vocab model_id, got {}",
        vocab_doc["model_id"]
    );

    // Distilled vocab-accumulate-v2 Hamming is weaker than FNV v1 identity matching.
    // Product ranking is fusion (default); that is what must surface checkout.
    let q = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "query",
        "checkout order cart",
        "--limit",
        "5",
    ]);
    assert_success(&q, "vocab query");
    let names = hit_names(&sandbox.parse_json(&q));
    assert!(
        names.iter().any(|n| n.contains("checkout")),
        "vocab fusion should rank checkout: {names:?}"
    );

    let index_diffuse = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "index",
        "--embedder",
        "vocab",
        "--diffuse",
        "--dimensions",
        "256",
    ]);
    assert_success(&index_diffuse, "semantic index vocab+diffuse");
    let q2 = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "query",
        "checkout order cart",
        "--limit",
        "5",
    ]);
    assert_success(&q2, "vocab+diffuse query");
    let names2 = hit_names(&sandbox.parse_json(&q2));
    assert!(
        names2.iter().any(|n| n.contains("checkout")),
        "vocab+diffuse should still rank checkout: {names2:?}"
    );
}

#[test]
fn semantic_query_scope_docs_and_function_are_enforced() {
    let sandbox = Sandbox::from_fixture(&markdown_fixture_repo());
    let discover = sandbox.run(&["-f", "json", "discover", ".", "--languages", "markdown"]);
    assert_success(&discover, "discover markdown");

    let docs_index = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "index",
        "--scope",
        "docs",
        "--embedder",
        "vocab",
        "--dimensions",
        "256",
    ]);
    assert_success(&docs_index, "semantic index docs");

    let docs_query = sandbox.run(&[
        "-f", "json", "semantic", "query", "checkout", "--scope", "docs", "--limit", "5",
    ]);
    assert_success(&docs_query, "semantic docs query");
    let docs_hits = hit_names(&sandbox.parse_json(&docs_query));
    assert!(
        !docs_hits.is_empty(),
        "expected docs hits for checkout query"
    );

    let function_query = sandbox.run(&[
        "-f", "json", "semantic", "query", "checkout", "--scope", "function", "--limit", "5",
    ]);
    assert!(
        !function_query.status.success(),
        "function scope query should fail against docs-only index"
    );
    let stderr = str::from_utf8(&function_query.stderr).unwrap_or("");
    assert!(
        stderr.contains("semantic query scope"),
        "expected scope mismatch message, got stderr={stderr}"
    );
}

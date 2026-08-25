//! Map / symbol-resolution collision QE (OpenSpec `qe-sanity-gates`).
//!
//! Policy: required failures stay red until fixed (see `rgctl-tests/correctness/QE.md`).

use rgctl::analysis::resolve_unique_symbol;
use rgctl::graph::backend::{GraphBackend, MemoryBackend};
use rgctl::graph::schema::{Node, NodeType};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/collision_repo")
}

fn rgctl_bin() -> PathBuf {
    option_env!("CARGO_BIN_EXE_rgctl")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/rgctl")
        })
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

struct Sandbox {
    _dir: tempfile::TempDir,
    repo: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        copy_dir_all(&fixture_root(), dir.path()).expect("copy collision fixture");
        Self {
            repo: dir.path().to_path_buf(),
            _dir: dir,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut cmd = Command::new(rgctl_bin());
        cmd.arg("-r").arg(&self.repo);
        cmd.args(args);
        cmd.output().expect("spawn rgctl")
    }

    fn parse_stdout_json(&self, output: &Output) -> Value {
        let stdout = str::from_utf8(&output.stdout).expect("utf8 stdout");
        serde_json::from_str(stdout).unwrap_or_else(|e| {
            panic!(
                "JSON parse failed: {e}\nstdout:\n{stdout}\nstderr:\n{}",
                str::from_utf8(&output.stderr).unwrap_or("")
            )
        })
    }
}

/// Public resolve MUST reject ambiguous bare names (already implemented).
#[test]
fn ambiguous_bare_name_fails_closed() {
    let mut backend = MemoryBackend::new();
    backend
        .insert_node(
            Node::new(NodeType::Function, "collide").with_qualified_name("AmbiguousA::collide"),
        )
        .unwrap();
    backend
        .insert_node(
            Node::new(NodeType::Function, "collide").with_qualified_name("AmbiguousB::collide"),
        )
        .unwrap();

    let err = resolve_unique_symbol(&backend, "collide").unwrap_err();
    assert!(
        err.to_string().contains("ambiguous"),
        "expected ambiguity error, got: {err}"
    );
}

/// After discover, bare `collide` MUST be ambiguous at blast-radius (required).
#[test]
fn discover_fixture_bare_collide_is_ambiguous() {
    let sandbox = Sandbox::new();
    let discover = sandbox.run(&["-f", "json", "discover", ".", "--languages", "java,rust"]);
    assert!(
        discover.status.success(),
        "discover failed:\n{}",
        str::from_utf8(&discover.stderr).unwrap_or("")
    );

    let blast = sandbox.run(&["-f", "json", "blast-radius", "collide"]);
    assert!(
        !blast.status.success(),
        "blast-radius collide must fail closed on ambiguity; stdout={}",
        str::from_utf8(&blast.stdout).unwrap_or("")
    );
    let combined = format!(
        "{}{}",
        str::from_utf8(&blast.stderr).unwrap_or(""),
        str::from_utf8(&blast.stdout).unwrap_or("")
    );
    assert!(
        combined.to_lowercase().contains("ambiguous")
            || combined.to_lowercase().contains("matches"),
        "expected ambiguity messaging, got:\n{combined}"
    );
}

/// Polyglot / multi-file short names: `twin` from two Rust modules must not
/// resolve uniquely via bare blast-radius.
#[test]
fn rust_twin_short_name_is_ambiguous() {
    let sandbox = Sandbox::new();
    let discover = sandbox.run(&["-f", "json", "discover", ".", "--languages", "rust"]);
    assert!(
        discover.status.success(),
        "discover failed:\n{}",
        str::from_utf8(&discover.stderr).unwrap_or("")
    );

    let blast = sandbox.run(&["-f", "json", "blast-radius", "twin"]);
    assert!(
        !blast.status.success(),
        "bare twin must be ambiguous across alpha/beta modules"
    );
}

/// Qualified-index lossiness: two nodes with the same FQN must not collapse to a
/// single definitive qualified lookup. See GraphBuilder unit tests in
/// `rgctl-extraction` — this subprocess check ensures both function nodes survive discover.
#[test]
fn duplicate_bare_names_both_nodes_survive_discover() {
    let sandbox = Sandbox::new();
    let discover = sandbox.run(&["-f", "json", "discover", ".", "--languages", "java"]);
    assert!(discover.status.success());

    let gql = sandbox.run(&[
        "-f",
        "json",
        "gql",
        "MATCH (n:Function) WHERE n.name = 'collide' RETURN n",
    ]);
    assert!(
        gql.status.success(),
        "gql failed:\n{}",
        str::from_utf8(&gql.stderr).unwrap_or("")
    );
    let doc = sandbox.parse_stdout_json(&gql);
    let rows = doc["rows"].as_array().expect("gql rows array");
    assert!(
        rows.len() >= 2,
        "expected ≥2 collide function nodes after discover, got {} ({doc})",
        rows.len()
    );
}

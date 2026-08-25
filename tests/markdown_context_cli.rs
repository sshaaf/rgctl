//! CLI integration for markdown context graph — discover → snapshot → GQL queries 1–6.
//!
//! Spawns `rgctl` against a temp copy of `tests/fixtures/markdown-context`.
//! Run: `cargo test --test markdown_context_cli`

use rgctl_graph::schema::NodeType;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str;

const SNAPSHOT_REL: &str = ".rgctl/graph.snapshot.bin";

fn rgctl_bin() -> PathBuf {
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(bin);
    }
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(bin);
    }
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        let release_candidate = PathBuf::from(&target).join("release/rgctl");
        if release_candidate.is_file() {
            return release_candidate;
        }
        let candidate = PathBuf::from(target).join("debug/rgctl");
        if candidate.is_file() {
            return candidate;
        }
    }
    let release_default = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/rgctl");
    if release_default.is_file() {
        return release_default;
    }
    if let Some(bin) = option_env!("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(bin);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rgctl")
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/markdown-context")
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".rgctl" {
            continue;
        }
        let target = dst.join(name);
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

struct FixtureRepo {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl FixtureRepo {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        copy_dir_all(&fixture_root(), dir.path()).expect("copy markdown-context fixture");
        Self {
            path: dir.path().to_path_buf(),
            _dir: dir,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut cmd = Command::new(rgctl_bin());
        cmd.arg("-r").arg(&self.path);
        cmd.args(args);
        cmd.output().expect("spawn rgctl")
    }

    fn discover_json(&self, languages: &str) -> Value {
        let out = self.run(&["-f", "json", "discover", ".", "-l", languages]);
        assert_success(&out, &format!("discover -l {languages}"));
        parse_stdout_json(&out)
    }

    fn gql_count(&self, query: &str) -> usize {
        let out = self.run(&["-f", "json", "gql", query]);
        assert_success(&out, "gql");
        let doc = parse_stdout_json(&out);
        doc.get("count")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .or_else(|| doc.get("rows").and_then(|r| r.as_array()).map(|a| a.len()))
            .unwrap_or(0)
    }

    fn snapshot_bytes(&self) -> u64 {
        let path = self.path.join(SNAPSHOT_REL);
        assert!(path.is_file(), "missing {}", path.display());
        fs::metadata(&path).expect("metadata").len()
    }
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed:\nstdout={}\nstderr={}",
        str::from_utf8(&output.stdout).unwrap_or(""),
        str::from_utf8(&output.stderr).unwrap_or("")
    );
}

fn parse_stdout_json(output: &Output) -> Value {
    let stdout = str::from_utf8(&output.stdout).expect("stdout utf8");
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "invalid JSON: {e}\nstdout={stdout}\nstderr={}",
            str::from_utf8(&output.stderr).unwrap_or("")
        )
    })
}

fn metrics_nodes(doc: &Value) -> usize {
    doc.get("metrics")
        .and_then(|m| m.get("nodes_generated"))
        .and_then(|v| v.as_u64())
        .expect("nodes_generated") as usize
}

#[test]
fn cli_discover_snapshot_has_section_body() {
    let repo = FixtureRepo::new();
    repo.discover_json("markdown,java");
    let snap = repo.path.join(SNAPSHOT_REL);
    let graph = rgctl_graph::CodeGraph::open_snapshot(&snap).expect("open snapshot");
    let checkout = graph
        .backend()
        .all_nodes()
        .expect("nodes")
        .into_iter()
        .find(|n| {
            n.node_type == NodeType::Module
                && n.name == "Checkout Flow"
                && n.get_property("kind") == Some("heading")
        })
        .expect("Checkout Flow heading node");
    assert!(
        checkout
            .get_property("body_text")
            .is_some_and(|b| b.contains("End-to-end checkout")),
        "snapshot body_text: {:?}, keys: {:?}",
        checkout.get_property("body_text"),
        checkout.properties.keys().collect::<Vec<_>>()
    );
}

#[test]
fn cli_discover_markdown_writes_snapshot() {
    let repo = FixtureRepo::new();
    let doc = repo.discover_json("markdown");
    assert!(
        metrics_nodes(&doc) > 10,
        "expected markdown nodes, got {}",
        metrics_nodes(&doc)
    );
    assert!(repo.snapshot_bytes() > 0);
}

#[test]
fn cli_gql_queries_1_through_6() {
    let repo = FixtureRepo::new();
    repo.discover_json("markdown,java");

    assert!(
        repo.gql_count(
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n"
        ) >= 1,
        "query 1"
    );
    assert!(
        repo.gql_count(
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' AND n.body_text LIKE 'End-to-end*' RETURN n"
        ) >= 1,
        "checkout section body_text"
    );
    assert!(
        repo.gql_count(
            "MATCH (a:Module)-[:CONTAINS]->(b:Module) WHERE a.kind = 'heading' AND b.kind = 'heading' RETURN a, b"
        ) >= 1,
        "query 2"
    );
    assert!(
        repo.gql_count(
            "MATCH (h:Module)-[:REFERENCES]->(f:File) WHERE h.kind = 'heading' AND f.name LIKE '*adr.md' RETURN h, f"
        ) >= 1,
        "query 3"
    );
    assert!(
        repo.gql_count(
            "MATCH (h:Module)-[:REFERENCES]->(t:Module) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND t.kind = 'heading' RETURN h, t"
        ) >= 1,
        "query 4"
    );
    assert!(
        repo.gql_count(
            "MATCH (h:Module)-[:CONTAINS*1..3]->(n:Module) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND n.kind = 'heading' RETURN h, n"
        ) >= 2,
        "query 5"
    );
    assert_eq!(
        repo.gql_count(
            "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' RETURN h, f, c"
        ),
        1,
        "query 6"
    );
}

#[test]
fn cli_footprint_markdown_larger_than_java_only() {
    let repo = FixtureRepo::new();

    let java_doc = repo.discover_json("java");
    let java_nodes = metrics_nodes(&java_doc);
    let java_bytes = repo.snapshot_bytes();

    let md_doc = repo.discover_json("markdown");
    let md_nodes = metrics_nodes(&md_doc);
    let md_bytes = repo.snapshot_bytes();

    assert!(
        md_nodes > java_nodes,
        "markdown nodes ({md_nodes}) should exceed java-only ({java_nodes})"
    );
    assert!(
        md_bytes > java_bytes,
        "markdown snapshot ({md_bytes} B) should exceed java-only ({java_bytes} B)"
    );
    assert_eq!(java_nodes, metrics_nodes(&java_doc));
    assert!(
        repo.gql_count("MATCH (c:Class) WHERE c.name = 'CheckoutService' RETURN c") == 0,
        "markdown-only graph must not contain CheckoutService class"
    );
    // Re-discover combined for sanity
    repo.discover_json("markdown,java");
    assert_eq!(
        repo.gql_count(
            "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' RETURN h, f, c"
        ),
        1
    );
}

#[test]
fn cli_slice_rejects_markdown_file() {
    let repo = FixtureRepo::new();
    repo.discover_json("markdown,java");
    let out = repo.run(&["slice", "docs/guide.md", "--line", "1", "--variable", "x"]);
    assert!(
        !out.status.success(),
        "slice on .md must fail:\n{}",
        str::from_utf8(&out.stderr).unwrap_or("")
    );
    let combined = format!(
        "{}{}",
        str::from_utf8(&out.stderr).unwrap_or(""),
        str::from_utf8(&out.stdout).unwrap_or("")
    );
    assert!(
        combined.contains("slice") && combined.contains("markdown"),
        "expected markup rejection message:\n{combined}"
    );
}

#[test]
fn cli_cpg_flows_rejects_markdown_file() {
    let repo = FixtureRepo::new();
    repo.discover_json("markdown,java");
    let out = repo.run(&[
        "cpg",
        "flows",
        "docs/guide.md",
        "--line",
        "1",
        "--variable",
        "x",
        "--function",
        "main",
    ]);
    assert!(!out.status.success());
    let combined = format!(
        "{}{}",
        str::from_utf8(&out.stderr).unwrap_or(""),
        str::from_utf8(&out.stdout).unwrap_or("")
    );
    assert!(combined.contains("cpg flows") || combined.contains("markdown"));
}

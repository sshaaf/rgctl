//! Multi-query semantic timing — vocab embedder on the polyglot fixture (+ optional linux).
//!
//! Run:
//! ```text
//! cargo test --test semantic_query_timing -- --nocapture
//! RGBUILDER_LINUX_SEMANTIC=1 cargo test --test semantic_query_timing linux -- --nocapture --ignored
//! ```

use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::str;
use std::time::{Duration, Instant};

fn fixture_repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo")
}

fn timing_queries_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/semantic_query_timing.json")
}

fn rgbuilder_bin() -> PathBuf {
    option_env!("CARGO_BIN_EXE_rg_ctl")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/rg_ctl")
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

#[derive(Debug, Deserialize)]
struct TimingSuite {
    embedder: String,
    dimensions: usize,
    queries: Vec<TimedQuery>,
}

#[derive(Debug, Deserialize)]
struct TimedQuery {
    id: String,
    query: String,
    limit: usize,
    #[serde(default)]
    fusion: bool,
    #[serde(default)]
    expect_name_contains: Vec<String>,
}

struct Sandbox {
    _dir: Option<tempfile::TempDir>,
    repo: PathBuf,
}

impl Sandbox {
    fn from_fixture() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        copy_dir_all(&fixture_repo(), dir.path()).expect("copy fixture");
        Self {
            repo: dir.path().to_path_buf(),
            _dir: Some(dir),
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

struct QueryTiming {
    id: String,
    wall: Duration,
    hits: usize,
}

fn load_suite() -> TimingSuite {
    let raw = fs::read_to_string(timing_queries_path()).expect("read semantic_query_timing.json");
    serde_json::from_str(&raw).expect("parse timing suite")
}

fn run_timed_queries(sandbox: &Sandbox, suite: &TimingSuite) -> Vec<QueryTiming> {
    let mut timings = Vec::with_capacity(suite.queries.len());
    for q in &suite.queries {
        let limit = q.limit.to_string();
        let mut args = vec![
            "-f",
            "json",
            "semantic",
            "query",
            q.query.as_str(),
            "--limit",
            limit.as_str(),
        ];
        if !q.fusion {
            args.push("--no-fusion");
        }

        let started = Instant::now();
        let out = sandbox.run(&args);
        let wall = started.elapsed();
        assert_success(&out, &format!("query {}", q.id));
        let doc = sandbox.parse_json(&out);
        let names = hit_names(&doc);
        if !q.expect_name_contains.is_empty() {
            let ok = q.expect_name_contains.iter().any(|expected| {
                names
                    .iter()
                    .any(|n| n == expected || n.contains(expected) || n.ends_with(expected))
            });
            assert!(
                ok,
                "query {}: expected one of {:?} in hits {:?}",
                q.id, q.expect_name_contains, names
            );
        }
        timings.push(QueryTiming {
            id: q.id.clone(),
            wall,
            hits: names.len(),
        });
    }
    timings
}

fn print_timing_table(label: &str, timings: &[QueryTiming]) {
    let total: Duration = timings.iter().map(|t| t.wall).sum();
    let max = timings.iter().map(|t| t.wall).max().unwrap_or_default();
    let min = timings.iter().map(|t| t.wall).min().unwrap_or_default();
    eprintln!("\n=== {label} ({n} queries) ===", n = timings.len());
    eprintln!("{:<22} {:>10} {:>6}", "query_id", "wall_ms", "hits");
    for t in timings {
        eprintln!(
            "{:<22} {:>10.2} {:>6}",
            t.id,
            t.wall.as_secs_f64() * 1000.0,
            t.hits
        );
    }
    eprintln!(
        "total={:.2}ms  min={:.2}ms  max={:.2}ms  mean={:.2}ms",
        total.as_secs_f64() * 1000.0,
        min.as_secs_f64() * 1000.0,
        max.as_secs_f64() * 1000.0,
        (total.as_secs_f64() * 1000.0) / timings.len().max(1) as f64
    );
}

#[test]
fn vocab_multi_query_timing_polyglot() {
    let suite = load_suite();
    assert_eq!(suite.embedder, "vocab");
    assert!(
        suite.queries.len() >= 5,
        "timing suite should include multiple queries"
    );

    let sandbox = Sandbox::from_fixture();
    let discover = sandbox.run(&["-f", "json", "discover", ".", "--languages", "java,rust"]);
    assert_success(&discover, "discover");

    let dims = suite.dimensions.to_string();
    let index = sandbox.run(&[
        "-f",
        "json",
        "semantic",
        "index",
        "--embedder",
        "vocab",
        "--dimensions",
        dims.as_str(),
    ]);
    assert_success(&index, "semantic index vocab");
    let index_doc = sandbox.parse_json(&index);
    assert!(
        index_doc["model_id"]
            .as_str()
            .unwrap_or("")
            .contains("vocab-accumulate"),
        "model_id={}",
        index_doc["model_id"]
    );

    let timings = run_timed_queries(&sandbox, &suite);
    print_timing_table("tiny_polyglot vocab queries", &timings);

    let total: Duration = timings.iter().map(|t| t.wall).sum();
    // CLI process spawn dominates on tiny indexes; keep a loose ceiling for CI flakes.
    assert!(
        total < Duration::from_secs(30),
        "multi-query total too slow: {total:?}"
    );
    assert!(
        timings.iter().all(|t| t.wall < Duration::from_secs(10)),
        "an individual query exceeded 10s: {:?}",
        timings.iter().map(|t| (&t.id, t.wall)).collect::<Vec<_>>()
    );
}

/// Times multiple in-process queries against a pre-built linux vocab index.
///
/// Loads the index once (amortizes mmap), then times Hamming queries only.
///
/// ```text
/// RGBUILDER_LINUX_SEMANTIC=1 cargo test --test semantic_query_timing linux_vocab -- --nocapture --ignored
/// ```
#[test]
#[ignore = "manual: requires example/linux with semantic_index.bin (vocab)"]
fn linux_vocab_multi_query_timing() {
    if std::env::var_os("RGBUILDER_LINUX_SEMANTIC").is_none() {
        eprintln!("set RGBUILDER_LINUX_SEMANTIC=1 to run linux timing");
        return;
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("example/linux");
    let index_path = repo.join(".rgbuilder/semantic_index.bin");
    assert!(
        index_path.is_file(),
        "missing {} — run discover + `semantic index --embedder vocab` first",
        index_path.display()
    );

    let load_start = Instant::now();
    let index = rgbuilder_analysis::SemanticIndex::load(&index_path).expect("load semantic index");
    let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;
    assert!(
        index.model_id.contains("vocab-accumulate"),
        "expected vocab index, got {}",
        index.model_id
    );
    eprintln!(
        "loaded {} rows ({} dims, {}) in {:.1}ms",
        index.len(),
        index.dimensions,
        index.model_id,
        load_ms
    );

    // Kernel-oriented queries (fixture oracles like checkout do not apply here).
    let queries = [
        ("skb_alloc", "allocate sk_buff packet buffer"),
        ("mutex_lock", "mutex lock spinlock acquire"),
        ("irq_handler", "interrupt irq handler"),
        ("dma_map", "dma map page address"),
        ("netdev_tx", "network device transmit packet"),
        ("page_fault", "page fault memory handler"),
    ];

    let reload = rgbuilder_analysis::OnnxReloadOptions::default();
    // Warm embedder + first scan.
    let _ = rgbuilder_analysis::query_index_with_embedder(&index, "warmup", 5, &reload)
        .expect("warmup");

    let mut timings = Vec::new();
    for (id, text) in queries {
        let started = Instant::now();
        let hits = rgbuilder_analysis::query_index_with_embedder(&index, text, 10, &reload)
            .expect("query");
        let wall = started.elapsed();
        timings.push(QueryTiming {
            id: id.to_string(),
            wall,
            hits: hits.len(),
        });
        eprintln!(
            "  {id}: top={:?} wall_ms={:.2}",
            hits.iter()
                .take(3)
                .map(|h| h.entry.name.as_str())
                .collect::<Vec<_>>(),
            wall.as_secs_f64() * 1000.0
        );
    }

    print_timing_table("example/linux in-process vocab Hamming", &timings);
    let mean_ms = timings
        .iter()
        .map(|t| t.wall.as_secs_f64() * 1000.0)
        .sum::<f64>()
        / timings.len().max(1) as f64;
    eprintln!("linux mean wall_ms={mean_ms:.2} (index load excluded: {load_ms:.1}ms)");
    // In-process Hamming over ~1.86M × 32 B should be well under 20s mean.
    assert!(
        mean_ms < 20_000.0,
        "linux mean query wall too high: {mean_ms:.1}ms"
    );
}

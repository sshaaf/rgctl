//! CLI coverage for discover `--with-harmonic` (#29).
//!
//! Default discover skips harmonic (zeros in analysis_results).
//! `--with-harmonic` restores exact/HyperBall scores.

mod dashboard_harness;

use dashboard_harness::{copy_dir_all, rgctl_bin};
use rgctl::analysis::AnalysisResults;
use std::path::Path;
use std::process::Command;

fn run_discover(repo: &Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(rgctl_bin());
    cmd.current_dir(repo)
        .args([
            "-r",
            repo.to_str().unwrap(),
            "discover",
            ".",
            "--languages",
            "java,rust",
        ]);
    cmd.args(extra);
    cmd.output().expect("spawn rgctl discover")
}

fn assert_ok(output: &std::process::Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn max_harmonic(analysis: &AnalysisResults) -> f32 {
    analysis
        .centrality
        .as_ref()
        .map(|t| t.harmonic.iter().copied().fold(0.0f32, f32::max))
        .unwrap_or(0.0)
}

fn max_pagerank(analysis: &AnalysisResults) -> f32 {
    analysis
        .centrality
        .as_ref()
        .map(|t| t.pagerank.iter().copied().fold(0.0f32, f32::max))
        .unwrap_or(0.0)
}

fn load_analysis(repo: &Path) -> AnalysisResults {
    let path = repo.join(".rgctl/analysis_results.bin");
    assert!(path.is_file(), "missing {}", path.display());
    AnalysisResults::load(&path).expect("load analysis_results")
}

#[test]
fn discover_default_skips_harmonic_columns() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    copy_dir_all(&fixture, &repo).expect("copy fixture");
    let _ = std::fs::remove_dir_all(repo.join(".rgctl"));

    let output = run_discover(&repo, &[]);
    assert_ok(&output, "discover default");

    let analysis = load_analysis(&repo);
    assert!(
        analysis.centrality.is_some(),
        "centrality table must still be written"
    );
    assert_eq!(
        max_harmonic(&analysis),
        0.0,
        "default discover must leave harmonic columns at zero (#29)"
    );
    assert!(
        max_pagerank(&analysis) > 0.0,
        "PageRank must still run when harmonic is off"
    );
}

#[test]
fn discover_with_harmonic_fills_nonzero_scores() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    copy_dir_all(&fixture, &repo).expect("copy fixture");
    let _ = std::fs::remove_dir_all(repo.join(".rgctl"));

    let output = run_discover(&repo, &["--with-harmonic"]);
    assert_ok(&output, "discover --with-harmonic");

    let analysis = load_analysis(&repo);
    assert!(
        max_harmonic(&analysis) > 0.0,
        "--with-harmonic must produce at least one positive harmonic score"
    );
    assert!(
        max_pagerank(&analysis) > 0.0,
        "PageRank must still run with --with-harmonic"
    );
}

#[test]
fn discover_help_documents_with_harmonic() {
    let output = Command::new(rgctl_bin())
        .args(["discover", "--help"])
        .output()
        .expect("spawn help");
    assert_ok(&output, "discover --help");
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        help.contains("--with-harmonic"),
        "help must list --with-harmonic:\n{help}"
    );
}

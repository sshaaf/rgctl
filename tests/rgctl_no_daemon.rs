//! Tier A/B integration tests for `--no-daemon` (artifacts in source tree).
//!
//! Tier A runs on `tiny_polyglot_repo` in CI.
//! Tier B (`#[ignore]`) runs cold discover + gql on `example/linux` when present.
//!
//! ```text
//! cargo test --release --test rgctl_no_daemon
//! cargo test --release --test rgctl_no_daemon -- --ignored --nocapture
//! ```

mod rgctl_harness;

use rgctl_harness::{
    assert_no_rgbuilder_under, assert_ok, assert_rgbuilder_snapshot, cli_json, discover_fixture,
    linux_repo_path, materialize_fixture, remove_rgbuilder, rgctl, run_no_daemon_in_repo,
    run_no_daemon_json,
};
use std::process::Command;

/// Sanity floor for linux function inventory after cold discover.
const LINUX_SMOKE_MIN_FUNCTIONS: u64 = 10_000;

#[test]
fn no_daemon_discover_writes_source_tree_snapshot() {
    let (_tmp, repo) = materialize_fixture();
    assert_ok(
        &run_no_daemon_in_repo(&repo, &["discover", "."]),
        "discover",
    );
    assert_rgbuilder_snapshot(&repo);
}

#[test]
fn no_daemon_gql_and_metrics_after_discover() {
    let (_tmp, repo) = materialize_fixture();
    discover_fixture(&repo);

    let query = cli_json(
        &repo,
        &["gql", "MATCH (n:Function) RETURN n LIMIT 3"],
    );
    assert_eq!(query["schema_version"], 1);
    assert!(query["rows"].as_array().map(|r| !r.is_empty()).unwrap_or(false));

    let metrics = cli_json(&repo, &["metrics", "--pagerank"]);
    assert_eq!(metrics["schema_version"], 1);
    assert!(metrics["pagerank"]["top"].is_array());
}

#[test]
fn discover_dot_from_repo_cwd_indexes_that_repo() {
    let (_tmp, repo) = materialize_fixture();
    assert_ok(
        &run_no_daemon_in_repo(&repo, &["discover", "."]),
        "discover from repo cwd",
    );
    assert_rgbuilder_snapshot(&repo);
}

/// Regression: `-r OTHER discover .` from a different cwd indexes cwd, not `-r`.
#[test]
fn discover_dot_ignores_dash_r_when_cwd_differs() {
    let (_tmp, repo) = materialize_fixture();
    let outer = tempfile::tempdir().unwrap();
    let out = Command::new(rgctl())
        .current_dir(outer.path())
        .env("RGCTL_NO_DAEMON", "1")
        .args([
            "--no-daemon",
            "-r",
            repo.to_str().unwrap(),
            "discover",
            ".",
        ])
        .output()
        .unwrap();
    assert_ok(&out, "discover from outer cwd");
    assert!(
        outer.path().join(".rgbuilder/graph.snapshot.bin").is_file(),
        "discover . should index cwd (outer temp), not -r target"
    );
    assert_no_rgbuilder_under(&repo);
}

#[test]
fn discover_absolute_path_works_from_any_cwd() {
    let (_tmp, repo) = materialize_fixture();
    let outer = tempfile::tempdir().unwrap();
    let out = Command::new(rgctl())
        .current_dir(outer.path())
        .env("RGCTL_NO_DAEMON", "1")
        .args([
            "--no-daemon",
            "discover",
            repo.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_ok(&out, "discover absolute path");
    assert_rgbuilder_snapshot(&repo);
    assert_no_rgbuilder_under(outer.path());
}

#[test]
#[ignore = "manual: cold discover + gql on example/linux (~145s on reference machine)"]
fn linux_no_daemon_discover_and_gql_smoke() {
    let repo = linux_repo_path();
    if !repo.is_dir() {
        eprintln!("skip: linux not at {}", repo.display());
        return;
    }

    remove_rgbuilder(&repo);
    let out = run_no_daemon_json(&repo, &["discover", ".", "-v"]);
    assert_ok(&out, "linux discover");
    assert_rgbuilder_snapshot(&repo);

    let query = cli_json(
        &repo,
        &["gql", "--macro-name", "all_functions", "unused"],
    );
    assert_eq!(query["schema_version"], 1);
    let functions = query["count"].as_u64().unwrap_or(0);
    assert!(
        functions > LINUX_SMOKE_MIN_FUNCTIONS,
        "linux smoke: function count {functions} below floor {LINUX_SMOKE_MIN_FUNCTIONS}"
    );

    let _ = cli_json(
        &repo,
        &["gql", "MATCH (n:Function) RETURN n LIMIT 1"],
    );
}

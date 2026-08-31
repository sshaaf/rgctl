//! CLI coverage for discover `--with-dashboard` / `--export-migration-hints` (#31).

mod dashboard_harness;

use dashboard_harness::{copy_dir_all, rgctl_bin};
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

fn materialize() -> (tempfile::TempDir, std::path::PathBuf) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    copy_dir_all(&fixture, &repo).expect("copy fixture");
    // Fixture may ship a stale `.rgctl/`; start clean so default-off is observable.
    let _ = std::fs::remove_dir_all(repo.join(".rgctl"));
    (tmp, repo)
}

#[test]
fn discover_default_skips_dashboard_dir() {
    let (_tmp, repo) = materialize();
    let output = run_discover(&repo, &[]);
    assert_ok(&output, "discover default");

    assert!(
        repo.join(".rgctl/graph.snapshot.bin").is_file(),
        "graph snapshot still required"
    );
    assert!(
        !repo.join(".rgctl/dashboard").exists(),
        "default discover must not write .rgctl/dashboard (#31)"
    );
}

#[test]
fn discover_with_dashboard_writes_bundle() {
    if !rgctl_dashboard::dist_embedded() {
        eprintln!("skip: dashboard/dist not embedded");
        return;
    }

    let (_tmp, repo) = materialize();
    let output = run_discover(&repo, &["--with-dashboard"]);
    assert_ok(&output, "discover --with-dashboard");

    let dash = repo.join(".rgctl/dashboard");
    assert!(dash.join("index.html").is_file(), "missing index.html");
    assert!(
        dash.join("manifest.json").is_file(),
        "missing manifest.json"
    );
}

#[test]
fn export_migration_hints_writes_plan_without_dashboard() {
    let (_tmp, repo) = materialize();
    let plan_path = repo.join("hints.json");
    let output = run_discover(
        &repo,
        &[
            "--export-migration-hints",
            "-o",
            plan_path.to_str().unwrap(),
        ],
    );
    assert_ok(&output, "discover --export-migration-hints");

    assert!(plan_path.is_file(), "migration hints file not written");
    let plan: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&plan_path).unwrap()).unwrap();
    assert_eq!(plan["schema_version"], 2);
    assert!(
        !repo.join(".rgctl/dashboard").exists(),
        "migration hints must not imply dashboard export"
    );
}

#[test]
fn export_migration_plan_alias_still_works() {
    let (_tmp, repo) = materialize();
    let plan_path = repo.join("legacy_alias.json");
    let output = run_discover(
        &repo,
        &["--export-migration-plan", "-o", plan_path.to_str().unwrap()],
    );
    assert_ok(&output, "discover --export-migration-plan alias");
    assert!(plan_path.is_file(), "alias must still write plan JSON");
}

#[test]
fn discover_help_documents_dashboard_flags() {
    let output = Command::new(rgctl_bin())
        .args(["discover", "--help"])
        .output()
        .expect("spawn help");
    assert_ok(&output, "discover --help");
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        help.contains("--with-dashboard"),
        "missing --with-dashboard"
    );
    assert!(
        help.contains("--export-migration-hints"),
        "missing --export-migration-hints"
    );
    assert!(
        help.contains("--export-migration-plan"),
        "alias --export-migration-plan should remain visible"
    );
}

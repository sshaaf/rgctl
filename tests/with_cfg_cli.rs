//! CLI coverage for discover `--with-cfg` / `--with-taint` / `--with-security` (#34).

mod dashboard_harness;

use dashboard_harness::{copy_dir_all, rgbuilder_bin};
use std::path::Path;
use std::process::Command;

fn materialize() -> (tempfile::TempDir, std::path::PathBuf) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    copy_dir_all(&fixture, &repo).expect("copy fixture");
    let _ = std::fs::remove_dir_all(repo.join(".rgbuilder"));
    (tmp, repo)
}

fn run_discover(repo: &Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(rgbuilder_bin());
    cmd.args([
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

#[test]
fn discover_rejects_all_flag() {
    let (_tmp, repo) = materialize();
    let output = run_discover(&repo, &["--all"]);
    assert!(!output.status.success(), "--all must not exist (#34)");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        err.contains("--all") || err.contains("unexpected"),
        "error should mention unknown --all:\n{err}"
    );
}

#[test]
fn discover_with_cfg_writes_archive_without_requiring_taint() {
    let (_tmp, repo) = materialize();
    let output = run_discover(&repo, &["--with-cfg"]);
    assert_ok(&output, "discover --with-cfg");
    assert!(
        repo.join(".rgbuilder/analysis/cfg_pdg.archive.bin")
            .is_file()
            || repo.join(".rgbuilder/analysis").is_dir(),
        "CFG pass should create analysis artifacts"
    );
}

#[test]
fn discover_cfg_alias_still_works() {
    let (_tmp, repo) = materialize();
    let output = run_discover(&repo, &["--cfg"]);
    assert_ok(&output, "discover --cfg alias");
}

#[test]
fn discover_with_dfg_loops_requires_cfg_pass() {
    let (_tmp, repo) = materialize();
    let output = run_discover(&repo, &["--with-dfg-loops", "--with-cfg"]);
    assert_ok(&output, "discover --with-dfg-loops --with-cfg");
}

#[test]
fn discover_with_ast_skeleton_requires_cfg_pass() {
    let (_tmp, repo) = materialize();
    let output = run_discover(&repo, &["--with-ast-skeleton", "--with-cfg"]);
    assert_ok(&output, "discover --with-ast-skeleton --with-cfg");
}

#[test]
fn discover_help_lists_with_flags_not_all() {
    let output = Command::new(rgbuilder_bin())
        .args(["discover", "--help"])
        .output()
        .expect("help");
    assert_ok(&output, "discover --help");
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--with-cfg"));
    assert!(help.contains("--with-taint"));
    assert!(help.contains("--with-security"));
    assert!(
        !help.lines().any(|l| l.trim_start().starts_with("--all")),
        "help must not advertise --all:\n{help}"
    );
}

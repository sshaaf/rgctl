//! CLI integration tests for fragment clone detection (`--mode fragment`).

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn rgctl_bin() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rgctl")
}

fn run_cmd(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let bin = rgctl_bin();
    let out = Command::new(&bin)
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (out.status.success(), stdout, stderr)
}

fn run_json(dir: &Path, args: &[&str]) -> Value {
    let (ok, stdout, stderr) = run_cmd(dir, args);
    assert!(
        ok,
        "rgctl {:?} failed:\nstdout={}\nstderr={}",
        args, stdout, stderr
    );
    let json_start = stdout.find('{').unwrap_or_else(|| {
        panic!("expected JSON object on stdout, got:\n{}", stdout)
    });
    serde_json::from_str(stdout[json_start..].trim()).unwrap_or_else(|e| {
        panic!("failed to parse JSON from stdout: {e}\noutput was:\n{}", &stdout[json_start..])
    })
}

#[test]
fn test_fragment_clones_cli_workflow() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path();
    let src_dir = repo_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let code_a = r#"
pub fn process_orders(orders: &[i32]) -> i32 {
    let mut total = 0;
    for order in orders {
        if *order > 0 {
            total += order;
        }
    }
    total
}
"#;

    let code_b = r#"
pub fn audit_orders(orders: &[i32], verbose: bool) -> i32 {
    let mut total = 0;
    for order in orders {
        if *order > 0 {
            total += order;
        }
    }
    if verbose {
        total * 2
    } else {
        total
    }
}
"#;

    fs::write(src_dir.join("a.rs"), code_a).unwrap();
    fs::write(src_dir.join("b.rs"), code_b).unwrap();

    // 1. Discover repo
    let (disc_ok, _, disc_err) = run_cmd(repo_dir, &["discover", ".", "-l", "rust"]);
    assert!(disc_ok, "discover failed: {disc_err}");

    // 2. Seed-first query pinpointing duplicated 8-line loop
    let report = run_json(
        repo_dir,
        &[
            "-f",
            "json",
            "clones",
            "--mode",
            "fragment",
            "--seed",
            "process_orders",
            "--lines",
            "3-7",
        ],
    );

    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["mode"], "fragment");
    assert_eq!(report["group_count"], 1);

    let seed = &report["seed"];
    assert_eq!(seed["enclosing_function"], "process_orders");
    assert!(seed["file"].as_str().unwrap().ends_with("src/a.rs"));
    assert!(seed["structural_hash"].as_str().is_some());

    let groups = report["groups"].as_array().expect("groups array");
    assert_eq!(groups.len(), 1);
    let group = &groups[0];
    assert_eq!(group["size"], 2);
    assert_eq!(group["score"], 1.0);

    let members = group["members"].as_array().expect("members array");
    assert_eq!(members.len(), 2);
    let names: Vec<&str> = members
        .iter()
        .map(|m| m["enclosing_function"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"process_orders"));
    assert!(names.contains(&"audit_orders"));

    // Check snippet coordinates in members
    for m in members {
        assert!(m["start_line"].as_u64().unwrap() >= 2);
        assert!(m["end_line"].as_u64().unwrap() >= m["start_line"].as_u64().unwrap());
        assert!(m["statement_count"].as_u64().unwrap() >= 3);
    }

    // 3. Ambiguous seed resolution error handling
    // Add another function with the identical name in a different file
    fs::write(
        src_dir.join("c.rs"),
        "pub fn process_orders(x: i32) -> i32 { x + 1 }\n",
    )
    .unwrap();
    let (disc2_ok, _, _) = run_cmd(repo_dir, &["discover", ".", "-l", "rust"]);
    assert!(disc2_ok);

    let (ambig_ok, _, ambig_err) = run_cmd(
        repo_dir,
        &[
            "clones",
            "--mode",
            "fragment",
            "--seed",
            "process_orders",
        ],
    );
    assert!(
        !ambig_ok,
        "ambiguous seed query must fail when multiple functions match"
    );
    assert!(
        ambig_err.to_lowercase().contains("ambiguous") || ambig_err.contains("--file"),
        "error message should cite ambiguity or recommend --file: got {ambig_err}"
    );

    // Resolving ambiguity with --file
    let disambiguated = run_json(
        repo_dir,
        &[
            "-f",
            "json",
            "clones",
            "--mode",
            "fragment",
            "--seed",
            "process_orders",
            "--file",
            "src/a.rs",
            "--lines",
            "3-7",
        ],
    );
    assert_eq!(disambiguated["group_count"], 1);

    // 4. Full-repo unseeded discovery with sidecar caching
    let unseeded = run_json(
        repo_dir,
        &[
            "-f",
            "json",
            "clones",
            "--mode",
            "fragment",
        ],
    );
    assert_eq!(unseeded["schema_version"], 2);
    assert_eq!(unseeded["mode"], "fragment");
    assert!(unseeded["seed"].is_null());
    assert!(unseeded["group_count"].as_u64().unwrap() >= 1);

    let sidecar_file = repo_dir.join(".rgctl/clones.fragment.json");
    assert!(sidecar_file.is_file(), "expected .rgctl/clones.fragment.json sidecar file to exist");
}

//! Exact clone detection on `rgctl-tests/clone-exact`.
//!
//! ```bash
//! cargo test --test clone_exact_integration -- --nocapture
//! ```

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn rgctl_bin() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rgctl")
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/clone-exact")
}

fn run_json(args: &[&str]) -> Value {
    let bin = rgctl_bin();
    let out = Command::new(&bin)
        .current_dir(fixture_root())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    assert!(
        out.status.success(),
        "rgctl {:?} failed:\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Pretty JSON on stdout; markup goes to stderr.
    let json_start = stdout.find('{').expect("json object on stdout");
    serde_json::from_str(stdout[json_start..].trim()).expect("parse json")
}

#[test]
fn exact_clones_finds_duplicate_normalize_payload() {
    let root = fixture_root();
    let _ = std::fs::remove_dir_all(root.join(".rgctl"));

    let _discover = run_json(&["-f", "json", "discover", ".", "-l", "java"]);

    let report = run_json(&[
        "-f",
        "json",
        "clones",
        "--mode",
        "exact",
        "--min-loc",
        "5",
        "--exclude",
        "test",
    ]);

    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["mode"], "exact");
    assert!(report["group_count"].as_u64().unwrap() >= 1);

    let groups = report["groups"].as_array().expect("groups");
    let dup = groups
        .iter()
        .find(|g| {
            g["members"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|m| m["name"] == "normalizePayload")
        })
        .expect("normalizePayload group");
    assert_eq!(dup["mode"], "exact");
    assert!(dup["size"].as_u64().unwrap() >= 2);
    assert!(dup["hash"].as_str().is_some());
    assert_eq!(dup["confidence"], 1.0);

    let names: Vec<&str> = dup["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["name"].as_str())
        .collect();
    assert!(names.contains(&"normalizePayload"));
    // excluded test/ path — only CloneA + CloneB
    assert_eq!(dup["size"], 2);

    let sidecar = root.join(".rgctl/clones.json");
    assert!(sidecar.is_file(), "expected {}", sidecar.display());

    // Symbol-scoped
    let scoped = run_json(&[
        "-f",
        "json",
        "clones",
        "normalizePayload",
        "--file",
        "CloneA.java",
        "--min-loc",
        "5",
        "--exclude",
        "test",
    ]);
    assert_eq!(scoped["group_count"], 1);
    assert_eq!(scoped["seed"]["name"], "normalizePayload");
    assert!(
        scoped["seed"]["file"]
            .as_str()
            .unwrap_or("")
            .contains("CloneA.java")
    );

    // Bloom candidates (exact dups share near-identical token blooms)
    let bloom = run_json(&[
        "-f",
        "json",
        "clones",
        "--mode",
        "bloom",
        "--threshold",
        "0.85",
        "--min-loc",
        "5",
        "--exclude",
        "test",
        "--no-cache",
    ]);
    assert_eq!(bloom["mode"], "bloom");
    assert_eq!(bloom["candidates"], true);
    assert_eq!(bloom["threshold"], 0.85);
    assert!(bloom["group_count"].as_u64().unwrap() >= 1);
    let bg = bloom["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| {
            g["members"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|m| m["name"] == "normalizePayload")
        })
        .expect("bloom normalizePayload group");
    assert_eq!(bg["mode"], "bloom");
    assert!(bg["score"].as_f64().unwrap() >= 0.85);
    assert!(bg["size"].as_u64().unwrap() >= 2);

    // With write enabled, bloom sidecar is mode-specific
    let _ = run_json(&[
        "-f",
        "json",
        "clones",
        "--mode",
        "bloom",
        "--threshold",
        "0.85",
        "--exclude",
        "test",
    ]);
    assert!(
        root.join(".rgctl/clones.bloom.json").is_file(),
        "expected clones.bloom.json sidecar"
    );
}

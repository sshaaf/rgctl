//! Dashboard gate — **ecommerce-php** test project (CFG/PDG/taint on PHP).

mod dashboard_harness;

use dashboard_harness::{
    assert_dashboard_bundle_all_analysis, default_php_repo, run_discover_all,
};
use rgctl_dashboard::dist_embedded;

const PHP_MIN_NODES: u64 = 10;
const PHP_MIN_FUNCTIONS: u64 = 5;
const PHP_MIN_METANODES: u64 = 1;

#[test]
fn discover_all_writes_php_cfg_dashboard_bundle() {
    if !dist_embedded() {
        panic!(
            "dashboard/dist not embedded — run ./scripts/build-dashboard.sh && cargo build --release"
        );
    }

    let repo = default_php_repo();
    if !repo.is_dir() {
        eprintln!("skip: php test repo not found at {}", repo.display());
        return;
    }

    let output = run_discover_all(&repo, Some("php"));
    assert!(
        output.status.success(),
        "discover --all on ecommerce-php failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_dashboard_bundle_all_analysis(&repo, PHP_MIN_NODES, PHP_MIN_METANODES);

    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo.join(".rgctl/dashboard/manifest.json")).unwrap(),
    )
    .unwrap();
    let functions = manifest["metrics"]["function_count"].as_u64().unwrap_or(0);
    assert!(
        functions >= PHP_MIN_FUNCTIONS,
        "expected >= {PHP_MIN_FUNCTIONS} functions, got {functions}"
    );

    let cfg_index: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo.join(".rgctl/dashboard/cfg_index.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(cfg_index["available"], true);
    assert!(
        cfg_index["function_count"].as_u64().unwrap_or(0) > 0,
        "cfg_index should list analyzed PHP functions"
    );

    let calls_count = manifest["metrics"]["calls_count"].as_u64().unwrap_or(0);
    assert!(calls_count > 0, "expected non-zero call graph edges");

    eprintln!(
        "ecommerce-php OK: {} nodes, {} functions, {} cfg functions, {} calls",
        manifest["graph"]["node_count"],
        functions,
        cfg_index["function_count"],
        calls_count
    );
}

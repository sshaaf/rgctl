//! Soft regression gates for golden-repo `discover --with-cfg --with-security --with-taint` (+10% tolerance).
//! Baseline seconds are defined in this file (`METASFRESH_*`, `GBUILDER_*` constants).
//!
//! Run manually after golden-repo discover:
//!   cargo test --release --test discover_perf_baselines -- --ignored --nocapture

mod dashboard_harness;

use dashboard_harness::{golden_repo_path, metasfresh_repo_path, run_discover_all_timed};
use std::time::Duration;

/// metasfresh reference: ~8.9 min discover --with-cfg --with-security --with-taint (531 s, measured 2026-07-10).
const METASFRESH_DISCOVER_ALL_BASELINE_SECS: f64 = 531.0;
/// gbuilder reference when env not set (5.5 s, measured 2026-07-10).
const GBUILDER_DISCOVER_ALL_BASELINE_SECS: f64 = 5.5;
const METASFRESH_TOLERANCE: f64 = 1.10;

fn assert_within_baseline(label: &str, elapsed: Duration, baseline_secs: f64) {
    let limit = baseline_secs * METASFRESH_TOLERANCE;
    assert!(
        elapsed.as_secs_f64() <= limit,
        "{label}: {:.1}s exceeds baseline {:.1}s (+10% = {:.1}s)",
        elapsed.as_secs_f64(),
        baseline_secs,
        limit
    );
}

#[test]
#[ignore = "manual: requires metasfresh checkout and long discover --with-cfg --with-security --with-taint run"]
fn metasfresh_discover_all_within_baseline() {
    let repo = metasfresh_repo_path();
    if !repo.is_dir() {
        eprintln!("skip: metasfresh not at {}", repo.display());
        return;
    }

    let (output, elapsed) = run_discover_all_timed(&repo, None);
    assert!(
        output.status.success(),
        "discover --with-cfg --with-security --with-taint failed"
    );
    eprintln!(
        "metasfresh discover --with-cfg --with-security --with-taint: {:.1}s (baseline {:.0}s)",
        elapsed.as_secs_f64(),
        METASFRESH_DISCOVER_ALL_BASELINE_SECS
    );
    assert_within_baseline(
        "metasfresh discover --with-cfg --with-security --with-taint",
        elapsed,
        METASFRESH_DISCOVER_ALL_BASELINE_SECS,
    );
}

#[test]
#[ignore = "manual: gbuilder discover --with-cfg --with-security --with-taint (~5.5s baseline)"]
fn gbuilder_discover_all_within_baseline() {
    let baseline_secs = std::env::var("RGCTL_GBUILDER_DISCOVER_ALL_BASELINE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(GBUILDER_DISCOVER_ALL_BASELINE_SECS);

    let repo = golden_repo_path();
    if !repo.is_dir() {
        eprintln!("skip: gbuilder not at {}", repo.display());
        return;
    }

    let (output, elapsed) = run_discover_all_timed(&repo, Some("java"));
    assert!(
        output.status.success(),
        "discover --with-cfg --with-security --with-taint failed"
    );
    eprintln!(
        "gbuilder discover --with-cfg --with-security --with-taint: {:.1}s (baseline {:.1}s)",
        elapsed.as_secs_f64(),
        baseline_secs
    );
    assert_within_baseline(
        "gbuilder discover --with-cfg --with-security --with-taint",
        elapsed,
        baseline_secs,
    );
}

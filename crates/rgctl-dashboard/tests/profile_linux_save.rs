//! Manual profile: dashboard export on example/linux (requires prior discover).

use rgctl_dashboard::DashboardExportContext;
use rgctl_dashboard::export_dashboard_bundle_with_context;
use rgctl_graph::snapshot::MmappedGraphSnapshot;
use std::path::Path;
use std::time::Instant;

#[test]
#[ignore = "manual: profile save_dashboard on example/linux artifact"]
fn profile_linux_dashboard_export() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("profile=info")
        .try_init();

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../example/linux");
    let snapshot_path = root.join(".rgctl/graph.snapshot.bin");
    if !snapshot_path.is_file() {
        eprintln!("skip: {} missing", snapshot_path.display());
        return;
    }

    let hydrate_start = Instant::now();
    let snapshot = MmappedGraphSnapshot::open(&snapshot_path).expect("open snapshot");
    let backend = snapshot.hydrate_backend().expect("hydrate backend");
    tracing::info!(
        target: "profile",
        secs = hydrate_start.elapsed().as_secs_f64(),
        nodes = backend.node_count(),
        "[profile] hydrate_backend for dashboard export"
    );

    let dashboard = root.join(".rgctl/dashboard");
    if dashboard.exists() {
        std::fs::remove_dir_all(&dashboard).expect("remove dashboard");
    }

    let analysis_path = root.join(".rgctl/analysis_results.bin");
    let analysis =
        rgctl_analysis::AnalysisResults::load(&analysis_path).expect("load analysis");

    export_dashboard_bundle_with_context(
        &backend,
        &root,
        &snapshot_path,
        DashboardExportContext::with_analysis(&analysis),
    )
    .expect("export dashboard");
}

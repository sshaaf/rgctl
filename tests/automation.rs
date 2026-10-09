//! Phase 13: change detection and incremental updates.

use rgctl::changes::ChangeDetector;
use rgctl::config::project::{RgctlConfig, RiskLevel};
use rgctl::graph::backend::GraphBackend;
use rgctl::graph::schema::{Edge, EdgeType, Node, NodeType};
use rgctl::incremental::{IncrementalUpdater, UpdateOptions, changes_for_paths};
use rgctl::languages::registry::LanguageRegistry;
use rgctl::pipeline::{PipelineConfig, ProcessingPipeline};
use std::fs;
use tempfile::TempDir;

fn chain_graph_repo(temp: &TempDir) -> rgctl::CodeGraph {
    let root = temp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn a() { b(); }\npub fn b() { c(); }\npub fn c() {}\n",
    )
    .unwrap();

    let pipeline = ProcessingPipeline::with_config(
        LanguageRegistry::new().into(),
        PipelineConfig {
            show_progress: false,
            ..PipelineConfig::default()
        },
    );
    let (graph, _) = pipeline.process_repository(root).unwrap();
    graph.save_to_repo(root).unwrap();

    let mut tracker = rgctl::incremental::FileTracker::new(root);
    let files = vec![root.join("src/lib.rs")];
    tracker.index_files(&files, &graph).unwrap();
    tracker.save().unwrap();
    graph
}

#[test]
fn test_changes_for_paths_modified() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let _graph = chain_graph_repo(&temp);
    let changes = changes_for_paths(root, &["src/lib.rs".into()]).unwrap();
    assert!(
        changes.changed.contains(&"src/lib.rs".to_string())
            || changes.added.contains(&"src/lib.rs".to_string())
    );
}

#[test]
fn test_detect_changes_risk_on_chain() {
    let mut graph = rgctl::CodeGraph::new();
    let backend = graph.backend_mut();
    let a = Node::new(NodeType::Function, "a").with_file_path("src/lib.rs");
    let b = Node::new(NodeType::Function, "b").with_file_path("src/lib.rs");
    let c = Node::new(NodeType::Function, "c").with_file_path("src/lib.rs");
    let id_a = a.id;
    let id_b = b.id;
    let id_c = c.id;
    backend.insert_node(a).unwrap();
    backend.insert_node(b).unwrap();
    backend.insert_node(c).unwrap();
    backend
        .insert_edge(Edge::new(id_a, id_b, EdgeType::Calls))
        .unwrap();
    backend
        .insert_edge(Edge::new(id_b, id_c, EdgeType::Calls))
        .unwrap();

    let detector = ChangeDetector::new();
    let result = detector.detect(&graph, &["src/lib.rs".into()]).unwrap();
    assert!(!result.details.is_empty());
    assert!(result.details.iter().any(|d| d.symbol == "c"));
}

#[test]
fn test_update_files_incremental() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let mut graph = chain_graph_repo(&temp);
    let lib = root.join("src/lib.rs");
    fs::write(
        &lib,
        "pub fn a() { b(); }\npub fn b() { c(); }\npub fn c() {}\npub fn d() {}\n",
    )
    .unwrap();

    let updater = IncrementalUpdater::with_options(
        LanguageRegistry::new().into(),
        UpdateOptions {
            show_progress: false,
            ..Default::default()
        },
    );
    let result = updater
        .update_files(&mut graph, root, &["src/lib.rs".into()])
        .unwrap();
    assert!(result.files_changed >= 1 || result.nodes_added > 0);
}

#[test]
fn test_rgctl_config_defaults() {
    let temp = TempDir::new().unwrap();
    let cfg = RgctlConfig::load(temp.path()).unwrap();
    assert_eq!(cfg.hooks.block_on_risk, RiskLevel::Critical);
    assert_eq!(cfg.watch.debounce_ms, 500);
}

#[test]
fn test_manual_graph_blast_risk() {
    let mut graph = rgctl::CodeGraph::new();
    let backend = graph.backend_mut();
    let a = Node::new(NodeType::Function, "a").with_file_path("f.rs");
    let b = Node::new(NodeType::Function, "b").with_file_path("f.rs");
    let c = Node::new(NodeType::Function, "c").with_file_path("f.rs");
    let id_a = a.id;
    let id_b = b.id;
    let id_c = c.id;
    backend.insert_node(a).unwrap();
    backend.insert_node(b).unwrap();
    backend.insert_node(c).unwrap();
    backend
        .insert_edge(Edge::new(id_a, id_b, EdgeType::Calls))
        .unwrap();
    backend
        .insert_edge(Edge::new(id_b, id_c, EdgeType::Calls))
        .unwrap();

    let result = ChangeDetector::new()
        .detect(&graph, &["f.rs".into()])
        .unwrap();
    assert!(result.details.iter().any(|d| d.symbol == "c"));
}

#[test]
fn test_critical_risk_blocks_per_config() {
    use rgctl::config::project::RgctlConfig;
    let cfg = RgctlConfig::default();
    assert!(cfg.hooks.block_on_risk.blocks(RiskLevel::Critical));
    assert!(!cfg.hooks.block_on_risk.blocks(RiskLevel::Medium));
}

#[test]
fn test_high_risk_blocked_when_configured() {
    use rgctl::config::project::{RgctlConfig, RiskLevel};
    let mut cfg = RgctlConfig::default();
    cfg.hooks.block_on_risk = RiskLevel::High;
    assert!(cfg.hooks.block_on_risk.blocks(RiskLevel::Critical));
    assert!(cfg.hooks.block_on_risk.blocks(RiskLevel::High));
    assert!(!cfg.hooks.block_on_risk.blocks(RiskLevel::Medium));
}

#[test]
fn test_detect_changes_json_contains_summary() {
    let mut graph = rgctl::CodeGraph::new();
    let backend = graph.backend_mut();
    let leaf = Node::new(NodeType::Function, "leaf").with_file_path("f.rs");
    backend.insert_node(leaf).unwrap();
    let result = ChangeDetector::new()
        .detect(&graph, &["f.rs".into()])
        .unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("summary"));
    assert!(json.contains("files_analyzed"));
}

#[test]
fn test_changes_for_paths_detects_new_file() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let _graph = chain_graph_repo(&temp);
    fs::write(root.join("src/extra.rs"), "pub fn extra() {}\n").unwrap();
    let changes = changes_for_paths(root, &["src/extra.rs".into()]).unwrap();
    assert!(changes.added.contains(&"src/extra.rs".to_string()));
}

#[test]
fn test_full_workflow_modify_and_update() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let mut graph = chain_graph_repo(&temp);
    let before = graph.node_count();

    let lib = root.join("src/lib.rs");
    fs::write(
        &lib,
        "pub fn a() { b(); }\npub fn b() { c(); }\npub fn c() {}\npub fn added() {}\n",
    )
    .unwrap();

    let updater = IncrementalUpdater::with_options(
        LanguageRegistry::new().into(),
        UpdateOptions {
            show_progress: false,
            ..Default::default()
        },
    );
    let result = updater
        .update_files(&mut graph, root, &["src/lib.rs".into()])
        .unwrap();
    assert!(result.files_affected() >= 1);
    assert!(graph.node_count() >= before);
}

#[test]
fn test_cli_update_paths_and_noop() {
    use rgctl::cli::update::{UpdateArgs, run_update_at, update_paths};

    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let graph = chain_graph_repo(&temp);
    // CLI update requires columnar snapshot (save_to_repo alone is legacy JSON).
    graph.save_snapshot(root).unwrap();

    let noop = run_update_at(
        root,
        &UpdateArgs {
            cascade_depth: 1,
            ..UpdateArgs::default()
        },
        false,
        true,
    )
    .unwrap();
    assert_eq!(noop.files_affected(), 0);

    fs::write(
        root.join("src/lib.rs"),
        "pub fn a() { b(); }\npub fn b() { c(); }\npub fn c() {}\npub fn e() {}\n",
    )
    .unwrap();
    let updated = update_paths(root, &["src/lib.rs".into()], 1).unwrap();
    assert!(updated.files_affected() >= 1 || updated.nodes_added > 0);
}

#[test]
fn test_update_in_process_blocked_when_watch_lock_held() {
    use rgctl::cli::pipeline_status::try_acquire_watch_lock;
    use rgctl::cli::update::{UpdateArgs, run_update_at};

    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let graph = chain_graph_repo(&temp);
    graph.save_snapshot(root).unwrap();

    let _watch = try_acquire_watch_lock(root).unwrap();
    // In-process compact still cannot take the lock while it is held (watch path).
    let err = run_update_at(
        root,
        &UpdateArgs {
            cascade_depth: 1,
            ..UpdateArgs::default()
        },
        false,
        true,
    )
    .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("watch") || msg.contains("watcher") || msg.contains("lock"),
        "unexpected error: {msg}"
    );
}

#[test]
fn test_update_queue_enqueue_drain_and_result() {
    use rgctl::cli::pipeline_status::{
        WATCH_LOCK_FILE, detect_live_watcher, process_alive, update_queue, watch_lock_path,
    };
    use rgctl::cli::update::apply_queue_batch;
    use update_queue::{
        UpdateQueueMode, build_request, drain_queue, enqueue, queue_nonempty, read_result,
    };

    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let graph = chain_graph_repo(&temp);
    graph.save_snapshot(root).unwrap();

    // Simulate an external live watcher (pid 1 is typically init/launchd).
    let external_pid = 1u32;
    if !process_alive(external_pid) || external_pid == std::process::id() {
        return;
    }
    let lock = watch_lock_path(root);
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    fs::write(&lock, format!("{external_pid}\n")).unwrap();
    assert!(
        detect_live_watcher(root).is_some(),
        "expected live watcher for pid {external_pid}"
    );
    assert!(lock.file_name().and_then(|n| n.to_str()) == Some(WATCH_LOCK_FILE));

    let req = build_request(
        UpdateQueueMode::Paths,
        Some(vec!["src/lib.rs".into()]),
        None,
        Some(1),
        None,
        None,
    )
    .unwrap();
    let id = req.request_id.clone();
    enqueue(root, &req).unwrap();
    assert!(queue_nonempty(root));

    let batch = drain_queue(root).unwrap();
    assert_eq!(batch.requests.len(), 1);
    apply_queue_batch(root, &batch, &[]).unwrap();
    let result = read_result(root, &id).unwrap().expect("result file");
    assert!(result.ok, "expected ok result, got {:?}", result.error);
    assert_eq!(result.source, "watch_queue");
}

#[test]
fn test_update_queue_no_wait_leaves_pending() {
    use rgctl::cli::pipeline_status::{process_alive, update_queue, watch_lock_path};
    use update_queue::{
        UpdateQueueMode, build_request, enqueue, queue_nonempty, queue_pending_count,
        wait_for_result,
    };
    use std::time::Duration;

    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let external_pid = 1u32;
    if !process_alive(external_pid) || external_pid == std::process::id() {
        return;
    }
    let lock = watch_lock_path(root);
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    fs::write(&lock, format!("{external_pid}\n")).unwrap();

    let req = build_request(UpdateQueueMode::HashDiff, None, None, Some(1), None, None).unwrap();
    let id = req.request_id.clone();
    enqueue(root, &req).unwrap();
    assert!(queue_nonempty(root));
    assert_eq!(queue_pending_count(root), 1);
    // No watcher drain → wait times out (same failure mode as CLI --wait-timeout).
    let err = wait_for_result(root, &id, Duration::from_millis(80)).unwrap_err();
    assert!(
        format!("{err:#}").contains("timed out"),
        "expected timeout, got {err:#}"
    );
}

#[test]
fn test_stale_watch_lock_allows_local_update() {
    use rgctl::cli::pipeline_status::{process_alive, try_acquire_watch_lock, watch_lock_path};
    use rgctl::cli::update::{UpdateArgs, run_update_at};

    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let graph = chain_graph_repo(&temp);
    graph.save_snapshot(root).unwrap();

    let dead_pid = 4_294_967_294u32;
    if process_alive(dead_pid) {
        return;
    }
    let lock = watch_lock_path(root);
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    fs::write(&lock, format!("{dead_pid}\n")).unwrap();

    // Reclaim + in-process update succeeds.
    let result = run_update_at(
        root,
        &UpdateArgs {
            cascade_depth: 1,
            ..UpdateArgs::default()
        },
        false,
        true,
    )
    .unwrap();
    assert_eq!(result.files_affected(), 0);
    // Lock should now be held by us or released after Drop of temporary acquire inside run.
    drop(result);
    let _ = try_acquire_watch_lock(root).expect("lock free or reclaimable after update");
}

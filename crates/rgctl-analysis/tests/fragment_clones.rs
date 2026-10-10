//! Unit and integration tests for fragment clone detection engine.

use rgctl_analysis::cfg_builder::build_cfg_for_function;
use rgctl_analysis::fragment_clones::{
    query_fragment_clones, stage1_filter_functions, FragmentSeedQuery,
};
use rgctl_analysis::sese::extract_sese_regions;
use rgctl_analysis::wl_hash::WeisfeilerLehmanHasher;
use rgctl_analysis::{FragmentCloneFilters, MODE_FRAGMENT};
use rgctl_graph::backend::{GraphBackend, MemoryBackend};
use rgctl_graph::schema::{Node, NodeType};
use rgctl_graph::structural_sketch::build_token_bloom;
use rgctl_graph::write_columnar_from_nodes_edges;
use rgctl_graph::SnapshotNodeStore;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_sese_and_wl_hash_pipeline() {
    let code_orig = r#"
fn process_events(events: &[i32]) -> i32 {
    let mut count = 0;
    for e in events {
        if *e > 0 {
            count += 1;
        }
    }
    count
}
"#;

    let code_renamed = r#"
fn handle_items(items: &[i32]) -> i32 {
    let mut total = 0;
    for item in items {
        if *item > 0 {
            total += 1;
        }
    }
    total
}
"#;

    let cfg_orig = build_cfg_for_function("rust", code_orig, "process_events").unwrap();
    let cfg_renamed = build_cfg_for_function("rust", code_renamed, "handle_items").unwrap();

    let regions_orig = extract_sese_regions(&cfg_orig, 3, 15);
    let regions_renamed = extract_sese_regions(&cfg_renamed, 3, 15);

    assert!(!regions_orig.is_empty(), "orig SESE regions");
    assert!(!regions_renamed.is_empty(), "renamed SESE regions");

    let hash_orig = WeisfeilerLehmanHasher::hash_region(&cfg_orig, &regions_orig[0]);
    let hash_renamed = WeisfeilerLehmanHasher::hash_region(&cfg_renamed, &regions_renamed[0]);

    assert_eq!(
        hash_orig, hash_renamed,
        "Structural 1-WL hash must match for identical CFG fragment with renamed variables"
    );
}

#[test]
fn test_stage1_bitwise_bloom_pruning() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("repo");
    fs::create_dir_all(&repo_dir).unwrap();

    let node1 = Node::new(NodeType::Function, "processPayment")
        .with_file_path("src/payment.rs")
        .with_location(1, 20);
    let mut node1 = node1;
    let bloom1 = build_token_bloom(
        "processPayment",
        None,
        None,
        Some("for item in items { if item.valid() { process(item); } }"),
    );
    node1.token_bloom = Some(bloom1);

    let node2 = Node::new(NodeType::Function, "renderTemplate")
        .with_file_path("src/view.rs")
        .with_location(1, 25);
    let mut node2 = node2;
    let bloom2 = build_token_bloom(
        "renderTemplate",
        None,
        None,
        Some("let html = div + span; return html;"),
    );
    node2.token_bloom = Some(bloom2);

    let mut backend = MemoryBackend::new();
    backend.insert_node(node1.clone()).unwrap();
    backend.insert_node(node2.clone()).unwrap();

    let snap_dir = repo_dir.join(".rgctl");
    fs::create_dir_all(&snap_dir).unwrap();
    let snap_file = snap_dir.join("graph.snapshot.bin");
    write_columnar_from_nodes_edges(
        backend.all_nodes().unwrap(),
        backend.all_edges().unwrap(),
        &snap_file,
    )
    .unwrap();

    let store = SnapshotNodeStore::open(&snap_file).unwrap();

    // Query for tokens in payment fragment
    let seed_text = "for item in items { process(item); }";
    let seed_bloom = build_token_bloom("", None, None, Some(seed_text));

    let filters = FragmentCloneFilters::default();
    let candidates = stage1_filter_functions(&store, &seed_bloom, &filters).unwrap();

    assert_eq!(candidates.len(), 1, "Stage 1 must prune unrelated renderTemplate function");
    assert_eq!(candidates[0].name, "processPayment");
}

#[test]
fn test_stage2_query_fragment_clones() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("repo");
    fs::create_dir_all(repo_dir.join("src")).unwrap();

    let src1 = r#"
fn service_a(items: &[i32]) -> i32 {
    let mut total = 0;
    for x in items {
        if *x > 0 {
            total += x;
        }
    }
    total
}
"#;

    let src2 = r#"
fn service_b(items: &[i32], flag: bool) -> i32 {
    let mut total = 0;
    for x in items {
        if *x > 0 {
            total += x;
        }
    }
    if flag { total * 2 } else { total }
}
"#;

    fs::write(repo_dir.join("src/a.rs"), src1).unwrap();
    fs::write(repo_dir.join("src/b.rs"), src2).unwrap();

    let node_a = Node::new(NodeType::Function, "service_a")
        .with_file_path("src/a.rs")
        .with_location(2, 11);
    let mut node_a = node_a;
    node_a.token_bloom = Some(build_token_bloom("service_a", None, None, Some(src1)));

    let node_b = Node::new(NodeType::Function, "service_b")
        .with_file_path("src/b.rs")
        .with_location(2, 12);
    let mut node_b = node_b;
    node_b.token_bloom = Some(build_token_bloom("service_b", None, None, Some(src2)));

    let mut backend = MemoryBackend::new();
    backend.insert_node(node_a.clone()).unwrap();
    backend.insert_node(node_b.clone()).unwrap();

    let snap_dir = repo_dir.join(".rgctl");
    fs::create_dir_all(&snap_dir).unwrap();
    let snap_file = snap_dir.join("graph.snapshot.bin");
    write_columnar_from_nodes_edges(
        backend.all_nodes().unwrap(),
        backend.all_edges().unwrap(),
        &snap_file,
    )
    .unwrap();

    let store = SnapshotNodeStore::open(&snap_file).unwrap();

    let query = FragmentSeedQuery {
        symbol: Some("service_a".to_string()),
        file: None,
        lines: Some((3, 6)),
    };

    let filters = FragmentCloneFilters::default();
    let report = query_fragment_clones(&store, &repo_dir, query, filters).unwrap();

    assert_eq!(report.mode, MODE_FRAGMENT);
    assert_eq!(report.schema_version, 2);
    assert!(report.seed.is_some());
    assert_eq!(report.group_count, 1, "expected 1 clone group found");
    assert_eq!(report.groups[0].size, 2, "expected both service_a and service_b as members");
}

#[test]
fn test_discover_fragment_clones_unseeded_and_sidecar() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("repo");
    fs::create_dir_all(repo_dir.join("src")).unwrap();

    let src1 = r#"
fn func_alpha(items: &[i32]) -> i32 {
    let mut total = 0;
    for x in items {
        if *x > 0 {
            total += x;
        }
    }
    total
}
"#;

    let src2 = r#"
fn func_beta(items: &[i32], verbose: bool) -> i32 {
    let mut total = 0;
    for x in items {
        if *x > 0 {
            total += x;
        }
    }
    if verbose { total * 2 } else { total }
}
"#;

    fs::write(repo_dir.join("src/a.rs"), src1).unwrap();
    fs::write(repo_dir.join("src/b.rs"), src2).unwrap();

    let node_a = Node::new(NodeType::Function, "func_alpha")
        .with_file_path("src/a.rs")
        .with_location(2, 11);
    let mut node_a = node_a;
    node_a.token_bloom = Some(build_token_bloom("func_alpha", None, None, Some(src1)));

    let node_b = Node::new(NodeType::Function, "func_beta")
        .with_file_path("src/b.rs")
        .with_location(2, 12);
    let mut node_b = node_b;
    node_b.token_bloom = Some(build_token_bloom("func_beta", None, None, Some(src2)));

    let mut backend = MemoryBackend::new();
    backend.insert_node(node_a).unwrap();
    backend.insert_node(node_b).unwrap();

    let snap_dir = repo_dir.join(".rgctl");
    fs::create_dir_all(&snap_dir).unwrap();
    let snap_file = snap_dir.join("graph.snapshot.bin");
    write_columnar_from_nodes_edges(
        backend.all_nodes().unwrap(),
        backend.all_edges().unwrap(),
        &snap_file,
    )
    .unwrap();

    let store = SnapshotNodeStore::open(&snap_file).unwrap();
    let filters = FragmentCloneFilters::default();

    // 1. Run unseeded discover
    let report = rgctl_analysis::fragment_clones::discover_fragment_clones(
        &store,
        &repo_dir,
        filters.clone(),
    )
    .unwrap();

    assert_eq!(report.mode, MODE_FRAGMENT);
    assert_eq!(report.schema_version, 2);
    assert!(report.seed.is_none());
    assert!(!report.groups.is_empty(), "expected at least one clone group");

    // 2. Save sidecar and verify reload
    rgctl_analysis::clones::save_fragment_sidecar(&repo_dir, &report).unwrap();
    let cached = rgctl_analysis::clones::load_fragment_sidecar_if_fresh(
        &repo_dir,
        &report.graph_digest,
    )
    .unwrap();
    assert!(cached.is_some(), "cached fragment report must load");
    let cached = cached.unwrap();
    assert_eq!(cached.groups.len(), report.groups.len());

    // 3. Stale digest returns None
    let stale = rgctl_analysis::clones::load_fragment_sidecar_if_fresh(
        &repo_dir,
        "stale_digest_12345",
    )
    .unwrap();
    assert!(stale.is_none(), "stale digest must be rejected");
}

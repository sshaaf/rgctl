//! End-to-end extract + populate tests for the markdown-context fixture.

use rgctl_extraction::discovery::DiscoveryConfig;
use rgctl_extraction::extractor::Extractor;
use rgctl_extraction::graph_builder::GraphBuilder;
use rgctl_graph::schema::{EdgeType, NodeType};
use std::path::PathBuf;
use std::sync::Arc;

fn checkout_heading<'a>(
    nodes: &'a [rgctl_graph::schema::Node],
) -> Option<&'a rgctl_graph::schema::Node> {
    nodes.iter().find(|n| {
        n.node_type == NodeType::Module
            && n.name == "Checkout Flow"
            && n.get_property("kind") == Some("heading")
    })
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/markdown-context")
}

#[test]
fn extract_repository_symbols_have_body_text() {
    let root = fixture_root();
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);
    let config = rgctl_extraction::discovery::DiscoveryConfig {
        languages: Some(vec!["markdown".to_string()]),
        ..rgctl_extraction::discovery::DiscoveryConfig::default()
    };
    let extractions = extractor
        .extract_repository(&root, &config)
        .expect("extract");
    let guide = extractions
        .iter()
        .find(|e| e.path.ends_with("guide.md"))
        .expect("guide extraction");
    let checkout = guide
        .symbols
        .iter()
        .find(|s| s.name == "Checkout Flow")
        .expect("checkout symbol");
    assert!(
        checkout.metadata.get("body_text").is_some(),
        "repository extract body_text: {:?}",
        checkout.metadata
    );
}

#[test]
fn full_repo_populate_graph_has_body_text() {
    let root = fixture_root();
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);
    let config = rgctl_extraction::discovery::DiscoveryConfig {
        languages: Some(vec!["markdown".to_string()]),
        ..rgctl_extraction::discovery::DiscoveryConfig::default()
    };
    let extractions = extractor
        .extract_repository(&root, &config)
        .expect("extract");
    let mut builder = GraphBuilder::new();
    extractor
        .populate_graph(&extractions, &mut builder)
        .expect("populate");
    let (nodes, _) = builder.into_graph();
    let checkout = nodes
        .iter()
        .find(|n| {
            n.node_type == rgctl_graph::schema::NodeType::Module
                && n.name == "Checkout Flow"
                && n.get_property("kind") == Some("heading")
        })
        .expect("checkout heading");
    assert!(
        checkout
            .get_property("body_text")
            .is_some_and(|b| b.contains("End-to-end checkout")),
        "populate_graph body_text: {:?}",
        checkout.get_property("body_text")
    );
}

#[test]
fn fixture_discover_snapshot_preserves_body_text_in_place() {
    let root = fixture_root();
    let tmp = tempfile::tempdir().expect("tempdir");
    let snapshot_path = tmp.path().join("graph.snapshot.bin");
    let registry = Arc::new(rgctl_languages::default_registry());
    let pipeline = rgctl_pipeline::ProcessingPipeline::with_config(
        registry,
        rgctl_pipeline::PipelineConfig {
            discovery: DiscoveryConfig {
                languages: Some(vec!["markdown".to_string(), "java".to_string()]),
                ..DiscoveryConfig::default()
            },
            ..rgctl_pipeline::PipelineConfig::default()
        },
    );
    pipeline
        .process_repository_to_snapshot(&root, &snapshot_path, None)
        .expect("discover snapshot");

    let graph = rgctl_graph::CodeGraph::open_snapshot(&snapshot_path).expect("open");
    let nodes = graph.backend().all_nodes().expect("nodes");
    let checkout = checkout_heading(&nodes).expect("checkout heading");
    assert!(
        checkout
            .get_property("body_text")
            .is_some_and(|b| b.contains("End-to-end checkout")),
        "discover spill snapshot body_text: {:?}",
        checkout.get_property("body_text")
    );
}

#[test]
fn fixture_snapshot_roundtrip_preserves_body_text() {
    let root = fixture_root();
    let guide = root.join("docs/guide.md");
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);
    let extraction = extractor.extract_file(&guide).expect("extract guide");

    let mut builder = GraphBuilder::new();
    extractor
        .populate_graph(&[extraction], &mut builder)
        .expect("populate");

    let (nodes, edges) = builder.into_graph();
    let mut graph = rgctl_graph::CodeGraph::new();
    graph.load(nodes, edges).expect("load graph");

    let tmp = tempfile::tempdir().expect("tempdir");
    let snapshot_path = tmp.path().join("graph.snapshot.bin");
    let prepared = rgctl_graph::snapshot::PreparedGraphSnapshot::from_backend(graph.backend())
        .expect("prepared");
    prepared
        .write_columnar_to_path(&snapshot_path)
        .expect("write snapshot");

    let graph = rgctl_graph::CodeGraph::open_snapshot(&snapshot_path).expect("open");
    let backend = graph.backend();
    let parsed = rgctl_gql::parser::parse(
        "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' AND n.body_text LIKE 'End-to-end*' RETURN n",
    )
    .expect("parse");
    let result = rgctl_gql::executor::QueryExecutor::new(backend)
        .execute(&parsed)
        .expect("execute");
    assert!(
        !result.rows.is_empty(),
        "body_text must survive snapshot roundtrip"
    );
}

#[test]
fn fixture_guide_populates_references_and_contains() {
    let root = fixture_root();
    let guide = root.join("docs/guide.md");
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);

    let extraction = extractor.extract_file(&guide).expect("extract guide");
    assert!(
        extraction.symbols.iter().any(|s| s.name == "Checkout Flow"),
        "Checkout Flow symbol"
    );
    assert!(
        extraction
            .relations
            .iter()
            .any(|r| { r.to.contains("adr.md#payments") || r.to.ends_with("adr.md#payments") }),
        "relation to payments fragment"
    );

    let mut builder = GraphBuilder::new();
    extractor
        .populate_graph(&[extraction], &mut builder)
        .expect("populate");

    let (nodes, edges) = builder.into_graph();

    let checkout = checkout_heading(&nodes).expect("Checkout Flow heading");
    assert!(
        checkout
            .get_property("body_text")
            .is_some_and(|b| b.contains("End-to-end checkout")),
        "checkout body_text on graph node: {:?}",
        checkout.get_property("body_text")
    );
    let cart = nodes
        .iter()
        .find(|n| n.name == "Cart" && n.node_type == NodeType::Module)
        .expect("Cart node");

    assert!(
        edges.iter().any(|e| {
            e.edge_type == EdgeType::Contains && e.from == checkout.id && e.to == cart.id
        }),
        "CONTAINS edge Checkout -> Cart"
    );
    assert!(
        edges
            .iter()
            .any(|e| { e.edge_type == EdgeType::References && e.from == checkout.id }),
        "REFERENCES edge from Checkout Flow"
    );
}

#[test]
fn fixture_discover_markdown_file_count() {
    let root = fixture_root();
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);
    let config = DiscoveryConfig {
        languages: Some(vec!["markdown".to_string()]),
        ..DiscoveryConfig::default()
    };
    let extractions = extractor
        .extract_repository(&root, &config)
        .expect("discover");
    assert!(
        extractions.len() >= 4,
        "README, guide, adr, overview.mdx — got {}",
        extractions.len()
    );
}

#[test]
fn fixture_readme_populates_frontmatter_variables() {
    let root = fixture_root();
    let readme = root.join("README.md");
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);
    let extraction = extractor.extract_file(&readme).expect("README");
    assert!(
        extraction
            .symbols
            .iter()
            .any(|s| s.name == "metadata.author"),
        "metadata.author symbol"
    );

    let mut builder = GraphBuilder::new();
    extractor
        .populate_graph(&[extraction], &mut builder)
        .expect("populate");
    let (nodes, _) = builder.into_graph();
    assert!(
        nodes.iter().any(|n| {
            n.node_type == NodeType::Variable
                && n.name == "metadata.author"
                && n.get_property("kind") == Some("frontmatter")
        }),
        "frontmatter Variable node in graph"
    );
}

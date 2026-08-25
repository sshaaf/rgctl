//! Spec-matrix integration tests — maps to `docs/superpowers/specs/2026-08-17-markdown-context-graph-design.md` Testing section.

use rgctl_extraction::discovery::DiscoveryConfig;
use rgctl_extraction::extractor::Extractor;
use rgctl_extraction::graph_builder::GraphBuilder;
use rgctl_gql::executor::QueryExecutor;
use rgctl_gql::parser::parse;
use rgctl_graph::backend::MemoryBackend;
use rgctl_graph::schema::{EdgeType, NodeType};
use std::path::PathBuf;
use std::sync::Arc;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/markdown-context")
}

fn populate(languages: &[&str]) -> MemoryBackend {
    let root = fixture_root();
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);
    let config = DiscoveryConfig {
        languages: Some(languages.iter().map(|s| s.to_string()).collect()),
        ..DiscoveryConfig::default()
    };
    let extractions = extractor
        .extract_repository(&root, &config)
        .expect("extract");
    let mut builder = GraphBuilder::new();
    extractor
        .populate_graph(&extractions, &mut builder)
        .expect("populate");
    let (nodes, edges) = builder.into_graph();
    let mut backend = MemoryBackend::new();
    backend.insert_nodes_batch(nodes).expect("nodes");
    backend.insert_edges_batch(edges).expect("edges");
    backend
}

fn count(backend: &MemoryBackend, query: &str) -> usize {
    let q = parse(query).expect("parse");
    QueryExecutor::new(backend)
        .execute(&q)
        .expect("execute")
        .rows
        .len()
}

/// GQL queries 1–5 (Phase 2a).
#[test]
fn spec_phase2a_all_five_gql_queries_non_empty() {
    let backend = populate(&["markdown"]);
    let queries = [
        "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n",
        "MATCH (a:Module)-[:CONTAINS]->(b:Module) WHERE a.kind = 'heading' AND b.kind = 'heading' RETURN a, b",
        "MATCH (h:Module)-[:REFERENCES]->(f:File) WHERE h.kind = 'heading' AND f.name LIKE '*adr.md' RETURN h, f",
        "MATCH (h:Module)-[:REFERENCES]->(t:Module) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND t.kind = 'heading' RETURN h, t",
        "MATCH (h:Module)-[:CONTAINS*1..3]->(n:Module) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND n.kind = 'heading' RETURN h, n",
    ];
    for (i, q) in queries.iter().enumerate() {
        assert!(count(&backend, q) > 0, "query {} must return rows", i + 1);
    }
}

/// GQL query 6 (Phase 2b).
#[test]
fn spec_phase2b_query6_checkout_service_class() {
    let backend = populate(&["markdown", "java"]);
    assert_eq!(
        count(
            &backend,
            "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' RETURN h, f, c",
        ),
        1,
        "query 6: single Checkout Flow -> CheckoutService.java -> CheckoutService row"
    );
}

#[test]
fn spec_references_to_adr_targets_file_node_id() {
    let backend = populate(&["markdown"]);
    let adr_id = backend
        .find_nodes_by_type(NodeType::File)
        .expect("files")
        .into_iter()
        .find(|n| n.name.ends_with("adr.md"))
        .expect("adr file")
        .id;
    assert!(
        backend
            .find_edges_by_type(EdgeType::References)
            .expect("refs")
            .iter()
            .any(|e| e.to == adr_id),
        "REFERENCES edge must target adr.md File node UUID"
    );
}

#[test]
fn spec_heading_qualified_names_and_kind_property() {
    let backend = populate(&["markdown"]);
    assert_eq!(
        count(
            &backend,
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.qualified_name LIKE '*#checkout-flow' RETURN n",
        ),
        1
    );
}

#[test]
fn spec_markdown_link_import_nodes_exist() {
    let backend = populate(&["markdown"]);
    assert!(
        count(
            &backend,
            "MATCH (i:Import) WHERE i.kind = 'markdown_link' RETURN i",
        ) >= 3,
        "guide links produce Import nodes"
    );
}

#[test]
fn spec_default_registry_extracts_fixture_readme() {
    let root = fixture_root();
    let readme = root.join("README.md");
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);
    let result = extractor.extract_file(&readme).expect("README.md");
    assert!(
        result.symbols.iter().any(|s| s.name == "metadata.scope"),
        "README frontmatter key"
    );
    assert!(
        result
            .relations
            .iter()
            .any(|r| { r.relation_type == rgctl_plugin_api::RelationType::Defines })
            || result
                .symbols
                .iter()
                .any(|s| s.name.contains("Markdown context")),
        "README headings or structure"
    );
}

#[test]
fn spec_full_fixture_defines_and_references_edges() {
    let root = fixture_root();
    let registry = Arc::new(rgctl_languages::default_registry());
    let extractor = Extractor::new(registry);
    let config = DiscoveryConfig {
        languages: Some(vec!["markdown".to_string()]),
        ..DiscoveryConfig::default()
    };
    let extractions = extractor
        .extract_repository(&root, &config)
        .expect("extract");
    let defines: usize = extractions
        .iter()
        .flat_map(|e| e.relations.iter())
        .filter(|r| r.relation_type == rgctl_plugin_api::RelationType::Defines)
        .count();
    let references: usize = extractions
        .iter()
        .flat_map(|e| e.relations.iter())
        .filter(|r| r.relation_type == rgctl_plugin_api::RelationType::References)
        .count();
    assert!(
        defines >= 4,
        "nested headings across fixture, got {defines}"
    );
    assert!(
        references >= 5,
        "cross-doc links in fixture, got {references}"
    );
}

#[test]
fn spec_java_file_not_indexed_markdown_only_discover() {
    let backend = populate(&["markdown"]);
    assert_eq!(
        count(
            &backend,
            "MATCH (c:Class) WHERE c.name = 'CheckoutService' RETURN c"
        ),
        0
    );
}

#[test]
fn spec_phase2_footprint_snapshot_and_node_counts() {
    use rgctl_graph::write_columnar_from_backend;

    let markdown_backend = populate(&["markdown"]);
    let java_backend = populate(&["java"]);

    let md_nodes = markdown_backend.node_count();
    let java_nodes = java_backend.node_count();
    assert!(
        md_nodes > java_nodes,
        "markdown fixture nodes {md_nodes} vs java-only {java_nodes}"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let md_path = dir.path().join("markdown.snapshot.bin");
    let java_path = dir.path().join("java.snapshot.bin");
    write_columnar_from_backend(&markdown_backend, &md_path).expect("write markdown snapshot");
    write_columnar_from_backend(&java_backend, &java_path).expect("write java snapshot");
    let md_bytes = std::fs::metadata(&md_path).expect("md meta").len();
    let java_bytes = std::fs::metadata(&java_path).expect("java meta").len();
    assert!(
        md_bytes > java_bytes,
        "markdown snapshot {md_bytes} B vs java-only {java_bytes} B"
    );
}

#[test]
fn spec_phase2_link_import_node_inflation() {
    let backend = populate(&["markdown"]);
    let link_imports = count(
        &backend,
        "MATCH (i:Import) WHERE i.kind = 'markdown_link' RETURN i",
    );
    let guide_headings = count(
        &backend,
        "MATCH (n:Module) WHERE n.kind = 'heading' AND n.file_path LIKE '*guide.md' RETURN n",
    );
    assert!(link_imports >= 3, "per-link Import symbols");
    assert!(guide_headings >= 3, "guide.md headings");
    assert!(
        link_imports >= guide_headings,
        "link-heavy docs inflate nodes (imports {link_imports} vs guide headings {guide_headings})"
    );
}

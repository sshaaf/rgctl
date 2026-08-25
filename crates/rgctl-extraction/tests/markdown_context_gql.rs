//! GQL integration tests for markdown context graph (Phase 2a / 2b).

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

fn populate_fixture(languages: &[&str]) -> MemoryBackend {
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

fn row_count(backend: &MemoryBackend, query: &str) -> usize {
    let parsed = parse(query).expect("parse query");
    QueryExecutor::new(backend)
        .execute(&parsed)
        .expect("execute")
        .rows
        .len()
}

fn first_binding_name(backend: &MemoryBackend, query: &str, var: &str) -> Option<String> {
    let parsed = parse(query).expect("parse");
    let result = QueryExecutor::new(backend)
        .execute(&parsed)
        .expect("execute");
    result
        .rows
        .first()
        .and_then(|row| row.get(var).map(|n| n.name.to_string()))
}

#[test]
fn markdown_phase2a_query1_checkout_headings() {
    let backend = populate_fixture(&["markdown"]);
    assert_eq!(
        row_count(
            &backend,
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n",
        ),
        1,
        "exactly one Checkout* heading"
    );
    assert_eq!(
        first_binding_name(
            &backend,
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' RETURN n",
            "n",
        ),
        Some("Checkout Flow".to_string())
    );
    assert!(
        row_count(
            &backend,
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name LIKE 'Checkout*' AND n.body_text LIKE 'End-to-end*' RETURN n",
        ) >= 1,
        "checkout section body_text queryable via GQL"
    );
}

#[test]
fn markdown_phase2a_query2_heading_contains_tree() {
    let backend = populate_fixture(&["markdown"]);
    assert!(
        row_count(
            &backend,
            "MATCH (a:Module)-[:CONTAINS]->(b:Module) WHERE a.name = 'Checkout Flow' AND b.name = 'Cart' RETURN a, b",
        ) >= 1,
        "Checkout Flow CONTAINS Cart"
    );
}

#[test]
fn markdown_phase2a_query3_doc_references_adr_file() {
    let backend = populate_fixture(&["markdown"]);
    assert!(
        row_count(
            &backend,
            "MATCH (h:Module)-[:REFERENCES]->(f:File) WHERE h.kind = 'heading' AND f.name LIKE '*adr.md' RETURN h, f",
        ) >= 1,
        "query 3: heading REFERENCES adr.md file"
    );

    let adr_file = backend
        .find_nodes_by_type(NodeType::File)
        .expect("files")
        .into_iter()
        .find(|n| n.name.ends_with("adr.md"))
        .expect("adr.md file node")
        .id;
    let has_ref_to_adr = backend
        .find_edges_by_type(EdgeType::References)
        .expect("refs")
        .iter()
        .any(|edge| edge.to == adr_file);
    assert!(has_ref_to_adr, "References edge targets adr.md File UUID");
}

#[test]
fn markdown_phase2a_query4_checkout_to_payments_heading() {
    let backend = populate_fixture(&["markdown"]);
    assert!(
        row_count(
            &backend,
            "MATCH (h:Module)-[:REFERENCES]->(t:Module) WHERE h.name = 'Checkout Flow' AND t.name = 'Payments' RETURN h, t",
        ) >= 1,
        "Checkout Flow REFERENCES Payments heading"
    );
}

#[test]
fn markdown_phase2a_query5_checkout_subtree() {
    let backend = populate_fixture(&["markdown"]);
    let rows = row_count(
        &backend,
        "MATCH (h:Module)-[:CONTAINS*1..3]->(n:Module) WHERE h.name = 'Checkout Flow' AND n.kind = 'heading' RETURN h, n",
    );
    assert!(
        rows >= 3,
        "subtree includes Cart and nested sections, got {rows}"
    );
    assert_eq!(
        first_binding_name(
            &backend,
            "MATCH (h:Module)-[:CONTAINS*1..3]->(n:Module) WHERE h.name = 'Checkout Flow' AND n.name = 'Cart' RETURN h, n",
            "n",
        ),
        Some("Cart".to_string())
    );
}

#[test]
fn markdown_phase2a_readme_frontmatter_in_graph() {
    let backend = populate_fixture(&["markdown"]);
    assert!(
        row_count(
            &backend,
            "MATCH (v:Variable) WHERE v.kind = 'frontmatter' AND v.name = 'metadata.author' RETURN v",
        ) >= 1,
        "README metadata.author Variable node"
    );
}

#[test]
fn markdown_phase2a_mdx_heading_indexed() {
    let backend = populate_fixture(&["markdown"]);
    assert!(
        row_count(
            &backend,
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name = 'MDX overview' RETURN n",
        ) == 1,
        "overview.mdx heading"
    );
}

#[test]
fn markdown_phase2a_file_path_filter_guide() {
    let backend = populate_fixture(&["markdown"]);
    assert!(
        row_count(
            &backend,
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.file_path LIKE '*guide.md' AND n.name = 'Checkout Flow' RETURN n",
        ) == 1,
        "file_path suffix matches guide.md"
    );
}

#[test]
fn markdown_language_filter_excludes_markdown_when_java_only() {
    let backend = populate_fixture(&["java"]);
    assert_eq!(
        row_count(
            &backend,
            "MATCH (n:Module) WHERE n.kind = 'heading' AND n.name = 'Checkout Flow' RETURN n",
        ),
        0,
        "java-only discover must not index markdown headings"
    );
}

#[test]
fn markdown_phase2b_doc_to_class_query() {
    let backend = populate_fixture(&["markdown", "java"]);

    assert!(
        row_count(
            &backend,
            "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.kind = 'heading' AND h.name LIKE 'Checkout*' AND f.name LIKE '*CheckoutService.java' RETURN h, f, c",
        ) >= 1,
        "query 6: Checkout Flow -> CheckoutService.java -> CheckoutService"
    );

    let class = backend
        .find_nodes_by_type(NodeType::Class)
        .expect("classes")
        .into_iter()
        .find(|n| n.name == "CheckoutService")
        .expect("CheckoutService class");
    assert_eq!(
        first_binding_name(
            &backend,
            "MATCH (h:Module)-[:REFERENCES]->(f:File)-[:CONTAINS]->(c:Class) WHERE h.name LIKE 'Checkout*' AND c.name = 'CheckoutService' RETURN c",
            "c",
        ),
        Some("CheckoutService".to_string())
    );
    let _ = class.id;
}

#[test]
fn markdown_indexes_more_nodes_than_java_only_on_same_tree() {
    let markdown_backend = populate_fixture(&["markdown"]);
    let java_only = populate_fixture(&["java"]);

    let markdown_modules = markdown_backend
        .find_nodes_by_type(NodeType::Module)
        .expect("modules")
        .len();
    let java_only_modules = java_only
        .find_nodes_by_type(NodeType::Module)
        .expect("modules")
        .len();

    assert!(
        markdown_modules > java_only_modules,
        "markdown discover adds doc Module nodes (markdown={markdown_modules}, java-only={java_only_modules})"
    );
}

//! Phase 14: GraphML export tests.

use rgctl::export::export_graphml;
use rgctl::graph::backend::GraphBackend;
use rgctl::graph::schema::{Edge, EdgeType, Node, NodeType};

#[test]
fn test_graphml_header_and_keys() {
    let mut backend = rgctl::graph::backend::MemoryBackend::new();
    backend
        .insert_node(Node::new(NodeType::Function, "main").with_file_path("src/main.rs"))
        .unwrap();

    let xml = export_graphml(&backend, "all").unwrap();
    assert!(xml.contains("<?xml version=\"1.0\""));
    assert!(xml.contains("graphml"));
    assert!(xml.contains(r#"<key id="name""#));
    assert!(xml.contains(r#"<key id="complexity""#));
}

#[test]
fn test_graphml_node_data() {
    let mut backend = rgctl::graph::backend::MemoryBackend::new();
    let node = Node::new(NodeType::Function, "main").with_property("cyclomatic".into(), "5".into());
    backend.insert_node(node).unwrap();

    let xml = export_graphml(&backend, "functions").unwrap();
    assert!(xml.contains("<data key=\"name\">main</data>"));
    assert!(xml.contains("<data key=\"complexity\">5</data>"));
}

#[test]
fn test_graphml_edges() {
    let mut backend = rgctl::graph::backend::MemoryBackend::new();
    let a = Node::new(NodeType::Function, "a");
    let b = Node::new(NodeType::Function, "b");
    let id_a = a.id;
    let id_b = b.id;
    backend.insert_node(a).unwrap();
    backend.insert_node(b).unwrap();
    backend
        .insert_edge(Edge::new(id_a, id_b, EdgeType::Calls))
        .unwrap();

    let xml = export_graphml(&backend, "all").unwrap();
    assert!(xml.contains("<edge id="));
    assert!(xml.contains("Calls"));
}

#[test]
fn test_graphml_xml_escapes_ampersand() {
    let mut backend = rgctl::graph::backend::MemoryBackend::new();
    backend
        .insert_node(Node::new(NodeType::Class, "A&B"))
        .unwrap();
    let xml = export_graphml(&backend, "all").unwrap();
    assert!(xml.contains("A&amp;B"));
}

#[test]
fn test_graphml_empty_query_errors() {
    let backend = rgctl::graph::backend::MemoryBackend::new();
    let err = export_graphml(&backend, "name:missing").unwrap_err();
    assert!(err.to_string().contains("No nodes matched"));
}

//! Columnar snapshot and adjacency micro-benchmarks.
//!
//! Run: `cargo bench -p rgctl-graph --bench columnar_snapshot`

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use memmap2::Mmap;
use rgctl_graph::backend::GraphBackend;
use rgctl_graph::backend::MemoryBackend;
use rgctl_graph::columnar_snapshot::ColumnarGraphMmap;
use rgctl_graph::schema::{Edge, EdgeType, Node, NodeType};
use rgctl_graph::write_columnar_from_nodes_edges;
use std::fs::File;
use std::sync::Arc;
use tempfile::TempDir;

fn build_dense_backend(nodes: usize, edges_per_node: usize) -> MemoryBackend {
    let mut backend = MemoryBackend::new();
    let mut ids = Vec::with_capacity(nodes);
    for i in 0..nodes {
        let node = Node::new(NodeType::Function, format!("fn{i}"));
        ids.push(node.id);
        backend.insert_node(node).unwrap();
    }
    for (i, from) in ids.iter().enumerate() {
        for j in 0..edges_per_node {
            let to = ids[(i + j + 1) % nodes];
            backend
                .insert_edge(Edge::new(*from, to, EdgeType::Calls))
                .unwrap();
        }
    }
    backend
}

fn build_columnar_snapshot(nodes: usize, edges: usize) -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bench.bin");
    let mut graph_nodes = Vec::with_capacity(nodes);
    let mut graph_edges = Vec::with_capacity(edges);
    let mut ids = Vec::with_capacity(nodes);
    for i in 0..nodes {
        let node = Node::new(NodeType::Function, format!("fn{i}"));
        ids.push(node.id);
        graph_nodes.push(node);
    }
    for e in 0..edges {
        let from = ids[e % nodes];
        let to = ids[(e * 7 + 3) % nodes];
        graph_edges.push(Edge::new(from, to, EdgeType::Calls));
    }
    write_columnar_from_nodes_edges(graph_nodes, graph_edges, &path).unwrap();
    (tmp, path)
}

fn bench_edge_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_lookup");
    for (nodes, degree) in [(5_000, 8), (20_000, 16)] {
        let backend = build_dense_backend(nodes, degree);
        let hub = backend.find_nodes_by_name("fn0").unwrap().pop().unwrap().id;
        group.bench_with_input(
            BenchmarkId::new("outgoing_adj", format!("{nodes}n_{degree}d")),
            &hub,
            |b, &hub| {
                b.iter(|| black_box(backend.get_outgoing_edges(hub).unwrap().len()));
            },
        );
    }
    group.finish();
}

fn bench_snapshot_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_open");
    for nodes in [10_000, 50_000] {
        let edges = nodes * 4;
        let (_tmp, path) = build_columnar_snapshot(nodes, edges);
        group.bench_with_input(
            BenchmarkId::new("metadata_only", nodes),
            &path,
            |b, path| {
                b.iter(|| {
                    let file = File::open(path).unwrap();
                    let mmap = Arc::new(unsafe { Mmap::map(&file).unwrap() });
                    let col = ColumnarGraphMmap::open(mmap).unwrap();
                    black_box((col.node_count(), col.edge_count()))
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("first_get_node", nodes),
            &path,
            |b, path| {
                b.iter(|| {
                    let file = File::open(path).unwrap();
                    let mmap = Arc::new(unsafe { Mmap::map(&file).unwrap() });
                    let col = ColumnarGraphMmap::open(mmap).unwrap();
                    let id = col.materialize_node_at(0).unwrap().id;
                    black_box(col.get_node(id).unwrap().unwrap().name.to_string())
                });
            },
        );
    }
    group.finish();
}

fn bench_has_edge(c: &mut Criterion) {
    let backend = build_dense_backend(10_000, 4);
    let from = backend.find_nodes_by_name("fn0").unwrap().pop().unwrap().id;
    let to = backend.find_nodes_by_name("fn1").unwrap().pop().unwrap().id;
    c.bench_function("has_edge_10k", |b| {
        b.iter(|| black_box(backend.has_edge(from, to, EdgeType::Calls)));
    });
}

criterion_group!(
    benches,
    bench_edge_lookup,
    bench_snapshot_open,
    bench_has_edge,
);
criterion_main!(benches);

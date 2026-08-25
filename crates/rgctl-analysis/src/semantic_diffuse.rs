//! Index-time Jacobi diffusion over call-graph dense embeddings.

use crate::callgraph::CallGraph;
use rayon::prelude::*;

/// Neighbor aggregation mode for diffusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffuseNeighborMode {
    /// Mean of callees only (`success_list`).
    #[default]
    Callees,
    /// Mean of callers and callees (union, averaged).
    Bidirectional,
}

/// Parameters for [`diffuse_call_topology`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffuseConfig {
    /// Blend weight toward neighbor mean (`0` = identity).
    pub alpha: f64,
    /// Jacobi iterations.
    pub iterations: usize,
    /// Neighbor set.
    pub mode: DiffuseNeighborMode,
}

impl Default for DiffuseConfig {
    fn default() -> Self {
        Self {
            alpha: 0.25,
            iterations: 2,
            mode: DiffuseNeighborMode::Callees,
        }
    }
}

impl DiffuseConfig {
    /// True when diffusion would change vectors.
    pub fn is_active(&self) -> bool {
        self.iterations > 0 && self.alpha > 0.0
    }
}

/// In-place Jacobi diffusion on a flat `f32` matrix `[n_fn * dims]` aligned to `call_graph`.
pub fn diffuse_call_topology(
    call_graph: &CallGraph,
    matrix: &mut [f32],
    dims: usize,
    config: DiffuseConfig,
) {
    let n = call_graph.function_count();
    if n == 0 || dims == 0 || !config.is_active() {
        return;
    }
    assert_eq!(
        matrix.len(),
        n * dims,
        "dense matrix must be CallGraph-sized"
    );

    let mut current = matrix.to_vec();
    let mut next = vec![0.0f32; n * dims];
    let alpha = config.alpha as f32;
    let keep = 1.0 - alpha;

    // Build bidirectional neighbor lists once (topology is immutable across iterations).
    let bidirectional_neighbors: Option<Vec<Vec<u32>>> =
        if matches!(config.mode, DiffuseNeighborMode::Bidirectional) {
            Some(
                (0..n)
                    .map(|node_idx| {
                        let mut ids = Vec::new();
                        ids.extend_from_slice(&call_graph.success_list[node_idx]);
                        ids.extend_from_slice(&call_graph.precursor_list[node_idx]);
                        ids.sort_unstable();
                        ids.dedup();
                        ids
                    })
                    .collect(),
            )
        } else {
            None
        };

    for _ in 0..config.iterations {
        next.par_chunks_mut(dims)
            .enumerate()
            .for_each(|(node_idx, next_row)| {
                next_row.fill(0.0);
                let local = &current[node_idx * dims..(node_idx + 1) * dims];

                let neighbor_idxs: &[u32] = match config.mode {
                    DiffuseNeighborMode::Callees => &call_graph.success_list[node_idx],
                    DiffuseNeighborMode::Bidirectional => {
                        bidirectional_neighbors.as_ref().unwrap()[node_idx].as_slice()
                    }
                };

                if neighbor_idxs.is_empty() {
                    next_row.copy_from_slice(local);
                    return;
                }

                let mut sum = vec![0.0f32; dims];
                for &nbr in neighbor_idxs {
                    let start = nbr as usize * dims;
                    let row = &current[start..start + dims];
                    for d in 0..dims {
                        sum[d] += row[d];
                    }
                }
                let inv = 1.0 / neighbor_idxs.len() as f32;
                for d in 0..dims {
                    next_row[d] = keep * local[d] + alpha * (sum[d] * inv);
                }
            });
        std::mem::swap(&mut current, &mut next);
    }

    current.par_chunks_mut(dims).for_each(l2_normalize_slice);
    matrix.copy_from_slice(&current);
}

fn l2_normalize_slice(vec: &mut [f32]) {
    let sum_sq: f32 = vec.iter().map(|x| x * x).sum();
    if sum_sq > 0.0 {
        let norm = sum_sq.sqrt();
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callgraph::CallGraph;
    use rgctl_graph::backend::{GraphBackend, MemoryBackend};
    use rgctl_graph::schema::{Edge, EdgeType, Node, NodeType};

    fn tiny_call_backend() -> MemoryBackend {
        let mut backend = MemoryBackend::new();
        let a = Node::new(NodeType::Function, "a");
        let b = Node::new(NodeType::Function, "b");
        let c = Node::new(NodeType::Function, "c");
        let a_id = a.id;
        let b_id = b.id;
        let c_id = c.id;
        backend.insert_node(a).unwrap();
        backend.insert_node(b).unwrap();
        backend.insert_node(c).unwrap();
        backend
            .insert_edge(Edge::new(a_id, b_id, EdgeType::Calls))
            .unwrap();
        backend
            .insert_edge(Edge::new(b_id, c_id, EdgeType::Calls))
            .unwrap();
        backend
    }

    #[test]
    fn alpha_zero_is_noop() {
        let backend = tiny_call_backend();
        let cg = CallGraph::from_backend(&backend).unwrap();
        let dims = 4;
        let mut matrix = vec![0.0f32; cg.function_count() * dims];
        for (i, slot) in matrix.iter_mut().enumerate() {
            *slot = (i as f32) + 1.0;
        }
        let before = matrix.clone();
        diffuse_call_topology(
            &cg,
            &mut matrix,
            dims,
            DiffuseConfig {
                alpha: 0.0,
                iterations: 3,
                ..Default::default()
            },
        );
        assert_eq!(matrix, before);
    }

    #[test]
    fn isolate_keeps_local_after_normalize() {
        let mut backend = MemoryBackend::new();
        let lonely = Node::new(NodeType::Function, "lonely");
        backend.insert_node(lonely).unwrap();
        let cg = CallGraph::from_backend(&backend).unwrap();
        let dims = 4;
        let mut matrix = vec![3.0f32, 0.0, 0.0, 0.0];
        diffuse_call_topology(
            &cg,
            &mut matrix,
            dims,
            DiffuseConfig {
                alpha: 0.5,
                iterations: 2,
                ..Default::default()
            },
        );
        // Isolate path copies local then final L2 normalize.
        let norm = (3.0f32 * 3.0).sqrt();
        assert!((matrix[0] - 3.0 / norm).abs() < 1e-5);
        assert!(matrix[1].abs() < 1e-6);
    }

    #[test]
    fn diffusion_moves_caller_toward_callee() {
        let backend = tiny_call_backend();
        let cg = CallGraph::from_backend(&backend).unwrap();
        let dims = 2;
        let n = cg.function_count();
        let mut matrix = vec![0.0f32; n * dims];
        // Put distinct one-hots on each function in CallGraph order.
        for i in 0..n {
            matrix[i * dims] = if i == 0 { 1.0 } else { 0.0 };
            matrix[i * dims + 1] = if i == 0 { 0.0 } else { 1.0 };
        }
        let before_caller = matrix[0..dims].to_vec();
        diffuse_call_topology(
            &cg,
            &mut matrix,
            dims,
            DiffuseConfig {
                alpha: 0.5,
                iterations: 1,
                mode: DiffuseNeighborMode::Callees,
            },
        );
        assert_ne!(&matrix[0..dims], before_caller.as_slice());
    }
}

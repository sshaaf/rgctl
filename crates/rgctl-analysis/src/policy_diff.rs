//! Temporal policy classification across base/head snapshots.

use crate::blast_radius_scc::BlastRadiusEngine;
use crate::centrality::CentralityScores;
use crate::policy::{PolicyRegistry, PolicyViolation, check_policies};
use crate::violation_ledger::{ViolationLedger, ledger_entry_from_delta};
use rgctl_error::Result;
use rgctl_graph::SnapshotNodeStore;
use rgctl_graph::schema::NodeType;
use rgctl_graph::stable_key::{StableNodeKey, node_row_ref, stable_key_from_row};
use std::collections::HashMap;
use uuid::Uuid;

/// Whether a violation is new, pre-existing, or resolved in the PR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalClass {
    /// Head violates policy; base did not.
    New,
    /// Both snapshots violate policy.
    Existing,
    /// Base violated policy; head is clean.
    Resolved,
    /// Head violates policy after a prior ledger resolution for this stable key.
    Regression,
}

impl std::fmt::Display for TemporalClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "new"),
            Self::Existing => write!(f, "existing"),
            Self::Resolved => write!(f, "resolved"),
            Self::Regression => write!(f, "regression"),
        }
    }
}

/// Policy outcome for one scoped entity.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyDelta {
    /// Stable entity identity across snapshots.
    pub key: StableNodeKey,
    /// Human-readable symbol name.
    pub symbol: String,
    /// Temporal classification for this violation.
    pub classification: TemporalClass,
    /// Policy breach details (from head, or base when resolved).
    pub violation: PolicyViolation,
}

/// Evaluate policy on scoped entities and classify violations temporally.
pub fn evaluate_temporal(
    base_store: &SnapshotNodeStore,
    head_store: &SnapshotNodeStore,
    base_backend: &rgctl_graph::backend::MemoryBackend,
    head_backend: &rgctl_graph::backend::MemoryBackend,
    head_engine: &BlastRadiusEngine,
    base_engine: &BlastRadiusEngine,
    scope_keys: &[StableNodeKey],
    registry: &PolicyRegistry,
    centrality: &HashMap<Uuid, CentralityScores>,
) -> Result<Vec<PolicyDelta>> {
    let base_index = stable_symbol_index(base_store)?;
    let head_index = stable_symbol_index(head_store)?;
    let mut deltas = Vec::new();

    for key in scope_keys {
        let head_entry = head_index.get(key);
        let base_entry = base_index.get(key);
        let head_violation = head_entry
            .and_then(|(id, _)| violation_for(head_engine, *id, head_backend, registry, centrality));
        let base_violation = base_entry
            .and_then(|(id, _)| violation_for(base_engine, *id, base_backend, registry, centrality));

        let classification = match classify_temporal_pair(
            head_violation.is_some(),
            base_violation.is_some(),
        ) {
            Some(classification) => classification,
            None => continue,
        };

        let violation = head_violation.or(base_violation).expect("classified violation");
        let symbol = head_entry
            .or(base_entry)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| key.as_u64().to_string());

        deltas.push(PolicyDelta {
            key: *key,
            symbol,
            classification,
            violation,
        });
    }

    Ok(deltas)
}

/// Reclassify snapshot-`new` violations as `regression` when the ledger shows prior resolution.
pub fn apply_ledger_regression(deltas: &mut [PolicyDelta], ledger: &ViolationLedger) {
    for delta in deltas.iter_mut() {
        if delta.classification != TemporalClass::New {
            continue;
        }
        let rule = delta.violation.rule_id();
        if ledger.was_resolved(delta.key.as_u64(), rule) {
            delta.classification = TemporalClass::Regression;
        }
    }
}

/// Append current temporal outcomes to the violation ledger.
pub fn record_deltas_to_ledger(
    ledger: &mut ViolationLedger,
    deltas: &[PolicyDelta],
    commit: &str,
) -> rgctl_error::Result<()> {
    for delta in deltas {
        let entry = ledger_entry_from_delta(
            delta.key.as_u64(),
            delta.violation.rule_id(),
            delta.classification,
            commit,
            Some(&delta.symbol),
        );
        ledger.append(entry)?;
    }
    Ok(())
}

fn classify_temporal_pair(
    head_violates: bool,
    base_violates: bool,
) -> Option<TemporalClass> {
    match (head_violates, base_violates) {
        (true, false) => Some(TemporalClass::New),
        (true, true) => Some(TemporalClass::Existing),
        (false, true) => Some(TemporalClass::Resolved),
        (false, false) => None,
    }
}

fn violation_for(
    engine: &BlastRadiusEngine,
    node_id: Uuid,
    backend: &rgctl_graph::backend::MemoryBackend,
    registry: &PolicyRegistry,
    centrality: &HashMap<Uuid, CentralityScores>,
) -> Option<PolicyViolation> {
    let result = engine.analyze(node_id).ok()?;
    check_policies(
        node_id,
        &result.impact_zone_ids,
        registry,
        backend,
        Some(centrality),
    )
    .err()
}

fn stable_symbol_index(
    store: &SnapshotNodeStore,
) -> Result<HashMap<StableNodeKey, (Uuid, String)>> {
    let col = store
        .columnar()
        .ok_or_else(|| rgctl_error::Error::GraphError("temporal policy requires columnar snapshot".into()))?;
    let mut index = HashMap::with_capacity(col.node_count());
    for idx in 0..col.node_count() {
        let row = node_row_ref(col, idx)?;
        if row.node_type != NodeType::Function {
            continue;
        }
        let key = stable_key_from_row(col, idx)?;
        let name = col
            .materialize_node_at(idx)?
            .name
            .to_string();
        index.insert(key, (row.id, name));
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::centrality::CentralityAnalyzer;
    use crate::PetGraphView;
    use rgctl_graph::backend::GraphBackend;
    use rgctl_graph::schema::{Edge, EdgeType, Node, NodeType};
    use rgctl_graph::write_columnar_from_nodes_edges;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn temporal_pair_classification_matrix() {
        assert_eq!(
            classify_temporal_pair(true, false),
            Some(TemporalClass::New)
        );
        assert_eq!(
            classify_temporal_pair(true, true),
            Some(TemporalClass::Existing)
        );
        assert_eq!(
            classify_temporal_pair(false, true),
            Some(TemporalClass::Resolved)
        );
        assert_eq!(classify_temporal_pair(false, false), None);
    }

    fn write_target_snapshot(path: &Path, caller_count: usize) {
        let mut backend = rgctl_graph::backend::MemoryBackend::new();
        let target = Node::new(NodeType::Function, "target_fn").with_file_path("src/pkg.rs");
        let target_id = target.id;
        backend.insert_node(target).unwrap();
        for i in 0..caller_count {
            let caller =
                Node::new(NodeType::Function, format!("caller{i}")).with_file_path("src/pkg.rs");
            let caller_id = caller.id;
            backend.insert_node(caller).unwrap();
            backend
                .insert_edge(Edge::new(caller_id, target_id, EdgeType::Calls))
                .unwrap();
        }
        write_columnar_from_nodes_edges(
            backend.all_nodes().unwrap(),
            backend.all_edges().unwrap(),
            path,
        )
        .unwrap();
    }

    fn target_stable_key(store: &SnapshotNodeStore) -> StableNodeKey {
        let col = store.columnar().unwrap();
        for idx in 0..col.node_count() {
            let node = col.materialize_node_at(idx).unwrap();
            if node.node_type == NodeType::Function && node.name.as_str() == "target_fn" {
                return stable_key_from_row(col, idx).unwrap();
            }
        }
        panic!("target_fn not found");
    }

    struct TemporalFixture {
        tmp: TempDir,
        base_store: SnapshotNodeStore,
        head_store: SnapshotNodeStore,
        base_backend: rgctl_graph::backend::MemoryBackend,
        head_backend: rgctl_graph::backend::MemoryBackend,
        head_engine: BlastRadiusEngine,
        base_engine: BlastRadiusEngine,
        centrality: HashMap<Uuid, CentralityScores>,
        key: StableNodeKey,
    }

    fn temporal_fixture(base_leaves: usize, head_leaves: usize) -> TemporalFixture {
        let tmp = TempDir::new().unwrap();
        let base_path = tmp.path().join("base.bin");
        let head_path = tmp.path().join("head.bin");
        write_target_snapshot(&base_path, base_leaves);
        write_target_snapshot(&head_path, head_leaves);

        let base_store = SnapshotNodeStore::open(&base_path).unwrap();
        let head_store = SnapshotNodeStore::open(&head_path).unwrap();
        let base_backend = base_store.hydrate_backend().unwrap();
        let head_backend = head_store.hydrate_backend().unwrap();
        let view = PetGraphView::from_backend(&head_backend).unwrap();
        let centrality = CentralityAnalyzer::new()
            .analyze_with_view(&view)
            .unwrap()
            .scores;
        let head_engine = BlastRadiusEngine::build(&head_backend).unwrap();
        let base_engine = BlastRadiusEngine::build(&base_backend).unwrap();
        let key = target_stable_key(&head_store);

        TemporalFixture {
            tmp,
            base_store,
            head_store,
            base_backend,
            head_backend,
            head_engine,
            base_engine,
            centrality,
            key,
        }
    }

    fn registry_scale_limit(max: usize) -> PolicyRegistry {
        let mut registry = PolicyRegistry::permissive();
        registry.max_impact_nodes = max;
        registry
    }

    fn run_temporal(
        fixture: &TemporalFixture,
        registry: &PolicyRegistry,
    ) -> Vec<PolicyDelta> {
        evaluate_temporal(
            &fixture.base_store,
            &fixture.head_store,
            &fixture.base_backend,
            &fixture.head_backend,
            &fixture.head_engine,
            &fixture.base_engine,
            &[fixture.key],
            registry,
            &fixture.centrality,
        )
        .unwrap()
    }

    #[test]
    fn temporal_new_violation_when_head_exceeds_limit() {
        let fixture = temporal_fixture(0, 6);
        let deltas = run_temporal(&fixture, &registry_scale_limit(5));
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].classification, TemporalClass::New);
        assert_eq!(deltas[0].symbol, "target_fn");
        assert!(matches!(
            deltas[0].violation,
            PolicyViolation::ScaleFailure { .. }
        ));
    }

    #[test]
    fn temporal_existing_when_both_snapshots_violate() {
        let fixture = temporal_fixture(6, 6);
        let deltas = run_temporal(&fixture, &registry_scale_limit(5));
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].classification, TemporalClass::Existing);
    }

    #[test]
    fn temporal_resolved_when_base_violates_head_clean() {
        let fixture = temporal_fixture(6, 0);
        let deltas = run_temporal(&fixture, &registry_scale_limit(5));
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].classification, TemporalClass::Resolved);
    }

    #[test]
    fn ledger_reclassifies_new_as_regression() {
        let fixture = temporal_fixture(0, 6);
        let registry = registry_scale_limit(5);
        let mut deltas = run_temporal(&fixture, &registry);
        assert_eq!(deltas[0].classification, TemporalClass::New);

        let mut ledger = crate::violation_ledger::ViolationLedger::in_memory();
        ledger
            .append(
                crate::violation_ledger::ledger_entry_from_delta(
                    deltas[0].key.as_u64(),
                    deltas[0].violation.rule_id(),
                    TemporalClass::Resolved,
                    "deadbeef",
                    Some(&deltas[0].symbol),
                ),
            )
            .unwrap();
        apply_ledger_regression(&mut deltas, &ledger);
        assert_eq!(deltas[0].classification, TemporalClass::Regression);
    }

    fn temporal_stable_key_matches_across_snapshot_uuids() {
        let fixture = temporal_fixture(3, 3);
        let base_key = target_stable_key(&fixture.base_store);
        let head_key = target_stable_key(&fixture.head_store);
        assert_eq!(base_key, head_key);
        let base_id = fixture.base_store.all_node_ids();
        let head_id = fixture.head_store.all_node_ids();
        assert_eq!(base_id, head_id, "deterministic node IDs align across snapshots");
    }
}

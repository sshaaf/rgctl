//! Reaching-definitions data-flow analysis.
//!
//! **Algorithm:** iterative gen/kill dataflow on the CFG worklist.
//! **Complexity:** O(b · d) blocks × definition-set size; gen/kill built in O(total defs).

use crate::cfg::{BasicBlock, BlockId, ControlFlowGraph};
use crate::pdg::ProgramDependenceGraph;
use bit_set::BitSet;
use std::collections::{HashMap, HashSet, VecDeque};

/// Result of reaching-definitions analysis.
#[derive(Debug, Clone, Default)]
pub struct ReachingDefs {
    /// Definitions reaching the entry of each block.
    pub in_set: HashMap<BlockId, HashSet<Definition>>,
    /// Definitions reaching the exit of each block.
    pub out_set: HashMap<BlockId, HashSet<Definition>>,
}

/// A definition of a variable at a specific statement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Definition {
    /// Variable name.
    pub variable: String,
    /// Block containing the definition.
    pub block: BlockId,
    /// Index of the defining statement within the block.
    pub statement_index: usize,
    /// PDG node id for the defining statement.
    pub pdg_node: uuid::Uuid,
}

struct DefCatalog {
    defs: Vec<Definition>,
    by_var: HashMap<String, Vec<usize>>,
}

impl DefCatalog {
    fn push(&mut self, def: Definition) -> usize {
        let idx = self.defs.len();
        self.by_var
            .entry(def.variable.clone())
            .or_default()
            .push(idx);
        self.defs.push(def);
        idx
    }
}

/// Compute reaching definitions for all blocks in `cfg`.
pub fn compute_reaching_definitions(
    cfg: &ControlFlowGraph,
    pdg: &ProgramDependenceGraph,
) -> ReachingDefs {
    let mut catalog = DefCatalog {
        defs: Vec::new(),
        by_var: HashMap::new(),
    };
    let mut block_local_defs: HashMap<BlockId, Vec<usize>> =
        HashMap::with_capacity(cfg.blocks.len());

    for (&block_id, block) in &cfg.blocks {
        let mut local = Vec::new();
        for def in collect_block_definitions(block, pdg) {
            let idx = catalog.push(def);
            local.push(idx);
        }
        block_local_defs.insert(block_id, local);
    }

    let mut gen_sets: HashMap<BlockId, BitSet> = HashMap::with_capacity(cfg.blocks.len());
    let mut kill_sets: HashMap<BlockId, BitSet> = HashMap::with_capacity(cfg.blocks.len());
    let mut in_set_bits: HashMap<BlockId, BitSet> = HashMap::with_capacity(cfg.blocks.len());
    let mut out_set_bits: HashMap<BlockId, BitSet> = HashMap::with_capacity(cfg.blocks.len());

    for &block_id in cfg.blocks.keys() {
        let local_defs = &block_local_defs[&block_id];

        let mut gen_b = BitSet::new();
        let mut defined_vars = HashSet::new();
        for &def_idx in local_defs.iter().rev() {
            let var = &catalog.defs[def_idx].variable;
            if defined_vars.insert(var.clone()) {
                gen_b.insert(def_idx);
            }
        }

        let mut kill_b = BitSet::new();
        for var in &defined_vars {
            if let Some(all_defs_of_var) = catalog.by_var.get(var) {
                for &def_idx in all_defs_of_var {
                    if !gen_b.contains(def_idx) {
                        kill_b.insert(def_idx);
                    }
                }
            }
        }

        gen_sets.insert(block_id, gen_b);
        kill_sets.insert(block_id, kill_b);
        in_set_bits.insert(block_id, BitSet::new());
        out_set_bits.insert(block_id, BitSet::new());
    }

    let mut worklist: VecDeque<BlockId> = cfg.blocks.keys().copied().collect();
    let mut on_worklist: HashSet<BlockId> = worklist.iter().copied().collect();

    while let Some(block_id) = worklist.pop_front() {
        on_worklist.remove(&block_id);

        let mut in_b = BitSet::new();
        for &pred in cfg.predecessors(block_id) {
            if let Some(pred_out) = out_set_bits.get(&pred) {
                in_b.union_with(pred_out);
            }
        }

        let gen_b = &gen_sets[&block_id];
        let kill_b = &kill_sets[&block_id];

        let mut out_b = gen_b.clone();
        for def_idx in in_b.iter() {
            if !kill_b.contains(def_idx) {
                out_b.insert(def_idx);
            }
        }

        if out_set_bits.get(&block_id) != Some(&out_b) {
            out_set_bits.insert(block_id, out_b);
            in_set_bits.insert(block_id, in_b);
            for &succ in cfg.successors(block_id) {
                if on_worklist.insert(succ) {
                    worklist.push_back(succ);
                }
            }
        } else {
            in_set_bits.insert(block_id, in_b);
        }
    }

    let bitset_to_hashset = |bits: &BitSet| -> HashSet<Definition> {
        bits.iter().map(|idx| catalog.defs[idx].clone()).collect()
    };

    let in_set = in_set_bits
        .into_iter()
        .map(|(block, bits)| (block, bitset_to_hashset(&bits)))
        .collect();
    let out_set = out_set_bits
        .into_iter()
        .map(|(block, bits)| (block, bitset_to_hashset(&bits)))
        .collect();

    ReachingDefs { in_set, out_set }
}

fn collect_block_definitions(block: &BasicBlock, pdg: &ProgramDependenceGraph) -> Vec<Definition> {
    let mut defs = Vec::new();
    for (idx, stmt) in block.statements.iter().enumerate() {
        if let Some(pdg_node) = pdg.find_node_by_block_and_line(block.id, stmt.line) {
            for var in &pdg_node.defined_vars {
                defs.push(Definition {
                    variable: var.clone(),
                    block: block.id,
                    statement_index: idx,
                    pdg_node: pdg_node.id,
                });
            }
        }
    }
    defs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg_for_function;
    use crate::pdg::ProgramDependenceGraph;

    #[test]
    fn test_reaching_definitions_merge() {
        let code = r#"
fn example(condition: bool) {
    let mut x = 1;
    if condition {
        x = 2;
    } else {
        x = 3;
    }
    let _y = x;
}
"#;
        let cfg = build_cfg_for_function("rust", code, "example").unwrap();
        let pdg = ProgramDependenceGraph::build(&cfg, code.as_bytes()).unwrap();
        let reaching = compute_reaching_definitions(&cfg, &pdg);

        let use_block = cfg
            .blocks
            .values()
            .find(|b| b.statements.iter().any(|s| s.text.contains("_y")))
            .expect("use block");

        let x_defs: Vec<_> = reaching.in_set[&use_block.id]
            .iter()
            .filter(|d| d.variable == "x")
            .collect();
        assert!(x_defs.len() >= 2);
    }

    #[test]
    fn test_reaching_definitions_diamond_merge() {
        let code = r#"
fn diamond(cond: bool) {
    let mut x = 1;
    if cond {
        x = 2;
    } else {
        x = 3;
    }
    let y = x;
    let _z = y;
}
"#;
        let cfg = build_cfg_for_function("rust", code, "diamond").unwrap();
        let pdg = ProgramDependenceGraph::build(&cfg, code.as_bytes()).unwrap();
        let reaching = compute_reaching_definitions(&cfg, &pdg);

        let merge_block = cfg
            .blocks
            .values()
            .find(|b| b.statements.iter().any(|s| s.text.contains("_z")))
            .expect("merge block");

        let x_defs: Vec<_> = reaching.in_set[&merge_block.id]
            .iter()
            .filter(|d| d.variable == "x")
            .collect();
        assert!(
            x_defs.len() >= 2,
            "diamond merge should see multiple reaching definitions for x"
        );
    }

    #[test]
    fn test_same_block_redefinition_masks_earlier_def() {
        let code = r#"
fn shadow() {
    let mut x = 1;
    x = 2;
    let _y = x;
}
"#;
        let cfg = build_cfg_for_function("rust", code, "shadow").unwrap();
        let pdg = ProgramDependenceGraph::build(&cfg, code.as_bytes()).unwrap();
        let reaching = compute_reaching_definitions(&cfg, &pdg);

        let mut saw_double_def_block = false;
        for (block_id, block) in &cfg.blocks {
            let local_x: Vec<_> = collect_block_definitions(block, &pdg)
                .into_iter()
                .filter(|d| d.variable == "x")
                .collect();
            if local_x.len() < 2 {
                continue;
            }
            saw_double_def_block = true;
            let out_x: Vec<_> = reaching.out_set[block_id]
                .iter()
                .filter(|d| d.variable == "x")
                .collect();
            assert_eq!(
                out_x.len(),
                1,
                "block with multiple assignments to x should gen only the last definition"
            );
        }
        assert!(
            saw_double_def_block,
            "expected cfg to place at least two x assignments in one basic block"
        );
    }
}

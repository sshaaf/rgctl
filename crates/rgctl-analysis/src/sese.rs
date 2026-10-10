//! CFG Single-Entry Single-Exit (SESE) hammock decomposition.
//!
//! Decomposes Control Flow Graphs into logical structural units (hammocks)
//! bounded by min and max statement bounds (default 3..=15).

use crate::cfg::{BasicBlock, BlockId, CfgEdgeType, ControlFlowGraph, Statement};
use crate::dominance::{DominatorTree, PostDominatorTree};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Default minimum statement count for a candidate SESE hammock.
pub const DEFAULT_MIN_STATEMENTS: usize = 3;

/// Default maximum statement count for a candidate SESE hammock.
pub const DEFAULT_MAX_STATEMENTS: usize = 15;

/// Virtual block ID for augmented single exit.
const VIRTUAL_EXIT_ID: BlockId = BlockId(u32::MAX);

/// A Single-Entry Single-Exit (SESE) region in a CFG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeseRegion {
    /// Entry block id.
    pub entry: BlockId,
    /// Exit block id (or virtual exit indicator).
    pub exit: BlockId,
    /// Basic blocks contained in this hammock (including entry and exit).
    pub blocks: Vec<BlockId>,
    /// Statements contained in this hammock, in execution order.
    pub statements: Vec<Statement>,
    /// 1-based start line of the region.
    pub start_line: usize,
    /// 1-based end line of the region.
    pub end_line: usize,
}

impl SeseRegion {
    /// Number of statements in this region.
    pub fn statement_count(&self) -> usize {
        self.statements.len()
    }
}

/// Returns true if a statement's text is a comment or empty.
pub fn is_comment(text: &str) -> bool {
    let t = text.trim();
    t.is_empty()
        || t.starts_with("//")
        || t.starts_with("/*")
        || t.starts_with('#')
        || t.starts_with('*')
}

/// Extract all candidate SESE hammocks from a CFG bounded by statement counts.
pub fn extract_sese_regions(
    cfg: &ControlFlowGraph,
    min_statements: usize,
    max_statements: usize,
) -> Vec<SeseRegion> {
    if cfg.blocks.is_empty() {
        return Vec::new();
    }

    // Build augmented CFG with a single virtual exit node to capture
    // regions that terminate at function exits.
    let mut aug_cfg = cfg.clone();
    let mut exit_blocks: HashSet<BlockId> = cfg.exits.iter().copied().collect();
    if exit_blocks.is_empty() {
        for &block in cfg.blocks.keys() {
            if cfg.successors(block).is_empty() {
                exit_blocks.insert(block);
            }
        }
    }
    if exit_blocks.is_empty() && !cfg.blocks.is_empty() {
        exit_blocks.insert(cfg.entry);
    }

    aug_cfg.add_block(BasicBlock {
        id: VIRTUAL_EXIT_ID,
        statements: Vec::new(),
        start_line: 0,
        end_line: 0,
    });
    for &exit in &exit_blocks {
        aug_cfg.add_edge(exit, VIRTUAL_EXIT_ID, CfgEdgeType::Next);
    }
    aug_cfg.exits = vec![VIRTUAL_EXIT_ID];
    aug_cfg.rebuild_adjacency();

    let dom = DominatorTree::build(&aug_cfg);
    let pdom = PostDominatorTree::build(&aug_cfg);

    let reachable_blocks: Vec<BlockId> = dom
        .reachable
        .iter()
        .copied()
        .filter(|&id| id != VIRTUAL_EXIT_ID && cfg.blocks.contains_key(&id))
        .collect();

    let mut candidate_exits = reachable_blocks.clone();
    candidate_exits.push(VIRTUAL_EXIT_ID);

    let mut seen_signatures = HashSet::new();
    let mut regions = Vec::new();

    // Check all pairs (u, v) where u is a CFG block, and v is a CFG block or VIRTUAL_EXIT_ID
    for &u in &reachable_blocks {
        for &v in &candidate_exits {
            // 1. u must dominate v
            if !dom.dominates(u, v) {
                continue;
            }

            // 2. v must post-dominate u
            if !pdom.post_dominates(v, u) {
                continue;
            }



            // 4. Collect candidate blocks W
            let mut w_set: HashSet<BlockId> = HashSet::new();
            for &w in &reachable_blocks {
                if dom.dominates(u, w) && pdom.post_dominates(v, w) {
                    w_set.insert(w);
                }
            }

            if !w_set.contains(&u) {
                continue;
            }
            if v != VIRTUAL_EXIT_ID && !w_set.contains(&v) {
                continue;
            }

            // 5. Single-Entry / Single-Exit boundary check on the original CFG
            let mut valid_boundary = true;
            for &w in &w_set {
                for &pred in cfg.predecessors(w) {
                    if !w_set.contains(&pred) && w != u {
                        valid_boundary = false;
                        break;
                    }
                }
                if !valid_boundary {
                    break;
                }
                for &succ in cfg.successors(w) {
                    if !w_set.contains(&succ) {
                        if v == VIRTUAL_EXIT_ID {
                            if !exit_blocks.contains(&w) {
                                valid_boundary = false;
                                break;
                            }
                        } else if w != v {
                            valid_boundary = false;
                            break;
                        }
                    }
                }
                if !valid_boundary {
                    break;
                }
            }

            if !valid_boundary {
                continue;
            }

            // 6. Collect statements and line boundaries
            let mut blocks_in_order: Vec<BlockId> = w_set.into_iter().collect();
            // Sort blocks by their start line / order in CFG
            blocks_in_order.sort_by_key(|&b| {
                cfg.blocks
                    .get(&b)
                    .map(|blk| (blk.start_line, b.0))
                    .unwrap_or((0, b.0))
            });

            let mut statements = Vec::new();
            let mut min_line = usize::MAX;
            let mut max_line = 0;

            for &b in &blocks_in_order {
                if let Some(block) = cfg.blocks.get(&b) {
                    for stmt in &block.statements {
                        if is_comment(&stmt.text) {
                            continue;
                        }
                        if stmt.line > 0 {
                            min_line = min_line.min(stmt.line);
                            max_line = max_line.max(stmt.line);
                        }
                        statements.push(stmt.clone());
                    }
                }
            }

            let stmt_count = statements.len();
            if stmt_count < min_statements || stmt_count > max_statements {
                continue;
            }

            if min_line == usize::MAX || max_line == 0 || min_line > max_line {
                continue;
            }

            // Deduplicate by (start_line, end_line, stmt_count)
            let sig = (min_line, max_line, stmt_count);
            if !seen_signatures.insert(sig) {
                continue;
            }

            regions.push(SeseRegion {
                entry: u,
                exit: v,
                blocks: blocks_in_order,
                statements,
                start_line: min_line,
                end_line: max_line,
            });
        }
    }

    // Sort regions by start line ascending, then statement count descending
    regions.sort_by_key(|r| (r.start_line, std::cmp::Reverse(r.statements.len())));
    regions
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg_for_function;

    #[test]
    fn test_sese_nested_loops() {
        let code = r#"
fn nested(matrix: Vec<Vec<i32>>) -> i32 {
    let mut sum = 0;
    for row in matrix {
        for val in row {
            sum += val;
        }
    }
    sum
}
"#;
        let cfg = build_cfg_for_function("rust", code, "nested").unwrap();
        let regions = extract_sese_regions(&cfg, 3, 15);
        assert!(!regions.is_empty(), "expected SESE regions for nested loops");
        // Verify at least one region captures the loop structure
        let loop_region = regions.iter().find(|r| r.statements.len() >= 3);
        assert!(loop_region.is_some());
        let r = loop_region.unwrap();
        assert!(r.start_line >= 2);
        assert!(r.end_line >= r.start_line);
    }

    #[test]
    fn test_sese_guard_clause() {
        let code = r#"
fn process(val: Option<i32>) -> i32 {
    if val.is_none() {
        return 0;
    }
    let x = val.unwrap();
    let y = x * 2;
    y + 1
}
"#;
        let cfg = build_cfg_for_function("rust", code, "process").unwrap();
        let regions = extract_sese_regions(&cfg, 3, 15);
        assert!(!regions.is_empty(), "expected SESE regions for guard clause");
        for r in &regions {
            assert!(r.statements.len() >= 3 && r.statements.len() <= 15);
            assert!(r.start_line <= r.end_line);
        }
    }

    #[test]
    fn test_sese_error_handler_if_else() {
        let code = r#"
fn handle(status: i32) -> &'static str {
    if status == 200 {
        let ok = "ok";
        ok
    } else {
        let err = "err";
        err
    }
}
"#;
        let cfg = build_cfg_for_function("rust", code, "handle").unwrap();
        let regions = extract_sese_regions(&cfg, 3, 15);
        assert!(!regions.is_empty(), "expected SESE regions for if-else");
        let branch_region = regions.iter().find(|r| r.blocks.len() >= 2);
        assert!(branch_region.is_some(), "expected multi-block SESE region for if-else");
    }
}

//! Backward and forward program slicing using the PDG.

use crate::cfg::ControlFlowGraph;
use crate::pdg::{PdgNodeId, ProgramDependenceGraph};
use rgctl_error::{Error, Result};
use std::collections::{HashSet, VecDeque};

/// Criterion for a program slice (variable at a line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceCriterion {
    /// Variable of interest.
    pub variable: String,
    /// Source line (1-based).
    pub line: usize,
}

/// Slice traversal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceDirection {
    /// Statements that influence the criterion.
    Backward,
    /// Statements influenced by the criterion.
    Forward,
}

/// Result of program slicing.
#[derive(Debug, Clone)]
pub struct CodeSlice {
    /// Original criterion.
    pub criterion: SliceCriterion,
    /// PDG nodes in the slice.
    pub statements: HashSet<PdgNodeId>,
    /// Source lines in the slice.
    pub lines: HashSet<usize>,
    /// Percentage of function lines excluded from slice.
    pub reduction_percent: f64,
}

fn variable_on_node(n: &crate::pdg::PdgNode, variable: &str) -> bool {
    n.defined_vars.contains(variable)
        || n.used_vars.contains(variable)
        || n.defined_vars
            .iter()
            .any(|d| d.starts_with(&format!("{variable}.")))
        || n.used_vars
            .iter()
            .any(|u| u.starts_with(&format!("{variable}.")))
}

/// Options for [`compute_slice`].
#[derive(Debug, Clone, Copy, Default)]
pub struct SliceOptions {
    /// Expand criterion via may-alias heuristics (P3 T2).
    pub with_alias: bool,
}

fn find_criterion_nodes(
    pdg: &ProgramDependenceGraph,
    criterion: &SliceCriterion,
    alias_names: &HashSet<String>,
) -> Result<Vec<PdgNodeId>> {
    let ids: Vec<PdgNodeId> = pdg
        .nodes
        .values()
        .filter(|n| {
            n.statement.line == criterion.line && alias_names.iter().any(|v| variable_on_node(n, v))
        })
        .map(|n| n.id)
        .collect();
    if ids.is_empty() {
        Err(Error::NotFound(format!(
            "no PDG node for {} at line {}",
            criterion.variable, criterion.line
        )))
    } else {
        Ok(ids)
    }
}

fn count_total_lines(cfg: &ControlFlowGraph) -> usize {
    let mut lines = HashSet::new();
    for block in cfg.blocks.values() {
        for stmt in &block.statements {
            if stmt.line > 0 {
                lines.insert(stmt.line);
            }
        }
    }
    lines.len()
}

fn reduction_percent(cfg: &ControlFlowGraph, lines: &HashSet<usize>) -> f64 {
    let total = count_total_lines(cfg);
    if total == 0 {
        0.0
    } else {
        100.0 * (1.0 - (lines.len() as f64 / total as f64))
    }
}

/// Backward slicer traversing data and control dependencies.
pub struct BackwardSlicer<'a> {
    pdg: &'a ProgramDependenceGraph,
    cfg: &'a ControlFlowGraph,
}

impl<'a> BackwardSlicer<'a> {
    /// Create a slicer over the given PDG and CFG.
    pub fn new(pdg: &'a ProgramDependenceGraph, cfg: &'a ControlFlowGraph) -> Self {
        Self { pdg, cfg }
    }

    /// Compute the backward slice for `criterion`.
    pub fn slice(&self, criterion: SliceCriterion) -> Result<CodeSlice> {
        self.slice_with_options(criterion, SliceOptions::default())
    }

    /// Backward slice with alias expansion options.
    pub fn slice_with_options(
        &self,
        criterion: SliceCriterion,
        options: SliceOptions,
    ) -> Result<CodeSlice> {
        let aliases = if options.with_alias {
            crate::alias::may_alias_names(self.cfg, &criterion.variable)
        } else {
            HashSet::from([criterion.variable.clone()])
        };
        let starts = find_criterion_nodes(self.pdg, &criterion, &aliases)?;
        let mut slice = HashSet::new();
        let mut worklist = VecDeque::from(starts);

        while let Some(node_id) = worklist.pop_front() {
            if !slice.insert(node_id) {
                continue;
            }

            for dep in self.pdg.data_deps.iter().filter(|d| d.to == node_id) {
                worklist.push_back(dep.from);
            }

            for ctrl in self
                .pdg
                .control_deps
                .iter()
                .filter(|c| c.dependent == node_id)
            {
                worklist.push_back(ctrl.controller);
            }
        }

        let lines: HashSet<usize> = slice
            .iter()
            .filter_map(|id| self.pdg.nodes.get(id).map(|n| n.statement.line))
            .collect();

        Ok(CodeSlice {
            criterion,
            statements: slice,
            lines: lines.clone(),
            reduction_percent: reduction_percent(self.cfg, &lines),
        })
    }
}

/// Forward slicer (reachableByFlows-style) over data and control deps.
pub struct ForwardSlicer<'a> {
    pdg: &'a ProgramDependenceGraph,
    cfg: &'a ControlFlowGraph,
}

impl<'a> ForwardSlicer<'a> {
    /// Create a slicer over the given PDG and CFG.
    pub fn new(pdg: &'a ProgramDependenceGraph, cfg: &'a ControlFlowGraph) -> Self {
        Self { pdg, cfg }
    }

    /// Compute the forward slice for `criterion`.
    pub fn slice(&self, criterion: SliceCriterion) -> Result<CodeSlice> {
        self.slice_with_options(criterion, SliceOptions::default())
    }

    /// Forward slice with alias expansion options.
    pub fn slice_with_options(
        &self,
        criterion: SliceCriterion,
        options: SliceOptions,
    ) -> Result<CodeSlice> {
        let aliases = if options.with_alias {
            crate::alias::may_alias_names(self.cfg, &criterion.variable)
        } else {
            HashSet::from([criterion.variable.clone()])
        };
        let starts = find_criterion_nodes(self.pdg, &criterion, &aliases)?;
        let mut slice = HashSet::new();
        let mut worklist = VecDeque::from(starts);

        while let Some(node_id) = worklist.pop_front() {
            if !slice.insert(node_id) {
                continue;
            }
            for dep in self.pdg.data_deps.iter().filter(|d| d.from == node_id) {
                worklist.push_back(dep.to);
            }
            for ctrl in self
                .pdg
                .control_deps
                .iter()
                .filter(|c| c.controller == node_id)
            {
                worklist.push_back(ctrl.dependent);
            }
        }

        let lines: HashSet<usize> = slice
            .iter()
            .filter_map(|id| self.pdg.nodes.get(id).map(|n| n.statement.line))
            .collect();

        Ok(CodeSlice {
            criterion,
            statements: slice,
            lines: lines.clone(),
            reduction_percent: reduction_percent(self.cfg, &lines),
        })
    }
}

/// Compute a slice in either direction.
pub fn compute_slice(
    pdg: &ProgramDependenceGraph,
    cfg: &ControlFlowGraph,
    criterion: SliceCriterion,
    direction: SliceDirection,
) -> Result<CodeSlice> {
    compute_slice_with_options(pdg, cfg, criterion, direction, SliceOptions::default())
}

/// Compute a slice with P3 options (alias).
pub fn compute_slice_with_options(
    pdg: &ProgramDependenceGraph,
    cfg: &ControlFlowGraph,
    criterion: SliceCriterion,
    direction: SliceDirection,
    options: SliceOptions,
) -> Result<CodeSlice> {
    match direction {
        SliceDirection::Backward => {
            BackwardSlicer::new(pdg, cfg).slice_with_options(criterion, options)
        }
        SliceDirection::Forward => {
            ForwardSlicer::new(pdg, cfg).slice_with_options(criterion, options)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::build_cfg_for_function;
    use crate::pdg::ProgramDependenceGraph;

    #[test]
    fn test_backward_slice_reduction() {
        let code = r#"
fn process(input: String) -> String {
    let a = 10;
    let b = 20;
    let x = input.len();
    let y = x * 2;
    format!("{}", y)
}
"#;
        let cfg = build_cfg_for_function("rust", code, "process").unwrap();
        let pdg = ProgramDependenceGraph::build(&cfg, code.as_bytes()).unwrap();
        let slicer = BackwardSlicer::new(&pdg, &cfg);

        let y_line = pdg
            .nodes
            .values()
            .find(|n| n.defined_vars.contains("y"))
            .unwrap()
            .statement
            .line;

        let slice = slicer
            .slice(SliceCriterion {
                variable: "y".to_string(),
                line: y_line,
            })
            .unwrap();
        assert!(slice.reduction_percent > 0.0);
        assert!(slice.lines.contains(&y_line));
    }

    #[test]
    fn test_forward_slice_java_order() {
        let code = r#"
public class OrderProcessor {
    public OrderDTO process(OrderDTO order) {
        order.status = "PROCESSED";
        return order;
    }
}
"#;
        let cfg = build_cfg_for_function("java", code, "process").unwrap();
        let pdg = ProgramDependenceGraph::build(&cfg, code.as_bytes()).unwrap();
        let write_line = pdg
            .nodes
            .values()
            .find(|n| n.defined_vars.contains("order.status"))
            .map(|n| n.statement.line)
            .expect("field write");
        let slice = ForwardSlicer::new(&pdg, &cfg)
            .slice(SliceCriterion {
                variable: "order".into(),
                line: write_line,
            })
            .unwrap();
        assert!(slice.lines.contains(&write_line));
        assert!(
            !slice.lines.is_empty(),
            "forward slice should include criterion line"
        );
    }
}

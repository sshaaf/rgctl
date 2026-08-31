//! `builtin.hasTags` evaluation.

use crate::engine::EvalNode;
use crate::eval::{MatchSite, violation};
use crate::findings::KantraViolation;

/// Match nodes whose labels contain all required tags.
pub fn eval_has_tags(rule_id: &str, tags: &[String], nodes: &[EvalNode]) -> Vec<KantraViolation> {
    let mut out = Vec::new();
    for node in nodes {
        if tags.iter().all(|t| node.labels.iter().any(|l| l == t)) {
            let file = node.file_path.clone().unwrap_or_default();
            let line = node.start_line.unwrap_or(1);
            out.push(violation(
                rule_id,
                "builtin.hasTags",
                &MatchSite::new(file, line).with_symbol(&node.name),
            ));
        }
    }
    out
}

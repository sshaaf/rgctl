//! Composite `and` / `or` / `not` evaluation.

use crate::eval::MatchSite;
use crate::findings::KantraViolation;
use crate::schema::WhenClause;
use std::collections::HashSet;

/// Evaluate composite clause against per-clause match sites.
pub fn eval_compose(
    clause: &WhenClause,
    rule_id: &str,
    leaf_sites: &[Vec<MatchSite>],
    leaf_index: &mut usize,
) -> Vec<KantraViolation> {
    match clause {
        WhenClause::And(items) => {
            if items.is_empty() {
                return Vec::new();
            }
            let mut sets: Vec<HashSet<MatchSite>> = Vec::new();
            for item in items {
                let sites = eval_compose_sites(item, leaf_sites, leaf_index);
                sets.push(sites.into_iter().collect());
            }
            let first = sets.first().cloned().unwrap_or_default();
            let intersection: HashSet<_> = sets
                .into_iter()
                .fold(first, |acc, set| acc.intersection(&set).cloned().collect());
            intersection
                .into_iter()
                .map(|s| crate::eval::violation(rule_id, "and", &s))
                .collect()
        }
        WhenClause::Or(items) => {
            let mut union = HashSet::new();
            for item in items {
                union.extend(eval_compose_sites(item, leaf_sites, leaf_index));
            }
            union
                .into_iter()
                .map(|s| crate::eval::violation(rule_id, "or", &s))
                .collect()
        }
        WhenClause::Not(inner) => {
            let inner_sites: HashSet<_> =
                eval_compose_sites(inner, leaf_sites, leaf_index).into_iter().collect();
            // NOT is only meaningful with a file-scoped universe; caller supplies leaf sites.
            let _ = inner_sites;
            Vec::new()
        }
        _ => {
            let sites = leaf_sites.get(*leaf_index).cloned().unwrap_or_default();
            *leaf_index += 1;
            sites
                .into_iter()
                .map(|s| {
                    let matched_by = match clause {
                        WhenClause::FileContent { .. } => "builtin.filecontent",
                        WhenClause::File { .. } => "builtin.file",
                        WhenClause::HasTags { .. } => "builtin.hasTags",
                        WhenClause::GoReferenced { .. } => "go.referenced",
                        WhenClause::JavaReferenced { .. } => "java.referenced",
                        _ => "leaf",
                    };
                    crate::eval::violation(rule_id, matched_by, &s)
                })
                .collect()
        }
    }
}

fn eval_compose_sites(
    clause: &WhenClause,
    leaf_sites: &[Vec<MatchSite>],
    leaf_index: &mut usize,
) -> Vec<MatchSite> {
    match clause {
        WhenClause::And(items) => {
            if items.is_empty() {
                return Vec::new();
            }
            let mut sets: Vec<HashSet<MatchSite>> = Vec::new();
            for item in items {
                sets.push(eval_compose_sites(item, leaf_sites, leaf_index).into_iter().collect());
            }
            let first = sets.first().cloned().unwrap_or_default();
            sets.into_iter()
                .fold(first, |acc, set| acc.intersection(&set).cloned().collect())
                .into_iter()
                .collect()
        }
        WhenClause::Or(items) => {
            let mut union = HashSet::new();
            for item in items {
                union.extend(eval_compose_sites(item, leaf_sites, leaf_index));
            }
            union.into_iter().collect()
        }
        WhenClause::Not(_) => Vec::new(),
        _ => {
            let sites = leaf_sites.get(*leaf_index).cloned().unwrap_or_default();
            *leaf_index += 1;
            sites
        }
    }
}

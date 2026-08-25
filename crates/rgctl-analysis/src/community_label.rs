//! Heuristic community naming (Graphify-style sidebar labels).
//!
//! Labels are derived metadata stored beside topology — never written into the
//! code-graph snapshot.

use crate::results::AnalysisResults;
use rgctl_error::Result;
use rgctl_graph::schema::Node;
use std::collections::HashMap;
use uuid::Uuid;

/// Inputs for naming a single community.
#[derive(Debug, Clone)]
pub struct CommunityLabelHints<'a> {
    /// Community id from label propagation.
    pub id: usize,
    /// Member symbol names.
    pub names: &'a [String],
    /// Member file paths.
    pub file_paths: &'a [String],
    /// Package / module labels (majority vote preferred).
    pub package_labels: &'a [String],
    /// Highest-PageRank member name within the community, if known.
    pub top_pagerank_name: Option<&'a str>,
    /// True when this id is the infrastructure / hub cluster.
    pub is_infrastructure: bool,
}

/// Infer a human-readable community label.
pub fn infer_community_label(hints: &CommunityLabelHints<'_>) -> String {
    if hints.is_infrastructure {
        return "Infrastructure / Common Library".to_string();
    }

    if let Some(pkg) = majority_token(hints.package_labels, 2) {
        if !pkg.is_empty() && pkg != "root" {
            let base = shorten_package_label(&pkg);
            if let Some(name) = hints.top_pagerank_name {
                let short = name.split(['/', '.', ':']).next_back().unwrap_or(name);
                if short.len() > 2 {
                    return format!("{base}::{short}");
                }
            }
            return base;
        }
    }

    if let Some(common) = find_common_path_prefix_strings(hints.file_paths) {
        if !common.is_empty() {
            return shorten_path_label(&common);
        }
    }

    if let Some(name) = hints.top_pagerank_name {
        let short = name.split(['/', '.', ':']).next_back().unwrap_or(name);
        if short.len() > 2 {
            return short.to_string();
        }
    }

    infer_label_from_names(hints.names, hints.id)
}

/// Ensure colliding labels become unique (`auth`, `auth (2)`, …).
pub fn dedupe_community_labels(labels: &mut [(usize, String)]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (_, label) in labels.iter_mut() {
        let key = label.to_ascii_lowercase();
        let count = seen.entry(key).or_insert(0);
        *count += 1;
        if *count > 1 {
            *label = format!("{label} ({count})");
        }
    }
}

/// Fill [`AnalysisResults::community`] labels from member metadata.
///
/// `lookup` returns `(name, file_path)` for each assigned node UUID.
pub fn fill_community_labels<F>(
    analysis: &mut AnalysisResults,
    infrastructure_id: Option<usize>,
    mut lookup: F,
) -> Result<()>
where
    F: FnMut(Uuid) -> Option<(String, Option<String>)>,
{
    let Some(table) = analysis.community.as_ref() else {
        return Ok(());
    };

    let mut members: HashMap<usize, Vec<(Uuid, u32)>> = HashMap::new();
    for (compact, &cid) in table.assignments.iter().enumerate() {
        let compact = compact as u32;
        if let Some(uuid) = analysis.get_uuid(compact) {
            members.entry(cid).or_default().push((uuid, compact));
        }
    }
    let pagerank = analysis.centrality.as_ref().map(|c| c.pagerank.clone());

    let mut labeled: Vec<(usize, String)> = Vec::with_capacity(members.len());
    for (cid, entries) in &members {
        let mut names = Vec::new();
        let mut paths = Vec::new();
        let mut packages = Vec::new();
        let mut top_pr_name: Option<String> = None;
        let mut top_pr = f32::MIN;

        for (uuid, compact) in entries {
            let Some((name, path)) = lookup(*uuid) else {
                continue;
            };
            if let Some(pr_table) = &pagerank {
                let score = pr_table.get(*compact as usize).copied().unwrap_or(0.0);
                if score > top_pr {
                    top_pr = score;
                    top_pr_name = Some(name.clone());
                }
            }
            if let Some(p) = &path {
                packages.push(package_label_from_path(p));
                paths.push(p.clone());
            }
            names.push(name);
        }

        let hints = CommunityLabelHints {
            id: *cid,
            names: &names,
            file_paths: &paths,
            package_labels: &packages,
            top_pagerank_name: top_pr_name.as_deref(),
            is_infrastructure: infrastructure_id == Some(*cid),
        };
        labeled.push((*cid, infer_community_label(&hints)));
    }

    labeled.sort_by_key(|(id, _)| *id);
    dedupe_community_labels(&mut labeled);

    if let Some(table) = analysis.community.as_mut() {
        table.infrastructure_community_id = infrastructure_id;
        table.labels = labeled.into_iter().collect();
    }
    Ok(())
}

/// Fill labels using full [`Node`] lookups (dashboard / MemoryBackend path).
pub fn fill_community_labels_from_nodes<F>(
    analysis: &mut AnalysisResults,
    infrastructure_id: Option<usize>,
    mut get_node: F,
) -> Result<()>
where
    F: FnMut(Uuid) -> Option<Node>,
{
    fill_community_labels(analysis, infrastructure_id, |uuid| {
        get_node(uuid).map(|n| {
            (
                n.name.to_string(),
                n.file_path.as_ref().map(|s| s.to_string()),
            )
        })
    })
}

fn majority_token(items: &[String], min_len: usize) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for item in items {
        if item.len() >= min_len {
            *counts.entry(item.as_str()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .filter(|(_, c)| *c * 3 >= items.len())
        .map(|(s, _)| s.to_string())
}

fn shorten_package_label(pkg: &str) -> String {
    let parts: Vec<&str> = pkg.split('.').filter(|p| !p.is_empty()).collect();
    if parts.len() <= 2 {
        return pkg.to_string();
    }
    parts[parts.len().saturating_sub(2)..].join(".")
}

fn shorten_path_label(path: &str) -> String {
    let path = path.replace('\\', "/");
    path.rsplit('/')
        .find(|s| !s.is_empty())
        .unwrap_or(path.as_str())
        .to_string()
}

fn package_label_from_path(file_path: &str) -> String {
    let path = file_path.replace('\\', "/");
    if let Some(idx) = path.find("/java/") {
        let after = &path[idx + 6..];
        if let Some(parent) = std::path::Path::new(after).parent() {
            let pkg = parent.to_string_lossy().replace('/', ".");
            if !pkg.is_empty() {
                return pkg;
            }
        }
    }
    if let Some(idx) = path.find("/src/") {
        let after = &path[idx + 5..];
        if let Some(parent) = std::path::Path::new(after).parent() {
            let pkg = parent.to_string_lossy().replace('/', ".");
            if !pkg.is_empty() {
                return pkg;
            }
        }
    }
    std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_string_lossy().replace('/', "."))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "root".into())
}

fn find_common_path_prefix_strings(paths: &[String]) -> Option<String> {
    if paths.is_empty() {
        return None;
    }
    let first = &paths[0];
    let mut prefix_len = first.len();
    for path in &paths[1..] {
        prefix_len = first
            .chars()
            .zip(path.chars())
            .take(prefix_len)
            .take_while(|(a, b)| a == b)
            .count();
    }
    if prefix_len == 0 {
        return None;
    }
    if let Some(last_slash) = first[..prefix_len].rfind('/') {
        return Some(first[..last_slash].to_string());
    }
    Some(first[..prefix_len].to_string())
}

fn infer_label_from_names(names: &[String], idx: usize) -> String {
    if names.is_empty() {
        return format!("Community {idx}");
    }

    let mut counts: HashMap<&str, usize> = HashMap::new();
    for name in names {
        for token in name.split(&['_', '-', '.'][..]) {
            if !token.is_empty() && token.len() > 2 {
                *counts.entry(token).or_insert(0) += 1;
            }
        }
    }

    if let Some((token, _)) = counts.iter().max_by_key(|(_, count)| *count) {
        if counts[token] * 3 >= names.len() {
            return (*token).to_string();
        }
    }

    format!("Community {idx}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_majority_wins() {
        let packages = vec![
            "com.example.auth".into(),
            "com.example.auth".into(),
            "com.example.other".into(),
        ];
        let names = vec!["a".into(), "b".into(), "c".into()];
        let label = infer_community_label(&CommunityLabelHints {
            id: 1,
            names: &names,
            file_paths: &[],
            package_labels: &packages,
            top_pagerank_name: None,
            is_infrastructure: false,
        });
        assert_eq!(label, "example.auth");
    }

    #[test]
    fn infrastructure_label() {
        let label = infer_community_label(&CommunityLabelHints {
            id: 0,
            names: &["util".into()],
            file_paths: &[],
            package_labels: &[],
            top_pagerank_name: None,
            is_infrastructure: true,
        });
        assert_eq!(label, "Infrastructure / Common Library");
    }

    #[test]
    fn dedupe_suffixes() {
        let mut labels = vec![(1, "auth".into()), (2, "auth".into()), (3, "cart".into())];
        dedupe_community_labels(&mut labels);
        assert_eq!(labels[0].1, "auth");
        assert_eq!(labels[1].1, "auth (2)");
        assert_eq!(labels[2].1, "cart");
    }

    #[test]
    fn token_fallback() {
        let names = vec!["do_login".into(), "try_login".into(), "login_admin".into()];
        let label = infer_community_label(&CommunityLabelHints {
            id: 9,
            names: &names,
            file_paths: &[],
            package_labels: &[],
            top_pagerank_name: None,
            is_infrastructure: false,
        });
        assert_eq!(label, "login");
    }
}

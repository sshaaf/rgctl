//! Query-time community overlay for GQL (virtual `:Community` + `community_id`).

use crate::community_label::{CommunityLabelHints, dedupe_community_labels, infer_community_label};
use crate::results::AnalysisResults;
use rgctl_graph::schema::{Node, NodeType};
use std::collections::HashMap;
use uuid::Uuid;

/// Property key marking a synthetic community node in GQL bindings.
pub const VIRTUAL_COMMUNITY_PROP: &str = "__virtual";
/// Property value for [`VIRTUAL_COMMUNITY_PROP`].
pub const VIRTUAL_COMMUNITY_VALUE: &str = "Community";

/// One named community for listing / filtering.
#[derive(Debug, Clone)]
pub struct CommunityInfo {
    /// Label-propagation community id.
    pub id: usize,
    /// Human-readable label.
    pub label: String,
    /// Member node count.
    pub member_count: usize,
}

/// Join context loaded beside the topology graph for community-aware GQL.
#[derive(Debug, Clone, Default)]
pub struct CommunityQueryContext {
    /// Graph-level modularity.
    pub modularity: f64,
    /// UUID → community id.
    pub uuid_to_community: HashMap<Uuid, usize>,
    /// Community summaries (sorted by member_count desc).
    pub communities: Vec<CommunityInfo>,
}

impl CommunityQueryContext {
    /// Build from columnar analysis; synthesizes labels when the table has none.
    pub fn from_analysis<F>(analysis: &AnalysisResults, mut lookup: F) -> Self
    where
        F: FnMut(Uuid) -> Option<(String, Option<String>)>,
    {
        let Some(table) = analysis.community.as_ref() else {
            return Self::default();
        };

        let mut members: HashMap<usize, Vec<(Uuid, u32)>> = HashMap::new();
        let mut uuid_to_community = HashMap::with_capacity(table.assignments.len());
        for (compact, &cid) in table.assignments.iter().enumerate() {
            let compact = compact as u32;
            if let Some(uuid) = analysis.get_uuid(compact) {
                uuid_to_community.insert(uuid, cid);
                members.entry(cid).or_default().push((uuid, compact));
            }
        }

        let labels = if table.labels.is_empty() {
            synthesize_labels(
                &members,
                table.infrastructure_community_id,
                analysis.centrality.as_ref().map(|c| c.pagerank.as_slice()),
                &mut lookup,
            )
        } else {
            table.labels.clone()
        };

        let mut communities: Vec<CommunityInfo> = members
            .iter()
            .map(|(id, entries)| CommunityInfo {
                id: *id,
                label: labels
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| format!("Community {id}")),
                member_count: entries.len(),
            })
            .collect();
        communities.sort_by_key(|c| std::cmp::Reverse(c.member_count));

        Self {
            modularity: table.modularity,
            uuid_to_community,
            communities,
        }
    }

    /// Community id for a graph node, if assigned.
    pub fn community_id(&self, uuid: Uuid) -> Option<usize> {
        self.uuid_to_community.get(&uuid).copied()
    }

    /// Synthetic GQL nodes for `MATCH (c:Community)`.
    pub fn community_nodes(&self) -> Vec<Node> {
        self.communities
            .iter()
            .map(|c| {
                let mut node = Node::new(NodeType::Module, c.label.clone());
                node.id = Uuid::from_u128(c.id as u128);
                node.properties.insert(
                    VIRTUAL_COMMUNITY_PROP.into(),
                    VIRTUAL_COMMUNITY_VALUE.into(),
                );
                node.properties
                    .insert("community_id".into(), c.id.to_string());
                node.properties.insert("label".into(), c.label.clone());
                node.properties
                    .insert("member_count".into(), c.member_count.to_string());
                node.properties
                    .insert("modularity".into(), format!("{:.6}", self.modularity));
                node.labels.push("Community".into());
                node
            })
            .collect()
    }
}

fn synthesize_labels<F>(
    members: &HashMap<usize, Vec<(Uuid, u32)>>,
    infrastructure_id: Option<usize>,
    pagerank: Option<&[f32]>,
    lookup: &mut F,
) -> HashMap<usize, String>
where
    F: FnMut(Uuid) -> Option<(String, Option<String>)>,
{
    let mut labeled: Vec<(usize, String)> = Vec::new();
    for (cid, entries) in members {
        let mut names = Vec::new();
        let mut paths = Vec::new();
        let mut packages = Vec::new();
        let mut top_pr_name: Option<String> = None;
        let mut top_pr = f32::MIN;
        for (uuid, compact) in entries {
            let Some((name, path)) = lookup(*uuid) else {
                continue;
            };
            if let Some(pr) = pagerank {
                let score = pr.get(*compact as usize).copied().unwrap_or(0.0);
                if score > top_pr {
                    top_pr = score;
                    top_pr_name = Some(name.clone());
                }
            }
            if let Some(p) = &path {
                packages.push(path_to_package(p));
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
    labeled.into_iter().collect()
}

fn path_to_package(file_path: &str) -> String {
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

/// True when a binding node is a virtual community.
pub fn is_virtual_community(node: &Node) -> bool {
    node.get_property(VIRTUAL_COMMUNITY_PROP) == Some(VIRTUAL_COMMUNITY_VALUE)
        || node.labels.iter().any(|l| l == "Community")
}

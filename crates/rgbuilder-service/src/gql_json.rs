//! Structured GQL CLI JSON response.

use rgbuilder_analysis::is_virtual_community;
use rgbuilder_gql::QueryResult;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Current GQL JSON schema version.
pub const GQL_SCHEMA_VERSION: u32 = 1;

/// One bound variable in a result row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GqlRowBinding {
    /// Variable name from the query pattern.
    pub binding: String,
    /// Matched node name.
    pub node: String,
    /// Node type label (`Community` for virtual overlay nodes).
    #[serde(rename = "type")]
    pub node_type: String,
    /// Fully-qualified name when present on the graph node (issue #49).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualified_name: Option<String>,
    /// Source file path when present.
    pub file: Option<String>,
    /// Community id when available (virtual property or community node).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_id: Option<usize>,
    /// Community label when binding a `:Community` node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Member count when binding a `:Community` node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<usize>,
    /// Allowlisted node properties for gate / agent observability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::BTreeMap<String, String>>,
}

/// Property keys projected into GQL JSON rows when present on the node.
const GQL_PROPERTY_ALLOWLIST: &[&str] = &[
    "type_params",
    "throws",
    "is_lambda",
    "is_external_stub",
    "is_annotation_element",
    "is_record",
    "is_constructor",
];

fn projected_properties(
    node: &rgbuilder_graph::schema::Node,
) -> Option<std::collections::BTreeMap<String, String>> {
    let mut out = std::collections::BTreeMap::new();
    for key in GQL_PROPERTY_ALLOWLIST {
        if let Some(val) = node.properties.get(*key) {
            out.insert((*key).to_string(), val.clone());
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

/// Top-level GQL JSON payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GqlJsonResponse {
    pub schema_version: u32,
    pub rows: Vec<Vec<GqlRowBinding>>,
    pub count: usize,
    pub explain: bool,
}

/// Serialize a [`QueryResult`] to the CLI JSON shape.
pub fn gql_result_to_json(result: &QueryResult, explain: bool) -> Value {
    let response = gql_response_from_result(result, explain);
    serde_json::to_value(&response).expect("GqlJsonResponse serializes")
}

/// Build a typed response from executor output.
pub fn gql_response_from_result(result: &QueryResult, explain: bool) -> GqlJsonResponse {
    let rows: Vec<Vec<GqlRowBinding>> = result
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|(name, node)| {
                    let virtual_community = is_virtual_community(node);
                    GqlRowBinding {
                        binding: name.clone(),
                        node: node.name.to_string(),
                        node_type: if virtual_community {
                            "Community".into()
                        } else {
                            format!("{:?}", node.node_type)
                        },
                        qualified_name: node.qualified_name.as_ref().map(|s| s.to_string()),
                        file: node.file_path.as_ref().map(|s| s.to_string()),
                        community_id: node
                            .get_property("community_id")
                            .and_then(|s| s.parse().ok()),
                        label: if virtual_community {
                            Some(
                                node.get_property("label")
                                    .unwrap_or(node.name.as_str())
                                    .to_string(),
                            )
                        } else {
                            None
                        },
                        member_count: node
                            .get_property("member_count")
                            .and_then(|s| s.parse().ok()),
                        properties: projected_properties(node),
                    }
                })
                .collect()
        })
        .collect();
    let count = rows.len();
    GqlJsonResponse {
        schema_version: GQL_SCHEMA_VERSION,
        rows,
        count,
        explain,
    }
}

/// Minimal fixture for schema sanity tests.
pub fn fixture_gql_response() -> GqlJsonResponse {
    GqlJsonResponse {
        schema_version: GQL_SCHEMA_VERSION,
        rows: vec![vec![GqlRowBinding {
            binding: "f".into(),
            node: "main".into(),
            node_type: "Function".into(),
            qualified_name: None,
            file: Some("src/main.rs".into()),
            community_id: None,
            label: None,
            member_count: None,
            properties: None,
        }]],
        count: 1,
        explain: false,
    }
}

pub fn fixture_gql_json() -> Value {
    json!(fixture_gql_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgbuilder_gql::QueryResult;
    use rgbuilder_graph::schema::{Node, NodeType};
    use std::collections::HashMap;

    #[test]
    fn projected_properties_allowlist() {
        let mut node = Node::new(NodeType::Function, "lam");
        node.properties.insert("is_lambda".into(), "true".into());
        node.properties.insert("type_params".into(), "<T>".into());
        node.properties
            .insert("unrelated_noise".into(), "skip".into());

        let mut row = HashMap::new();
        row.insert("f".to_string(), node);
        let response = gql_response_from_result(
            &QueryResult {
                rows: vec![row],
                plan: None,
            },
            false,
        );
        let props = response.rows[0][0].properties.as_ref().expect("properties");
        assert_eq!(props.get("is_lambda").map(String::as_str), Some("true"));
        assert_eq!(props.get("type_params").map(String::as_str), Some("<T>"));
        assert!(!props.contains_key("unrelated_noise"));
    }

    #[test]
    fn projects_qualified_name_when_present() {
        let node = Node::new(NodeType::Class, "Context")
            .with_qualified_name("org.openmrs.api.context.Context");
        let mut row = HashMap::new();
        row.insert("n".to_string(), node);
        let response = gql_response_from_result(
            &QueryResult {
                rows: vec![row],
                plan: None,
            },
            false,
        );
        assert_eq!(
            response.rows[0][0].qualified_name.as_deref(),
            Some("org.openmrs.api.context.Context")
        );
        let doc = serde_json::to_value(&response).unwrap();
        assert_eq!(
            doc["rows"][0][0]["qualified_name"].as_str(),
            Some("org.openmrs.api.context.Context")
        );
    }
}

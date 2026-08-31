//! Kantra YAML rule types.

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;

/// Parsed Kantra ruleset metadata + rules.
#[derive(Debug, Clone)]
pub struct KantraRulesetDoc {
    pub name: String,
    pub description: Option<String>,
    pub rules: Vec<KantraRule>,
}

/// One Kantra rule from YAML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KantraRule {
    #[serde(rename = "ruleID")]
    pub rule_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub effort: Option<u32>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    pub when: Value,
}

/// Support level for a rule after classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSupport {
    Supported,
    Partial,
    Unsupported,
}

/// Parsed `when` clause tree.
#[derive(Debug, Clone)]
pub enum WhenClause {
    FileContent {
        pattern: String,
        file_pattern: Option<String>,
    },
    File {
        pattern: String,
    },
    HasTags {
        tags: Vec<String>,
    },
    GoReferenced {
        pattern: String,
    },
    JavaReferenced {
        pattern: String,
        location: Option<String>,
        annotated_pattern: Option<String>,
    },
    And(Vec<WhenClause>),
    Or(Vec<WhenClause>),
    Not(Box<WhenClause>),
    Unsupported {
        provider: String,
    },
}

impl WhenClause {
    /// Parse a Kantra `when` YAML value into a clause tree.
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Mapping(map) => {
                if let Some(and) = map.get(&Value::from("and")) {
                    if let Value::Sequence(items) = and {
                        return WhenClause::And(
                            items.iter().map(WhenClause::from_value).collect(),
                        );
                    }
                }
                if let Some(or) = map.get(&Value::from("or")) {
                    if let Value::Sequence(items) = or {
                        return WhenClause::Or(items.iter().map(WhenClause::from_value).collect());
                    }
                }
                if let Some(not) = map.get(&Value::from("not")) {
                    return WhenClause::Not(Box::new(WhenClause::from_value(not)));
                }
                for (key, val) in map {
                    let provider = match key.as_str() {
                        Some(s) => s,
                        None => continue,
                    };
                    return parse_provider(provider, val);
                }
                WhenClause::Unsupported {
                    provider: "empty".into(),
                }
            }
            _ => WhenClause::Unsupported {
                provider: "invalid".into(),
            },
        }
    }

    /// Collect provider names for classification.
    pub fn providers(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_providers(&mut out);
        out
    }

    /// Regex patterns used by supported evaluators (for compile-time validation).
    pub fn regex_patterns(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_regex_patterns(&mut out);
        out
    }

    fn collect_regex_patterns<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            WhenClause::FileContent { pattern, .. } => out.push(pattern),
            WhenClause::GoReferenced { pattern } => out.push(pattern),
            WhenClause::JavaReferenced {
                pattern,
                annotated_pattern,
                ..
            } => {
                out.push(pattern);
                if let Some(p) = annotated_pattern {
                    out.push(p);
                }
            }
            WhenClause::And(items) | WhenClause::Or(items) => {
                for item in items {
                    item.collect_regex_patterns(out);
                }
            }
            WhenClause::Not(inner) => inner.collect_regex_patterns(out),
            WhenClause::File { .. } | WhenClause::HasTags { .. } | WhenClause::Unsupported { .. } => {}
        }
    }

    fn collect_providers<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            WhenClause::FileContent { .. } => out.push("builtin.filecontent"),
            WhenClause::File { .. } => out.push("builtin.file"),
            WhenClause::HasTags { .. } => out.push("builtin.hasTags"),
            WhenClause::GoReferenced { .. } => out.push("go.referenced"),
            WhenClause::JavaReferenced { .. } => out.push("java.referenced"),
            WhenClause::And(items) | WhenClause::Or(items) => {
                for item in items {
                    item.collect_providers(out);
                }
            }
            WhenClause::Not(inner) => inner.collect_providers(out),
            WhenClause::Unsupported { provider } => out.push(provider),
        }
    }
}

fn parse_provider(provider: &str, val: &Value) -> WhenClause {
    match provider {
        "builtin.filecontent" => {
            let (pattern, file_pattern) = pattern_fields(val);
            WhenClause::FileContent {
                pattern,
                file_pattern,
            }
        }
        "builtin.file" => WhenClause::File {
            pattern: string_field(val, "pattern").unwrap_or_default(),
        },
        "builtin.hasTags" => WhenClause::HasTags {
            tags: tags_field(val),
        },
        "go.referenced" => WhenClause::GoReferenced {
            pattern: string_field(val, "pattern").unwrap_or_default(),
        },
        "java.referenced" => {
            let location = string_field(val, "location");
            let annotated_pattern = val
                .get("annotated")
                .and_then(|a| a.get("pattern"))
                .and_then(|p| p.as_str())
                .map(str::to_string);
            if val.get("annotated.elements").is_some() {
                return WhenClause::Unsupported {
                    provider: "java.referenced.annotated.elements".into(),
                };
            }
            WhenClause::JavaReferenced {
                pattern: string_field(val, "pattern").unwrap_or_default(),
                location,
                annotated_pattern,
            }
        }
        other => WhenClause::Unsupported {
            provider: other.to_string(),
        },
    }
}

fn pattern_fields(val: &Value) -> (String, Option<String>) {
    (
        string_field(val, "pattern").unwrap_or_default(),
        string_field(val, "filePattern").or_else(|| string_field(val, "filepattern")),
    )
}

fn string_field(val: &Value, key: &str) -> Option<String> {
    val.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn tags_field(val: &Value) -> Vec<String> {
    if let Some(seq) = val.as_sequence() {
        return seq
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
    }
    if let Some(map) = val.as_mapping() {
        return map
            .get(&Value::from("tags"))
            .and_then(|t| t.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
    }
    Vec::new()
}

/// Ruleset metadata from `ruleset.yaml`.
#[derive(Debug, Deserialize)]
pub struct RulesetMeta {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Index rules by id for composite evaluation.
pub fn index_rules(rules: &[KantraRule]) -> HashMap<String, usize> {
    rules
        .iter()
        .enumerate()
        .map(|(i, r)| (r.rule_id.clone(), i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filecontent_when() {
        let yaml = r#"
builtin.filecontent:
  pattern: hkdf
  filePattern: "**/*.go"
"#;
        let val: Value = serde_yaml::from_str(yaml).unwrap();
        let clause = WhenClause::from_value(&val);
        match clause {
            WhenClause::FileContent { pattern, file_pattern } => {
                assert_eq!(pattern, "hkdf");
                assert_eq!(file_pattern.as_deref(), Some("**/*.go"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parses_and_when() {
        let yaml = r#"
and:
  - builtin.filecontent:
      pattern: foo
  - go.referenced:
      pattern: bar
"#;
        let val: Value = serde_yaml::from_str(yaml).unwrap();
        let clause = WhenClause::from_value(&val);
        assert!(matches!(clause, WhenClause::And(items) if items.len() == 2));
    }
}

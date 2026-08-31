//! Kantra findings artifact schema.

use crate::schema::KantraRule;
use serde::{Deserialize, Serialize};

/// One rule violation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KantraViolation {
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub file: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub matched_by: String,
}

/// Rule skipped during evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkippedRule {
    pub rule_id: String,
    pub reason: String,
}

/// Discover-time Kantra findings artifact (`.rgctl/kantra_findings.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KantraFindings {
    pub schema_version: u32,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    pub ruleset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_filter: Option<String>,
    pub evaluated_rules: usize,
    pub violations: Vec<KantraViolation>,
    pub skipped_rules: Vec<SkippedRule>,
}

impl KantraFindings {
    /// Build findings from evaluation output.
    pub fn new(
        ruleset_name: &str,
        catalog_id: Option<&str>,
        target_filter: Option<&str>,
        evaluated: usize,
    ) -> Self {
        Self {
            schema_version: 1,
            command: "kantra_findings".into(),
            catalog_id: catalog_id.map(str::to_string),
            ruleset: ruleset_name.into(),
            target_filter: target_filter.map(str::to_string),
            evaluated_rules: evaluated,
            violations: Vec::new(),
            skipped_rules: Vec::new(),
        }
    }

    /// Sort violations deterministically.
    pub fn sort_violations(&mut self) {
        self.violations.sort_by(|a, b| {
            (&a.file, a.line, &a.rule_id).cmp(&(&b.file, b.line, &b.rule_id))
        });
    }

    /// Attach message/category from rule metadata.
    pub fn enrich_from_rule(&mut self, rule: &KantraRule, idx: usize) {
        if let Some(v) = self.violations.get_mut(idx) {
            if v.message.is_none() {
                v.message = rule.message.clone();
            }
            if v.category.is_none() {
                v.category = rule.category.clone();
            }
        }
    }
}

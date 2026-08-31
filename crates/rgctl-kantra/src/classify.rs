//! Classify Kantra rules by supported providers.

use crate::schema::{KantraRule, RuleSupport, WhenClause};

/// Classification result for one rule.
#[derive(Debug, Clone)]
pub struct ClassifiedRule {
    pub rule_index: usize,
    pub support: RuleSupport,
    pub reason: Option<String>,
    pub clause: WhenClause,
}

const UNSUPPORTED_PROVIDERS: &[&str] = &[
    "java.dependency",
    "go.dependency",
    "builtin.xml",
    "builtin.json",
    "annotated.elements",
    "java.referenced.annotated.elements",
];

/// Classify all rules in a ruleset.
pub fn classify_rules(rules: &[KantraRule]) -> Vec<ClassifiedRule> {
    rules
        .iter()
        .enumerate()
        .map(|(i, rule)| {
            let clause = WhenClause::from_value(&rule.when);
            let providers = clause.providers();
            let unsupported: Vec<_> = providers
                .iter()
                .filter(|p| is_unsupported_provider(p))
                .copied()
                .collect();
            let supported_count = providers
                .iter()
                .filter(|p| is_supported_provider(p))
                .count();
            let (support, reason) = if unsupported.is_empty() {
                (RuleSupport::Supported, None)
            } else if supported_count > 0 {
                (
                    RuleSupport::Partial,
                    Some(format!("unsupported: {}", unsupported.join(", "))),
                )
            } else {
                (
                    RuleSupport::Unsupported,
                    Some(format!("unsupported: {}", unsupported.join(", "))),
                )
            };
            ClassifiedRule {
                rule_index: i,
                support,
                reason,
                clause,
            }
        })
        .collect()
}

fn is_unsupported_provider(provider: &str) -> bool {
    UNSUPPORTED_PROVIDERS
        .iter()
        .any(|u| provider == *u || provider.contains("dependency") || provider.contains("annotated.elements"))
}

fn is_supported_provider(provider: &str) -> bool {
    matches!(
        provider,
        "builtin.filecontent"
            | "builtin.file"
            | "builtin.hasTags"
            | "go.referenced"
            | "java.referenced"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::KantraRule;

    fn rule(when: &str) -> KantraRule {
        KantraRule {
            rule_id: "t".into(),
            description: None,
            category: None,
            effort: None,
            message: None,
            labels: vec![],
            when: serde_yaml::from_str(when).unwrap(),
        }
    }

    #[test]
    fn filecontent_supported() {
        let c = classify_rules(&[rule(
            "builtin.filecontent:\n  pattern: x\n",
        )]);
        assert_eq!(c[0].support, RuleSupport::Supported);
    }

    #[test]
    fn java_dependency_unsupported() {
        let c = classify_rules(&[rule(
            "java.dependency:\n  name: foo\n",
        )]);
        assert_eq!(c[0].support, RuleSupport::Unsupported);
        assert!(c[0].reason.as_ref().unwrap().contains("java.dependency"));
    }

    #[test]
    fn xml_unsupported() {
        let c = classify_rules(&[rule("builtin.xml:\n  xpath: //x\n")]);
        assert_eq!(c[0].support, RuleSupport::Unsupported);
    }
}

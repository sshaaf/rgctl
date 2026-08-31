//! Load Konveyor ruleset directories.

use crate::catalog::merge_ruleset_labels;
use crate::error::{KantraError, Result};
use crate::schema::{KantraRule, KantraRulesetDoc, RulesetMeta};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Loaded ruleset ready for evaluation.
#[derive(Debug, Clone)]
pub struct KantraRuleset {
    pub doc: KantraRulesetDoc,
    pub rules_dir: PathBuf,
}

impl KantraRuleset {
    /// Load `ruleset.yaml` and all `*.yaml` rule list files in `dir`.
    pub fn load(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let meta_path = dir.join("ruleset.yaml");
        let meta_text = fs::read_to_string(&meta_path)
            .map_err(|e| KantraError::msg(format!("{}: {e}", meta_path.display())))?;
        let meta: RulesetMeta = serde_yaml::from_str(&meta_text)
            .map_err(|e| KantraError::msg(format!("{}: {e}", meta_path.display())))?;

        let mut rules = Vec::new();
        let mut seen_ids = HashSet::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            if path.file_name() == Some(std::ffi::OsStr::new("ruleset.yaml")) {
                continue;
            }
            let text = fs::read_to_string(&path)
                .map_err(|e| KantraError::msg(format!("{}: {e}", path.display())))?;
            let batch: Vec<KantraRule> = serde_yaml::from_str(&text)
                .map_err(|e| KantraError::msg(format!("{}: {e}", path.display())))?;
            for mut rule in batch {
                merge_ruleset_labels(&meta.labels, &mut rule);
                if !seen_ids.insert(rule.rule_id.clone()) {
                    return Err(KantraError::msg(format!(
                        "duplicate ruleID {} in {}",
                        rule.rule_id,
                        path.display()
                    )));
                }
                rules.push(rule);
            }
        }

        Ok(Self {
            doc: KantraRulesetDoc {
                name: meta.name,
                description: meta.description,
                rules,
            },
            rules_dir: dir.to_path_buf(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn loads_fixture_ruleset() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("ruleset.yaml"),
            "name: test\ndescription: d\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("rules.yaml"),
            r#"
- ruleID: r1
  message: m
  when:
    builtin.filecontent:
      pattern: foo
"#,
        )
        .unwrap();
        let rs = KantraRuleset::load(dir.path()).unwrap();
        assert_eq!(rs.doc.rules.len(), 1);
        assert_eq!(rs.doc.rules[0].rule_id, "r1");
    }
}

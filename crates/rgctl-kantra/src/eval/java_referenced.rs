//! `java.referenced` evaluation.

use crate::engine::{EvalEdge, EvalNode};
use crate::eval::filecontent::SourceCache;
use crate::eval::{MatchSite, violation};
use crate::findings::KantraViolation;
use regex::Regex;
use std::path::Path;

const CODE_EXTENSIONS: &[&str] = &[".java", ".kt", ".scala"];

/// Evaluate Java referenced rules against graph nodes and optional source fallback.
pub fn eval_java_referenced(
    rule_id: &str,
    pattern: &str,
    location: Option<&str>,
    nodes: &[EvalNode],
    edges: &[EvalEdge],
    repo_root: &Path,
    sources: &SourceCache,
) -> Result<Vec<KantraViolation>, regex::Error> {
    let re = Regex::new(pattern)?;
    let loc = location.unwrap_or("IMPORT").to_ascii_uppercase();
    let mut out = Vec::new();
    match loc.as_str() {
        "IMPORT" => {
            for node in nodes {
                if node.node_type != "Import" || !is_code_file(node.file_path.as_deref()) {
                    continue;
                }
                if re.is_match(&node.name) {
                    push_node_hit(&mut out, rule_id, "java.referenced", node, "IMPORT");
                }
            }
        }
        "TYPE" | "CLASS" => {
            for node in nodes {
                if !matches!(node.node_type.as_str(), "Class" | "Interface" | "Enum") {
                    continue;
                }
                if !is_code_file(node.file_path.as_deref()) {
                    continue;
                }
                let qn = node.qualified_name.as_deref().unwrap_or(&node.name);
                if re.is_match(qn) || re.is_match(&node.name) {
                    push_node_hit(&mut out, rule_id, "java.referenced", node, &loc);
                }
            }
        }
        "PACKAGE" => {
            for node in nodes {
                if node.node_type != "Import" || !is_code_file(node.file_path.as_deref()) {
                    continue;
                }
                if re.is_match(&node.name) {
                    push_node_hit(&mut out, rule_id, "java.referenced", node, "PACKAGE");
                }
            }
            out.extend(filecontent_fallback(
                rule_id,
                &re,
                repo_root,
                sources,
                "java.referenced",
            )?);
        }
        "INHERITANCE" | "IMPLEMENTS_TYPE" => {
            let want = if loc == "INHERITANCE" {
                "EXTENDS"
            } else {
                "IMPLEMENTS"
            };
            for edge in edges {
                if edge.edge_type != want {
                    continue;
                }
                if re.is_match(&edge.to_qualified)
                    || re.is_match(&edge.to_name)
                    || re.is_match(&edge.from_qualified)
                {
                    out.push(violation(
                        rule_id,
                        "java.referenced",
                        &MatchSite::new(edge.file_path.clone(), edge.line).with_symbol(&edge.from_name),
                    ));
                }
            }
            out.extend(filecontent_fallback(
                rule_id,
                &re,
                repo_root,
                sources,
                "java.referenced",
            )?);
        }
        "ANNOTATION" => {
            for edge in edges {
                if edge.edge_type != "ANNOTATED_WITH" {
                    continue;
                }
                if re.is_match(&edge.to_qualified) || re.is_match(&edge.to_name) {
                    out.push(violation(
                        rule_id,
                        "java.referenced",
                        &MatchSite::new(edge.file_path.clone(), edge.line).with_symbol(&edge.from_name),
                    ));
                }
            }
            for node in nodes {
                if node.node_type == "Annotation"
                    && is_code_file(node.file_path.as_deref())
                    && (re.is_match(node.qualified_name.as_deref().unwrap_or(&node.name))
                        || re.is_match(&node.name))
                {
                    push_node_hit(&mut out, rule_id, "java.referenced", node, "ANNOTATION");
                }
            }
        }
        _ => {
            out.extend(filecontent_fallback(
                rule_id,
                &re,
                repo_root,
                sources,
                "java.referenced",
            )?);
        }
    }
    Ok(out)
}

fn push_node_hit(out: &mut Vec<KantraViolation>, rule_id: &str, matched_by: &str, node: &EvalNode, _loc: &str) {
    let file = node.file_path.clone().unwrap_or_default();
    let line = node.start_line.unwrap_or(1);
    let sym = node.qualified_name.as_deref().unwrap_or(&node.name);
    out.push(violation(
        rule_id,
        matched_by,
        &MatchSite::new(file, line).with_symbol(sym),
    ));
}

fn filecontent_fallback(
    rule_id: &str,
    re: &Regex,
    repo_root: &Path,
    sources: &SourceCache,
    matched_by: &str,
) -> Result<Vec<KantraViolation>, regex::Error> {
    let _ = (rule_id, re, repo_root, sources, matched_by);
  // Fallback: scan java sources for pattern when graph match insufficient.
    let mut out = Vec::new();
    for (path, content) in sources {
        if !is_code_file(Some(path)) {
            continue;
        }
        for (i, line) in content.lines().enumerate() {
            if re.is_match(line) {
                out.push(violation(
                    rule_id,
                    matched_by,
                    &MatchSite::new(path.clone(), i + 1),
                ));
            }
        }
    }
    Ok(out)
}

fn is_code_file(path: Option<&str>) -> bool {
    path.map(|p| CODE_EXTENSIONS.iter().any(|ext| p.ends_with(ext)))
        .unwrap_or(false)
}

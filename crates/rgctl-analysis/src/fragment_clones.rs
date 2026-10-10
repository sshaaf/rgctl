//! Fragment clone detection engine (CFG SESE hammocks, 1-WL hashing, two-stage filtering).
//!
//! Provides sub-quadratic structural clone detection across sub-function fragments
//! using 256-bit token bloom coarse pre-filtering and 1-WL canonical graph matching.

use crate::cfg::ControlFlowGraph;
use crate::cfg_builder::build_cfg_for_function;
use crate::cfg_pdg_archive::CfgPdgArchive;
use crate::clones::{
    path_excluded, CloneError, FragmentCloneFilters, FragmentCloneGroup, FragmentCloneReport,
    FragmentMember, FragmentSeedInfo, CLONE_REPORT_SCHEMA_VERSION_V2, MODE_FRAGMENT,
};
use crate::sese::extract_sese_regions;
use crate::wl_hash::WeisfeilerLehmanHasher;
use rgctl_graph::schema::{Node, NodeType};
use rgctl_graph::structural_sketch::{
    bloom_popcount, build_token_bloom, TokenBloom, TOKEN_BLOOM_WORDS,
};
use rgctl_graph::SnapshotNodeStore;
use std::collections::HashMap;
use std::path::Path;

/// Stage 1 bitwise bloom pre-filter matching against `SnapshotNodeStore` function blooms.
/// Evaluates `(func.bloom & seed_bloom) == seed_bloom` in O(N) bitwise time.
pub fn stage1_filter_functions(
    store: &SnapshotNodeStore,
    seed_bloom: &TokenBloom,
    filters: &FragmentCloneFilters,
) -> Result<Vec<Node>, CloneError> {
    let Some(col) = store.columnar() else {
        return Err(CloneError::Graph(rgctl_error::Error::Other(
            "columnar snapshot required for clone detection".into(),
        )));
    };
    let indexes = col.indexes_shared().map_err(CloneError::Graph)?;
    let function_ids = indexes
        .1
        .get(&NodeType::Function)
        .cloned()
        .unwrap_or_default();

    let seed_popcount = bloom_popcount(seed_bloom);
    let mut candidates = Vec::new();

    for id in function_ids {
        let Some(node) = store.get_node(id).map_err(CloneError::Graph)? else {
            continue;
        };

        if let Some(ref path) = node.file_path {
            if path_excluded(path.as_ref(), &filters.exclude) {
                continue;
            }
        }

        // If seed has no tokens, everything passes Stage 1
        if seed_popcount == 0 {
            candidates.push(node);
            continue;
        }

        if let Some(ref func_bloom) = node.token_bloom {
            let mut matches = true;
            for i in 0..TOKEN_BLOOM_WORDS {
                if (func_bloom[i] & seed_bloom[i]) != seed_bloom[i] {
                    matches = false;
                    break;
                }
            }
            if matches {
                candidates.push(node);
            }
        } else {
            // Include node conservatively if bloom is absent
            candidates.push(node);
        }
    }

    Ok(candidates)
}

/// Stage 2 CFG resolver: fast path via `cfg_pdg.archive.bin`, fallback via tree-sitter parse.
pub fn resolve_cfg_for_function(
    repo_root: &Path,
    node: &Node,
) -> Result<Option<ControlFlowGraph>, CloneError> {
    // 1. Fast path: check CFG/PDG archive Table of Contents
    let archive_path = CfgPdgArchive::default_path(repo_root);
    if archive_path.is_file() {
        if let Ok(Some(record)) = CfgPdgArchive::load_record_from_path(&archive_path, node.id) {
            return Ok(Some((*record.cfg).clone()));
        }
    }

    // 2. Fallback path: on-demand parse source with tree-sitter
    let Some(ref file_path) = node.file_path else {
        return Ok(None);
    };

    let full_path = repo_root.join(file_path.as_str());
    if !full_path.is_file() {
        return Ok(None);
    }

    let source = match std::fs::read_to_string(&full_path) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    let ext = full_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let ext_lang = language_from_ext(ext);
    let language = node
        .get_property("language")
        .unwrap_or(&ext_lang);

    match build_cfg_for_function(language, &source, &node.name) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(_) => Ok(None),
    }
}

fn language_from_ext(ext: &str) -> String {
    match ext.to_ascii_lowercase().as_str() {
        "rs" => "rust".into(),
        "java" => "java".into(),
        "c" | "h" => "c".into(),
        "cpp" | "cc" | "cxx" | "hpp" => "cpp".into(),
        "cs" => "csharp".into(),
        "go" => "go".into(),
        "py" => "python".into(),
        "js" => "javascript".into(),
        "ts" => "typescript".into(),
        "php" => "php".into(),
        "rb" => "ruby".into(),
        "pp" => "puppet".into(),
        "kt" | "kts" => "kotlin".into(),
        "groovy" => "groovy".into(),
        other => other.to_string(),
    }
}

/// Query seed configuration for fragment clone search.
#[derive(Debug, Clone, Default)]
pub struct FragmentSeedQuery {
    /// Function symbol name.
    pub symbol: Option<String>,
    /// File path.
    pub file: Option<String>,
    /// 1-based statement or line range (start, end).
    pub lines: Option<(usize, usize)>,
}

/// Execute a seed-scoped fragment clone query.
pub fn query_fragment_clones(
    store: &SnapshotNodeStore,
    repo_root: &Path,
    query: FragmentSeedQuery,
    filters: FragmentCloneFilters,
) -> Result<FragmentCloneReport, CloneError> {
    let graph_digest = store.content_digest().map_err(CloneError::Graph)?.to_string();

    // 1. Resolve seed node
    let (seed_node, seed_lines) = resolve_seed_node(store, &query)?;

    // 2. Resolve CFG for seed node
    let seed_cfg = resolve_cfg_for_function(repo_root, &seed_node)?
        .ok_or_else(|| CloneError::Graph(rgctl_error::Error::NotFound(format!("CFG for seed '{}'", seed_node.name))))?;

    // 3. Extract SESE hammocks and locate target seed hammock
    let seed_regions = extract_sese_regions(&seed_cfg, filters.min_statements, filters.max_statements);
    if seed_regions.is_empty() {
        return Ok(FragmentCloneReport {
            schema_version: CLONE_REPORT_SCHEMA_VERSION_V2,
            mode: MODE_FRAGMENT.to_string(),
            graph_digest,
            filters,
            seed: None,
            group_count: 0,
            groups: Vec::new(),
        });
    }

    let seed_region = if let Some((want_start, want_end)) = seed_lines {
        // Find region with maximum overlap / minimal distance to specified line range
        seed_regions
            .iter()
            .min_by_key(|r| {
                let dist = r.start_line.abs_diff(want_start) + r.end_line.abs_diff(want_end);
                let overlaps = r.start_line <= want_end && r.end_line >= want_start;
                (if overlaps { 0 } else { 1 }, dist)
            })
            .cloned()
            .unwrap_or_else(|| seed_regions[0].clone())
    } else {
        // Prominent hammock: largest statement count
        seed_regions
            .iter()
            .max_by_key(|r| r.statements.len())
            .cloned()
            .unwrap_or_else(|| seed_regions[0].clone())
    };

    let seed_hash = WeisfeilerLehmanHasher::hash_region(&seed_cfg, &seed_region);
    let seed_file = seed_node.file_path.as_deref().unwrap_or("?").to_string();

    let seed_info = FragmentSeedInfo {
        file: seed_file.clone(),
        start_line: seed_region.start_line,
        end_line: seed_region.end_line,
        enclosing_function: Some(seed_node.name.to_string()),
        structural_hash: seed_hash.clone(),
    };

    // 4. Compute Stage 1 bloom filter from seed statements
    let seed_snippet: String = seed_region
        .statements
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let seed_bloom = build_token_bloom("", None, None, Some(&seed_snippet));

    // 5. Stage 1 bitwise pre-filter
    let candidate_nodes = stage1_filter_functions(store, &seed_bloom, &filters)?;

    // 6. Stage 2 candidate evaluation and matching
    let mut matched_members = Vec::new();

    // Include seed itself
    matched_members.push(FragmentMember {
        id: seed_node.id.to_string(),
        name: seed_node.name.to_string(),
        file: seed_file,
        start_line: seed_region.start_line,
        end_line: seed_region.end_line,
        enclosing_function: seed_node.name.to_string(),
        statement_count: seed_region.statements.len(),
    });

    for cand_node in candidate_nodes {
        if cand_node.id == seed_node.id {
            continue;
        }

        let Ok(Some(cand_cfg)) = resolve_cfg_for_function(repo_root, &cand_node) else {
            continue;
        };

        let cand_regions = extract_sese_regions(&cand_cfg, filters.min_statements, filters.max_statements);
        for r in cand_regions {
            let h = WeisfeilerLehmanHasher::hash_region(&cand_cfg, &r);
            if h == seed_hash {
                matched_members.push(FragmentMember {
                    id: cand_node.id.to_string(),
                    name: cand_node.name.to_string(),
                    file: cand_node.file_path.as_deref().unwrap_or("?").to_string(),
                    start_line: r.start_line,
                    end_line: r.end_line,
                    enclosing_function: cand_node.name.to_string(),
                    statement_count: r.statements.len(),
                });
            }
        }
    }

    matched_members.sort_by(|a, b| (&a.file, a.start_line).cmp(&(&b.file, b.start_line)));
    matched_members.dedup_by(|a, b| a.file == b.file && a.start_line == b.start_line && a.end_line == b.end_line);

    let groups = if matched_members.len() >= 2 {
        vec![FragmentCloneGroup {
            structural_hash: seed_hash,
            size: matched_members.len(),
            score: 1.0,
            members: matched_members,
        }]
    } else {
        Vec::new()
    };

    Ok(FragmentCloneReport {
        schema_version: CLONE_REPORT_SCHEMA_VERSION_V2,
        mode: MODE_FRAGMENT.to_string(),
        graph_digest,
        filters,
        seed: Some(seed_info),
        group_count: groups.len(),
        groups,
    })
}

/// Execute full-repo unseeded fragment clone discovery.
pub fn discover_fragment_clones(
    store: &SnapshotNodeStore,
    repo_root: &Path,
    filters: FragmentCloneFilters,
) -> Result<FragmentCloneReport, CloneError> {
    let graph_digest = store.content_digest().map_err(CloneError::Graph)?.to_string();

    let Some(col) = store.columnar() else {
        return Err(CloneError::Graph(rgctl_error::Error::Other(
            "columnar snapshot required for clone detection".into(),
        )));
    };
    let indexes = col.indexes_shared().map_err(CloneError::Graph)?;
    let function_ids = indexes
        .1
        .get(&NodeType::Function)
        .cloned()
        .unwrap_or_default();

    let mut hash_to_members: HashMap<String, Vec<FragmentMember>> = HashMap::new();

    for id in function_ids {
        let Some(node) = store.get_node(id).map_err(CloneError::Graph)? else {
            continue;
        };

        if let Some(ref path) = node.file_path {
            if path_excluded(path.as_ref(), &filters.exclude) {
                continue;
            }
        }

        let Ok(Some(cfg)) = resolve_cfg_for_function(repo_root, &node) else {
            continue;
        };

        let regions = extract_sese_regions(&cfg, filters.min_statements, filters.max_statements);
        let file = node.file_path.as_deref().unwrap_or("?").to_string();

        for r in regions {
            let h = WeisfeilerLehmanHasher::hash_region(&cfg, &r);
            hash_to_members.entry(h).or_default().push(FragmentMember {
                id: node.id.to_string(),
                name: node.name.to_string(),
                file: file.clone(),
                start_line: r.start_line,
                end_line: r.end_line,
                enclosing_function: node.name.to_string(),
                statement_count: r.statements.len(),
            });
        }
    }

    let mut groups = Vec::new();
    for (structural_hash, mut members) in hash_to_members {
        members.sort_by(|a, b| (&a.file, a.start_line).cmp(&(&b.file, b.start_line)));
        members.dedup_by(|a, b| a.file == b.file && a.start_line == b.start_line && a.end_line == b.end_line);

        if members.len() >= 2 {
            groups.push(FragmentCloneGroup {
                structural_hash,
                size: members.len(),
                score: 1.0,
                members,
            });
        }
    }

    groups.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| b.members[0].statement_count.cmp(&a.members[0].statement_count)));

    Ok(FragmentCloneReport {
        schema_version: CLONE_REPORT_SCHEMA_VERSION_V2,
        mode: MODE_FRAGMENT.to_string(),
        graph_digest,
        filters,
        seed: None,
        group_count: groups.len(),
        groups,
    })
}

fn resolve_seed_node(
    store: &SnapshotNodeStore,
    query: &FragmentSeedQuery,
) -> Result<(Node, Option<(usize, usize)>), CloneError> {
    if let Some(ref sym) = query.symbol {
        // Look up by function symbol name
        let Some(col) = store.columnar() else {
            return Err(CloneError::Graph(rgctl_error::Error::Other("columnar store required".into())));
        };
        let indexes = col.indexes_shared().map_err(CloneError::Graph)?;
        let function_ids = indexes.1.get(&NodeType::Function).cloned().unwrap_or_default();

        let mut matches = Vec::new();
        for id in function_ids {
            if let Ok(Some(n)) = store.get_node(id) {
                if n.name == *sym || n.qualified_name.as_deref() == Some(sym.as_str()) {
                    matches.push(n);
                }
            }
        }

        if matches.is_empty() {
            return Err(CloneError::Graph(rgctl_error::Error::NotFound(format!("symbol '{sym}'"))));
        }
        if matches.len() > 1 {
            // Disambiguate by file if provided
            if let Some(ref file) = query.file {
                if let Some(m) = matches.iter().find(|n| n.file_path.as_deref().unwrap_or("").ends_with(file.as_str())) {
                    return Ok((m.clone(), query.lines));
                }
            }

            return Err(CloneError::Graph(rgctl_error::Error::AmbiguousSymbol {
                name: sym.clone(),
                count: matches.len(),
                candidates: matches
                    .iter()
                    .map(|n| rgctl_error::SymbolCandidate {
                        id: n.id.to_string(),
                        name: n.name.to_string(),
                        qualified_name: n.qualified_name.as_deref().map(Into::into),
                        node_type: format!("{:?}", n.node_type).to_lowercase(),
                        file: n.file_path.as_deref().map(Into::into),
                        line: n.start_line,
                    })
                    .collect(),
            }));
        }

        return Ok((matches[0].clone(), query.lines));
    }

    if let Some(ref file) = query.file {
        let (start, end) = query.lines.unwrap_or((1, usize::MAX));
        let Some(col) = store.columnar() else {
            return Err(CloneError::Graph(rgctl_error::Error::Other("columnar store required".into())));
        };
        let indexes = col.indexes_shared().map_err(CloneError::Graph)?;
        let function_ids = indexes.1.get(&NodeType::Function).cloned().unwrap_or_default();

        let mut best: Option<Node> = None;
        for id in function_ids {
            if let Ok(Some(n)) = store.get_node(id) {
                if n.file_path.as_deref().unwrap_or("").ends_with(file.as_str()) {
                    let s = n.start_line.unwrap_or(0);
                    let e = n.end_line.unwrap_or(usize::MAX);
                    if s <= start && e >= end {
                        best = Some(n);
                        break;
                    } else if s <= end && e >= start {
                        best = Some(n);
                    }
                }
            }
        }

        if let Some(n) = best {
            return Ok((n, query.lines));
        }

        return Err(CloneError::Graph(rgctl_error::Error::NotFound(format!("function in '{file}' spanning lines {start}-{end}"))));
    }

    Err(CloneError::Graph(rgctl_error::Error::InvalidQuery("no seed symbol or file:lines specified".into())))
}

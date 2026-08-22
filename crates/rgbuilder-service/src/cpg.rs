//! Hybrid CPG / inspect / slice family.

use crate::command::{CpgArgs, CpgOp};
use crate::error::{Result, ServiceError};
use crate::inspect_json::{inspect_cfg_json, inspect_pdg_json};
use crate::slice_json::text_slice_json;
use rgbuilder_analysis::{
    AstSkeletonArchive, BackwardSlicer, CpgFlowsArgs, ForwardSlicer, MutationQuery,
    ProgramDependenceGraph, SliceCriterion, SliceDirection as AnalysisSliceDirection,
    build_cfg_for_function, cpg_calls, cpg_flows, cpg_function, cpg_mutations, cpg_status,
    language_id_from_path,
};
use rgbuilder_graph::CodeGraph;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Whether CFG/PDG archive exists.
#[must_use]
pub fn cfg_ready(repo: &Path) -> bool {
    rgbuilder_analysis::CfgPdgArchive::default_path(repo).is_file()
}

/// Dispatch CPG family ops.
pub fn run_cpg(graph: Option<&CodeGraph>, repo: &Path, args: &CpgArgs) -> Result<Value> {
    match args.op {
        CpgOp::Status => {
            let status = cpg_status(repo).map_err(ServiceError::from)?;
            serde_json::to_value(&status).map_err(ServiceError::from)
        }
        CpgOp::Function => {
            let graph = graph.ok_or_else(|| ServiceError::Failed("graph required".into()))?;
            let symbol = require(args.symbol.as_deref(), "symbol")?;
            let info = cpg_function(graph.backend(), repo, symbol).map_err(ServiceError::from)?;
            serde_json::to_value(&info).map_err(ServiceError::from)
        }
        CpgOp::Calls => {
            let graph = graph.ok_or_else(|| ServiceError::Failed("graph required".into()))?;
            let symbol = require(args.symbol.as_deref(), "symbol")?;
            let info = cpg_calls(graph.backend(), symbol).map_err(ServiceError::from)?;
            serde_json::to_value(&info).map_err(ServiceError::from)
        }
        CpgOp::Mutations => {
            let type_name = require(args.type_name.as_deref(), "type_name")?;
            let result = cpg_mutations(
                repo,
                MutationQuery {
                    type_name: type_name.to_string(),
                    exclude_ctors: args.exclude_ctors,
                    member: args.member.clone(),
                    include_unresolved: args.include_unresolved,
                },
            )
            .map_err(ServiceError::from)?;
            serde_json::to_value(&result).map_err(ServiceError::from)
        }
        CpgOp::Flows => run_flows(repo, args),
        CpgOp::Slice => run_slice(repo, args),
        CpgOp::Inspect | CpgOp::Pdg => run_inspect(graph, repo, args),
        CpgOp::Ast => run_ast(repo, args),
    }
}

fn require<'a>(value: Option<&'a str>, name: &str) -> Result<&'a str> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ServiceError::InvalidParams(format!("`{name}` is required")))
}

fn run_flows(repo: &Path, args: &CpgArgs) -> Result<Value> {
    let file = require(args.file.as_deref(), "file")?;
    let variable = require(args.variable.as_deref(), "variable")?;
    let function = require(args.function.as_deref(), "function")?;
    let line = args
        .line
        .ok_or_else(|| ServiceError::InvalidParams("`line` is required".into()))?;
    let direction = parse_direction(args.direction.as_deref())?;
    if is_markup(Path::new(file)) {
        return Err(ServiceError::Failed(
            "cpg flows: Markdown context files are not CFG-capable".into(),
        ));
    }
    let result = cpg_flows(CpgFlowsArgs {
        repo_root: repo.to_path_buf(),
        file: file.to_string(),
        line,
        variable: variable.to_string(),
        function: function.to_string(),
        language: None,
        direction,
        with_alias: args.with_alias,
    })
    .map_err(ServiceError::from)?;
    serde_json::to_value(&result).map_err(ServiceError::from)
}

fn run_slice(repo: &Path, args: &CpgArgs) -> Result<Value> {
    let file = require(args.file.as_deref(), "file")?;
    let variable = require(args.variable.as_deref(), "variable")?;
    let line = args
        .line
        .ok_or_else(|| ServiceError::InvalidParams("`line` is required".into()))?;
    let path = resolve_file(repo, file)?;
    if is_markup(&path) {
        return Err(ServiceError::Failed(
            "slice: Markdown context files are not CFG-capable".into(),
        ));
    }
    let source = std::fs::read_to_string(&path).map_err(rgbuilder_error::Error::from)?;
    let lang = language_id_from_path(&path)
        .unwrap_or("rust")
        .to_string();
    let fn_name = args
        .function
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("main")
                .to_string()
        });
    let cfg = build_cfg_for_function(&lang, &source, &fn_name).map_err(ServiceError::from)?;
    let pdg = ProgramDependenceGraph::build(&cfg, source.as_bytes()).map_err(ServiceError::from)?;
    let criterion = SliceCriterion {
        variable: variable.to_string(),
        line,
    };
    let direction = parse_direction(args.direction.as_deref())?;
    let slice = match direction {
        AnalysisSliceDirection::Backward => BackwardSlicer::new(&pdg, &cfg)
            .slice(criterion.clone())
            .map_err(ServiceError::from)?,
        AnalysisSliceDirection::Forward => ForwardSlicer::new(&pdg, &cfg)
            .slice(criterion.clone())
            .map_err(ServiceError::from)?,
    };
    let dir_label = match direction {
        AnalysisSliceDirection::Forward => "forward",
        AnalysisSliceDirection::Backward => "backward",
    };
    let response = text_slice_json(file, &criterion, dir_label, &slice, &pdg);
    serde_json::to_value(&response).map_err(ServiceError::from)
}

fn run_inspect(graph: Option<&CodeGraph>, repo: &Path, args: &CpgArgs) -> Result<Value> {
    let graph = graph.ok_or_else(|| ServiceError::Failed("graph required".into()))?;
    let symbol = require(args.symbol.as_deref(), "symbol")?;
    let (node, source) = resolve_function(graph, symbol)?;
    let file = node.file_path.as_deref().unwrap_or(".");
    if is_markup(Path::new(file)) {
        return Err(ServiceError::Failed(
            "inspect: Markdown context files are not CFG-capable".into(),
        ));
    }
    let lang = language_id_from_path(Path::new(file))
        .unwrap_or("rust")
        .to_string();
    let mut cfg = build_cfg_for_function(&lang, &source, &node.name).map_err(ServiceError::from)?;
    let pdg = ProgramDependenceGraph::build(&cfg, source.as_bytes()).map_err(ServiceError::from)?;
    let _ = repo;
    if args.op == CpgOp::Pdg {
        let response = inspect_pdg_json(symbol, &pdg, false, pdg.data_deps.len(), pdg.control_deps.len());
        return serde_json::to_value(&response).map_err(ServiceError::from);
    }
    cfg.prune_unreachable_blocks();
    let response = inspect_cfg_json(symbol, &cfg, true);
    serde_json::to_value(&response).map_err(ServiceError::from)
}

fn run_ast(repo: &Path, args: &CpgArgs) -> Result<Value> {
    let symbol = require(args.symbol.as_deref(), "symbol")?;
    let archive = AstSkeletonArchive::open_if_exists(repo)
        .map_err(ServiceError::from)?
        .ok_or_else(|| {
            ServiceError::Failed(
                "AST skeleton archive missing (run `rg-build discover --with-ast-skeleton`)"
                    .into(),
            )
        })?;
    let matches: Vec<_> = archive
        .records
        .iter()
        .filter(|r| {
            r.function_name == symbol
                || r.function_name.ends_with(symbol)
                || format!("{}.{}", r.file_path, r.function_name).contains(symbol)
        })
        .collect();
    if matches.is_empty() {
        return Err(ServiceError::Failed(format!(
            "no AST skeleton for '{symbol}'"
        )));
    }
    Ok(serde_json::json!({
        "schema_version": 1,
        "records": matches,
    }))
}

fn parse_direction(raw: Option<&str>) -> Result<AnalysisSliceDirection> {
    match raw.unwrap_or("backward") {
        "forward" => Ok(AnalysisSliceDirection::Forward),
        "backward" => Ok(AnalysisSliceDirection::Backward),
        other => Err(ServiceError::InvalidParams(format!(
            "unknown direction '{other}'. allowed: forward, backward"
        ))),
    }
}

fn is_markup(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some(ext) if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("mdx")
    )
}

fn resolve_file(repo: &Path, file: &str) -> Result<PathBuf> {
    let as_given = PathBuf::from(file);
    if as_given.is_file() {
        return Ok(as_given);
    }
    let under_repo = repo.join(file);
    if under_repo.is_file() {
        return Ok(under_repo);
    }
    Err(ServiceError::InvalidParams(format!(
        "source file not found: {file}"
    )))
}

fn resolve_function(
    graph: &CodeGraph,
    symbol: &str,
) -> Result<(rgbuilder_graph::schema::Node, String)> {
    use rgbuilder_graph::schema::NodeType;
    let backend = graph.backend();
    let matches = backend
        .find_nodes_by_name(symbol)
        .map_err(ServiceError::from)?;
    let node = matches
        .into_iter()
        .find(|n| n.node_type == NodeType::Function)
        .or_else(|| {
            backend.all_nodes().ok()?.into_iter().find(|n| {
                n.node_type == NodeType::Function
                    && (n.name == symbol || n.name.ends_with(symbol))
            })
        })
        .ok_or_else(|| ServiceError::Failed(format!("function symbol not found: {symbol}")))?;
    let file = node
        .file_path
        .clone()
        .ok_or_else(|| ServiceError::Failed("function has no file path".into()))?;
    let source = std::fs::read_to_string(&file).map_err(rgbuilder_error::Error::from)?;
    Ok((node, source))
}

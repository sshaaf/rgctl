//! `rgctl slice` — line-level slicing and taint policy checks.

use super::args::{OutputFormat, SliceDirection, SliceView};
use super::context::{CliContext, language_from_path};
use super::markup::markup_context_unsupported;
use super::slice_output::{
    cfg_topology_json, pdg_topology_json, taint_slice_json, text_slice_json,
};
use crate::analysis::{
    BackwardSlicer, ForwardSlicer, ProgramDependenceGraph, SliceCriterion, TaintAnalyzer,
    build_cfg_for_function,
};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::PathBuf;

pub struct SliceArgs {
    pub file: String,
    pub line: usize,
    pub variable: String,
    pub function: Option<String>,
    pub language: Option<String>,
    pub direction: SliceDirection,
    pub taint: bool,
    pub view: SliceView,
}

pub fn run(ctx: &CliContext, args: SliceArgs) -> Result<()> {
    let path = resolve_slice_path(ctx, &args.file)?;
    if let Some(msg) = markup_context_unsupported("slice", &path) {
        bail!(msg);
    }
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let lang = args
        .language
        .clone()
        .unwrap_or_else(|| language_from_path(&path));
    let fn_name = args.function.clone().unwrap_or_else(|| {
        if lang == "python" {
            "process".to_string()
        } else {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("main")
                .to_string()
        }
    });

    let mut cfg = build_cfg_for_function(&lang, &source, &fn_name)?;
    let pdg = ProgramDependenceGraph::build(&cfg, source.as_bytes())?;
    let criterion = SliceCriterion {
        variable: args.variable.clone(),
        line: args.line,
    };

    if args.taint {
        let mut analyzer = TaintAnalyzer::new(&pdg, &cfg);
        analyzer.detect_patterns(&lang);
        let flows = analyzer
            .analyze_with_policy()
            .map_err(|v| anyhow::anyhow!(v.to_string()))?;
        if ctx.format == OutputFormat::Json {
            let response = taint_slice_json(
                &args.file,
                &fn_name,
                args.line,
                &args.variable,
                flows.len(),
                flows.iter().filter(|f| f.is_vulnerable()).count(),
            );
            return ctx.emit_json_value(&serde_json::to_value(&response)?);
        }
        println!(
            "Taint flows: {} ({} vulnerable)",
            flows.len(),
            flows.iter().filter(|f| f.is_vulnerable()).count()
        );
        return Ok(());
    }

    let slice = match args.direction {
        SliceDirection::Backward => BackwardSlicer::new(&pdg, &cfg).slice(criterion.clone())?,
        SliceDirection::Forward => ForwardSlicer::new(&pdg, &cfg).slice(criterion.clone())?,
    };

    match args.view {
        SliceView::Cfg => {
            cfg.prune_unreachable_blocks();
            if ctx.format == OutputFormat::Json {
                let response = cfg_topology_json(&path.display().to_string(), &fn_name, &cfg);
                ctx.emit_json_value(&serde_json::to_value(&response)?)?;
            } else {
                println!(
                    "CFG: {} blocks, {} edges",
                    cfg.blocks.len(),
                    cfg.edges.len()
                );
            }
        }
        SliceView::Pdg => {
            if ctx.format == OutputFormat::Json {
                let response =
                    pdg_topology_json(&path.display().to_string(), &fn_name, &pdg, false);
                ctx.emit_json_value(&serde_json::to_value(&response)?)?;
            } else {
                println!(
                    "PDG: {} nodes, {} data deps, {} control deps",
                    pdg.nodes.len(),
                    pdg.data_deps.len(),
                    pdg.control_deps.len()
                );
            }
        }
        SliceView::Text => render_slice_text(
            ctx,
            &args.file,
            &fn_name,
            &criterion,
            &slice,
            &pdg,
            args.direction,
        )?,
    }
    Ok(())
}

/// Resolve a slice source path: prefer as-given, then relative to `--repo`.
fn resolve_slice_path(ctx: &CliContext, file: &str) -> Result<PathBuf> {
    let as_given = PathBuf::from(file);
    if as_given.is_file() {
        return Ok(as_given);
    }
    if as_given.is_absolute() {
        bail!("source file not found: {}", as_given.display());
    }
    let under_repo = ctx.repo.join(file);
    if under_repo.is_file() {
        return Ok(under_repo);
    }
    bail!(
        "source file not found: {file} (also tried {})",
        under_repo.display()
    );
}

fn render_slice_text(
    ctx: &CliContext,
    display_path: &str,
    function: &str,
    criterion: &SliceCriterion,
    slice: &crate::analysis::CodeSlice,
    pdg: &ProgramDependenceGraph,
    direction: SliceDirection,
) -> Result<()> {
    if ctx.format == OutputFormat::Json {
        let response = text_slice_json(
            display_path,
            criterion,
            &format!("{direction:?}").to_lowercase(),
            slice,
            pdg,
        );
        return ctx.emit_json_value(&serde_json::to_value(&response)?);
    }
    println!(
        "{direction:?} slice for {display_path}:{} (variable: {})",
        criterion.line, criterion.variable
    );
    println!("Reduction: {:.1}%", slice.reduction_percent);
    let mut lines: Vec<_> = slice.lines.iter().copied().collect();
    lines.sort_unstable();
    for ln in lines {
        println!("  {ln}");
    }
    let _ = function;
    Ok(())
}

//! `rgctl cpg` — hybrid CPG façade (L_repo ⟷ L_proc).

use super::args::{InspectLayer, OutputFormat, PdgEdgeLayer, SliceDirection, SliceView};
use super::context::CliContext;
use super::inspect::{self, InspectArgs};
use super::markup::markup_context_unsupported;
use super::slice::{self, SliceArgs};
use anyhow::Result;
use rgctl_analysis::{
    AstSkeletonArchive, CpgExportFormat, CpgExportScope, CpgFlowsArgs, MutationQuery,
    SliceDirection as AnalysisSliceDirection, cpg_calls, cpg_flows, cpg_function, cpg_mutations,
    cpg_status, export_cpg,
};
use std::path::Path;

pub enum CpgAction {
    Status,
    Function {
        symbol: String,
    },
    Calls {
        symbol: String,
    },
    Mutations {
        type_name: String,
        exclude_ctors: bool,
        member: Option<String>,
        include_unresolved: bool,
    },
    Flows {
        file: String,
        line: usize,
        variable: String,
        function: String,
        language: Option<String>,
        direction: SliceDirection,
        with_alias: bool,
    },
    Ast {
        symbol: String,
    },
    Export {
        format: String,
        output: String,
        path_contains: Option<String>,
        include_l_proc: bool,
        include_field_writes: bool,
    },
    Pdg {
        symbol: String,
        edge_layer: PdgEdgeLayer,
        def_use: bool,
    },
    Slice {
        file: String,
        line: usize,
        variable: String,
        function: Option<String>,
        language: Option<String>,
        direction: SliceDirection,
        taint: bool,
        view: SliceView,
    },
}

fn cpg_action_to_service(action: &CpgAction) -> rgctl_service::CpgArgs {
    use rgctl_service::{CpgArgs, CpgOp};
    let dir = |d: &SliceDirection| match d {
        SliceDirection::Forward => Some("forward".into()),
        SliceDirection::Backward => Some("backward".into()),
    };
    match action {
        CpgAction::Status => CpgArgs {
            op: CpgOp::Status,
            symbol: None,
            type_name: None,
            file: None,
            line: None,
            variable: None,
            function: None,
            exclude_ctors: false,
            member: None,
            include_unresolved: false,
            direction: None,
            with_alias: false,
        },
        CpgAction::Function { symbol } => CpgArgs {
            op: CpgOp::Function,
            symbol: Some(symbol.clone()),
            type_name: None,
            file: None,
            line: None,
            variable: None,
            function: None,
            exclude_ctors: false,
            member: None,
            include_unresolved: false,
            direction: None,
            with_alias: false,
        },
        CpgAction::Calls { symbol } => CpgArgs {
            op: CpgOp::Calls,
            symbol: Some(symbol.clone()),
            type_name: None,
            file: None,
            line: None,
            variable: None,
            function: None,
            exclude_ctors: false,
            member: None,
            include_unresolved: false,
            direction: None,
            with_alias: false,
        },
        CpgAction::Mutations {
            type_name,
            exclude_ctors,
            member,
            include_unresolved,
        } => CpgArgs {
            op: CpgOp::Mutations,
            symbol: None,
            type_name: Some(type_name.clone()),
            file: None,
            line: None,
            variable: None,
            function: None,
            exclude_ctors: *exclude_ctors,
            member: member.clone(),
            include_unresolved: *include_unresolved,
            direction: None,
            with_alias: false,
        },
        CpgAction::Flows {
            file,
            line,
            variable,
            function,
            direction,
            with_alias,
            ..
        } => CpgArgs {
            op: CpgOp::Flows,
            symbol: None,
            type_name: None,
            file: Some(file.clone()),
            line: Some(*line),
            variable: Some(variable.clone()),
            function: Some(function.clone()),
            exclude_ctors: false,
            member: None,
            include_unresolved: false,
            direction: dir(direction),
            with_alias: *with_alias,
        },
        CpgAction::Ast { symbol } => CpgArgs {
            op: CpgOp::Ast,
            symbol: Some(symbol.clone()),
            type_name: None,
            file: None,
            line: None,
            variable: None,
            function: None,
            exclude_ctors: false,
            member: None,
            include_unresolved: false,
            direction: None,
            with_alias: false,
        },
        CpgAction::Pdg { symbol, .. } => CpgArgs {
            op: CpgOp::Pdg,
            symbol: Some(symbol.clone()),
            type_name: None,
            file: None,
            line: None,
            variable: None,
            function: None,
            exclude_ctors: false,
            member: None,
            include_unresolved: false,
            direction: None,
            with_alias: false,
        },
        CpgAction::Slice {
            file,
            line,
            variable,
            function,
            direction,
            ..
        } => CpgArgs {
            op: CpgOp::Slice,
            symbol: None,
            type_name: None,
            file: Some(file.clone()),
            line: Some(*line),
            variable: Some(variable.clone()),
            function: function.clone(),
            exclude_ctors: false,
            member: None,
            include_unresolved: false,
            direction: dir(direction),
            with_alias: false,
        },
        CpgAction::Export { .. } => unreachable!("export stays on CLI path"),
    }
}

pub fn run(ctx: &CliContext, action: CpgAction) -> Result<()> {
    if ctx.format == OutputFormat::Json
        && !matches!(&action, CpgAction::Export { .. })
    {
        let svc = cpg_action_to_service(&action);
        let mut session = rgctl_service::Session::new(&ctx.repo);
        let value =
            rgctl_service::execute(&mut session, rgctl_service::Command::Cpg(svc))?;
        return ctx.emit_json_value(&value);
    }
    match action {
        CpgAction::Status => run_status(ctx),
        CpgAction::Function { symbol } => run_function(ctx, &symbol),
        CpgAction::Calls { symbol } => run_calls(ctx, &symbol),
        CpgAction::Mutations {
            type_name,
            exclude_ctors,
            member,
            include_unresolved,
        } => run_mutations(ctx, type_name, exclude_ctors, member, include_unresolved),
        CpgAction::Flows {
            file,
            line,
            variable,
            function,
            language,
            direction,
            with_alias,
        } => run_flows(
            ctx, file, line, variable, function, language, direction, with_alias,
        ),
        CpgAction::Ast { symbol } => run_ast(ctx, &symbol),
        CpgAction::Export {
            format,
            output,
            path_contains,
            include_l_proc,
            include_field_writes,
        } => run_export(
            ctx,
            &format,
            &output,
            path_contains,
            include_l_proc,
            include_field_writes,
        ),
        CpgAction::Pdg {
            symbol,
            edge_layer,
            def_use,
        } => inspect::run(
            ctx,
            InspectArgs {
                symbol,
                layer: InspectLayer::Pdg {
                    edge_layer,
                    def_use,
                },
            },
        ),
        CpgAction::Slice {
            file,
            line,
            variable,
            function,
            language,
            direction,
            taint,
            view,
        } => slice::run(
            ctx,
            SliceArgs {
                file,
                line,
                variable,
                function,
                language,
                direction,
                taint,
                view,
            },
        ),
    }
}

fn run_status(ctx: &CliContext) -> Result<()> {
    let status = cpg_status(&ctx.repo)?;
    if ctx.format == OutputFormat::Json {
        return ctx.emit_json_value(&serde_json::to_value(&status)?);
    }
    if status.archive_present {
        println!(
            "CPG L_proc: ready ({} functions) at {}",
            status.function_count, status.archive_path
        );
        if let Some(d) = &status.graph_digest {
            println!("  graph_digest: {d}");
        }
    } else {
        println!(
            "CPG L_proc: missing (run `rgctl discover --with-cfg`)\n  expected: {}",
            status.archive_path
        );
    }
    if status.field_write_index_present {
        println!(
            "CPG field writes: {} indexed (cpg mutations)",
            status.field_write_count
        );
    } else {
        println!("CPG field writes: missing (run discover --with-cfg)");
    }
    if status.ast_skeleton_present {
        println!("CPG AST skeleton: {} functions", status.ast_skeleton_count);
    } else {
        println!("CPG AST skeleton: missing (optional: discover --with-ast-skeleton)");
    }
    println!("CPG L_repo: use `cpg function` / `cpg calls` against the graph snapshot");
    Ok(())
}

fn run_mutations(
    ctx: &CliContext,
    type_name: String,
    exclude_ctors: bool,
    member: Option<String>,
    include_unresolved: bool,
) -> Result<()> {
    let result = cpg_mutations(
        &ctx.repo,
        MutationQuery {
            type_name,
            exclude_ctors,
            member,
            include_unresolved,
        },
    )?;
    if ctx.format == OutputFormat::Json {
        return ctx.emit_json_value(&serde_json::to_value(&result)?);
    }
    println!(
        "Mutations of {}{} ({} hits):",
        result.type_name,
        if result.exclude_ctors {
            " [excl. ctors]"
        } else {
            ""
        },
        result.mutations.len()
    );
    for m in &result.mutations {
        println!("  {}:{}  {}", m.file, m.line, m.code.trim());
    }
    Ok(())
}

fn run_flows(
    ctx: &CliContext,
    file: String,
    line: usize,
    variable: String,
    function: String,
    language: Option<String>,
    direction: SliceDirection,
    with_alias: bool,
) -> Result<()> {
    if let Some(msg) = markup_context_unsupported("cpg flows", Path::new(&file)) {
        anyhow::bail!(msg);
    }
    let direction = match direction {
        SliceDirection::Forward => AnalysisSliceDirection::Forward,
        SliceDirection::Backward => AnalysisSliceDirection::Backward,
    };
    let result = cpg_flows(CpgFlowsArgs {
        repo_root: ctx.repo.clone(),
        file,
        line,
        variable,
        function,
        language,
        direction,
        with_alias,
    })?;
    if ctx.format == OutputFormat::Json {
        return ctx.emit_json_value(&serde_json::to_value(&result)?);
    }
    println!(
        "CPG flows ({}) {} @ {}:{} — {} steps ({:.0}% reduction){}",
        result.direction,
        result.variable,
        result.file,
        result.line,
        result.steps.len(),
        result.reduction_percent,
        if with_alias { " [alias]" } else { "" }
    );
    for step in &result.steps {
        println!("  {}: {}", step.line, step.code.trim());
    }
    Ok(())
}

fn run_ast(ctx: &CliContext, symbol: &str) -> Result<()> {
    let archive = AstSkeletonArchive::open_if_exists(&ctx.repo)?.ok_or_else(|| {
        anyhow::anyhow!(
            "AST skeleton archive missing (run `rgctl discover --with-ast-skeleton`)"
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
        anyhow::bail!("no AST skeleton for '{symbol}'");
    }
    if ctx.format == OutputFormat::Json {
        return ctx.emit_json_value(&serde_json::json!({
            "schema_version": 1,
            "records": matches,
        }));
    }
    for rec in matches {
        println!(
            "{} ({}) — {} nodes",
            rec.function_name,
            rec.file_path,
            rec.nodes.len()
        );
        for n in &rec.nodes {
            let indent = if n.parent.is_some() { "  " } else { "" };
            println!(
                "{indent}[{:?}] L{}-{} {}",
                n.kind, n.start_line, n.end_line, n.label
            );
        }
    }
    Ok(())
}

fn run_export(
    ctx: &CliContext,
    format: &str,
    output: &str,
    path_contains: Option<String>,
    include_l_proc: bool,
    include_field_writes: bool,
) -> Result<()> {
    let format = match format.to_ascii_lowercase().as_str() {
        "graphml" => CpgExportFormat::GraphMl,
        "graphson" | "json" => CpgExportFormat::GraphSon,
        other => anyhow::bail!("unsupported export format '{other}' (graphml|graphson)"),
    };
    let graph = ctx.load_graph()?;
    let content = export_cpg(
        graph.backend(),
        &ctx.repo,
        format,
        &CpgExportScope {
            path_contains,
            include_l_proc,
            include_field_writes,
        },
    )?;
    std::fs::write(output, content)?;
    if ctx.format != OutputFormat::Json {
        println!("Wrote CPG export to {output}");
    } else {
        ctx.emit_json_value(&serde_json::json!({
            "schema_version": 1,
            "output": output,
        }))?;
    }
    Ok(())
}

fn run_function(ctx: &CliContext, symbol: &str) -> Result<()> {
    let graph = ctx.load_graph()?;
    let info = cpg_function(graph.backend(), &ctx.repo, symbol)?;
    if ctx.format == OutputFormat::Json {
        return ctx.emit_json_value(&serde_json::to_value(&info)?);
    }
    println!(
        "{} ({}){}",
        info.name,
        info.id,
        if info.is_constructor {
            " [constructor]"
        } else {
            ""
        }
    );
    if let Some(qn) = &info.qualified_name {
        println!("  qualified: {qn}");
    }
    if let Some(file) = &info.file_path {
        println!("  file: {}:{}", file, info.start_line.unwrap_or(0));
    }
    println!(
        "  L_proc: {}",
        if info.has_l_proc {
            "yes (CFG/PDG archived)"
        } else {
            "no — run discover --with-cfg"
        }
    );
    Ok(())
}

fn run_calls(ctx: &CliContext, symbol: &str) -> Result<()> {
    let graph = ctx.load_graph()?;
    let info = cpg_calls(graph.backend(), symbol)?;
    if ctx.format == OutputFormat::Json {
        return ctx.emit_json_value(&serde_json::to_value(&info)?);
    }
    println!(
        "CALL neighborhood for {} ({}):",
        info.function_name, info.function_id
    );
    if info.edges.is_empty() {
        println!("  (no CALL edges)");
        return Ok(());
    }
    for e in &info.edges {
        println!("  [{}] {} ({})", e.direction, e.name, e.id);
    }
    Ok(())
}

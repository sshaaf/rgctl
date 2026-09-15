//! `rgctl check` — CI policy gateway.

use super::args::OutputFormat;
use super::check_output::{build_check_response, violations_from_json_values};
use super::context::CliContext;
use super::policy_file::PolicyFile;
use crate::analysis::{BlastRadiusEngine, CentralityAnalyzer, PetGraphView, PolicyViolation};
use anyhow::Result;
use serde_json::json;
use rgctl_incremental::changed_function_symbols;
use std::path::Path;

pub struct CheckArgs {
    pub policy_file: String,
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub strict: bool,
    pub temporal: bool,
    pub strict_calendar: bool,
}

pub fn run(ctx: &CliContext, args: CheckArgs) -> Result<()> {
    if args.temporal {
        return super::pr_check::run(
            ctx,
            super::pr_check::PrCheckArgs {
                policy_file: args.policy_file,
                base_artifact: None,
                head_artifact: None,
                base_ref: args.base_ref.unwrap_or_else(|| "origin/main".to_string()),
                head_ref: args.head_ref.unwrap_or_else(|| "HEAD".to_string()),
                strict: args.strict,
                cascade_depth: 1,
                full_snapshots: false,
                bisect: false,
                synthetic_head: None,
                strict_calendar: args.strict_calendar,
            },
        );
    }

    if ctx.format == OutputFormat::Json {
        let mut session = rgctl_service::Session::new(&ctx.repo);
        if !session.graph_ready() {
            anyhow::bail!("Graph not found (run `rgctl discover` first)");
        }
        let value = rgctl_service::execute(
            &mut session,
            rgctl_service::Command::Check(rgctl_service::CheckArgs {
                policy_file: args.policy_file.clone(),
                base_ref: args.base_ref.clone(),
                head_ref: args.head_ref.clone(),
                strict: args.strict,
            }),
        )?;
        ctx.emit_json_value(&value)?;
        if value.get("passed").and_then(|v| v.as_bool()) == Some(false) {
            std::process::exit(1);
        }
        return Ok(());
    }

    let policy = PolicyFile::load(Path::new(&args.policy_file))?;
    let strict = args.strict || policy.scope.strict_diff;
    let registry = policy.into_registry();
    let centrality_threshold = registry.centrality_alert_threshold;
    let graph = ctx.load_graph()?;
    let backend = graph.backend();
    let view = PetGraphView::from_backend(backend)?;
    let centrality = CentralityAnalyzer::new().analyze_with_view(&view)?.scores;
    let engine = BlastRadiusEngine::build(backend)?;

    let symbols = changed_function_symbols(
        &ctx.repo,
        backend,
        &rgctl_incremental::SymbolScopeOptions {
            base_ref: args.base_ref.clone(),
            head_ref: args.head_ref.clone(),
            strict,
        },
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let symbol_count = symbols.len();
    let mut violation_rows = Vec::new();

    for symbol in symbols {
        let Ok((id, _)) = crate::analysis::resolve_unique_symbol(backend, &symbol) else {
            continue;
        };
        if let Err(err) =
            engine.analyze_with_policy(id, Some(backend), Some(&registry), Some(&centrality))
        {
            violation_rows.push(json!({
                "symbol": symbol,
                "error": err.to_string(),
            }));
            continue;
        }
        if let Ok(result) = engine.analyze(id) {
            for node_id in &result.impact_zone_ids {
                if let Some(score) = centrality.get(node_id) {
                    if score.betweenness > centrality_threshold {
                        violation_rows.push(json!({
                            "symbol": symbol,
                            "violation": format!("{}", PolicyViolation::CascadeHazard {
                                node: *node_id,
                                betweenness: score.betweenness,
                                threshold: centrality_threshold,
                            }),
                        }));
                    }
                }
            }
        }
    }

    let response = build_check_response(
        &args.policy_file,
        violations_from_json_values(&violation_rows),
    );

    if ctx.format == OutputFormat::Json {
        ctx.emit_json_value(&serde_json::to_value(&response)?)?;
    } else if response.passed {
        println!("Policy check passed ({} symbols)", symbol_count);
    } else {
        println!("Policy violations: {}", response.violations.len());
        for v in &response.violations {
            println!("  {v:?}");
        }
    }

    if !response.passed {
        std::process::exit(1);
    }
    Ok(())
}


//! `rgctl communities` — list / refresh community labels.

use super::args::OutputFormat;
use super::context::CliContext;
use anyhow::{Context, Result, bail};
use rgctl_analysis::{AnalysisResults, CommunityQueryContext, fill_community_labels};
use rgctl_graph::backend::GraphBackend;
use serde_json::json;

pub struct CommunitiesLabelArgs {
    /// Rewrite `.rgctl/analysis_results.bin` labels (heuristic refresh).
    pub write: bool,
}

pub fn run_label(ctx: &CliContext, args: CommunitiesLabelArgs) -> Result<()> {
    let analysis_path = ctx.repo.join(".rgctl/analysis_results.bin");
    if !analysis_path.is_file() {
        bail!(
            "analysis results not found at {} (run `rgctl discover` first)",
            analysis_path.display()
        );
    }
    let mut analysis = AnalysisResults::load(&analysis_path)
        .with_context(|| format!("load {}", analysis_path.display()))?;
    let infra = analysis
        .community
        .as_ref()
        .and_then(|c| c.infrastructure_community_id);

    let graph = ctx.load_graph()?;
    let backend = graph.backend();
    fill_community_labels(&mut analysis, infra, |uuid| {
        backend.get_node(uuid).ok().flatten().map(|n| {
            (
                n.name.to_string(),
                n.file_path.as_ref().map(|s| s.to_string()),
            )
        })
    })?;

    if args.write {
        analysis
            .save(&analysis_path)
            .with_context(|| format!("write {}", analysis_path.display()))?;
    }

    let ctx_q = CommunityQueryContext::from_analysis(&analysis, |uuid| {
        backend.get_node(uuid).ok().flatten().map(|n| {
            (
                n.name.to_string(),
                n.file_path.as_ref().map(|s| s.to_string()),
            )
        })
    });

    if ctx.format == OutputFormat::Json {
        let communities: Vec<_> = ctx_q
            .communities
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "label": c.label,
                    "member_count": c.member_count,
                })
            })
            .collect();
        return ctx.emit_json_value(&json!({
            "schema_version": 1,
            "modularity": ctx_q.modularity,
            "written": args.write,
            "communities": communities,
        }));
    }

    println!(
        "{} communities (modularity {:.3}){}",
        ctx_q.communities.len(),
        ctx_q.modularity,
        if args.write {
            " — labels written"
        } else {
            ""
        }
    );
    for c in &ctx_q.communities {
        println!("  [{:>4}] {} ({} members)", c.id, c.label, c.member_count);
    }
    Ok(())
}

pub fn run_list(ctx: &CliContext) -> Result<()> {
    run_label(ctx, CommunitiesLabelArgs { write: false })
}

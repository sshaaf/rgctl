//! `rgctl metrics` — PageRank, betweenness, and community detection.

use super::args::OutputFormat;
use super::context::CliContext;
use anyhow::Result;
use rgbuilder_service::command::{Command, MetricsArgs as SvcMetrics};
use rgbuilder_service::{Session, execute};
use serde_json::json;

pub struct MetricsArgs {
    pub pagerank: bool,
    pub betweenness: bool,
    pub communities: bool,
    pub iterations: Option<usize>,
}

pub fn run(ctx: &CliContext, args: MetricsArgs) -> Result<()> {
    let run_all = !args.pagerank && !args.betweenness && !args.communities;
    if ctx.format == OutputFormat::Json {
        if super::daemon::route_metrics(
            ctx,
            args.pagerank || run_all,
            args.betweenness || run_all,
            args.communities || run_all,
        )? {
            return Ok(());
        }

        let mut session = Session::new(&ctx.repo);
        if !session.graph_ready() {
            anyhow::bail!("Graph not found (run `rgctl discover` first)");
        }
        let value = execute(
            &mut session,
            Command::Metrics(SvcMetrics {
                pagerank: args.pagerank || run_all,
                betweenness: args.betweenness || run_all,
                communities: args.communities || run_all,
            }),
        )?;
        let _ = args.iterations;
        return ctx.emit_json_value(&value);
    }

    use super::metrics_output::{
        MetricsCommunitiesSection, MetricsPagerankSection, build_metrics_response,
    };
    use crate::analysis::{
        BetweennessCentrality, CommunityDetector, FastPageRank, PetGraphView,
        community_edge_types_for_backend, default_behavioral_edges,
    };

    let graph = ctx.load_graph()?;
    let view = PetGraphView::from_backend(graph.backend())?;
    let iterations = args.iterations.unwrap_or(20);
    let allowed = default_behavioral_edges();

    let mut pagerank = None;
    let mut betweenness = None;
    let mut communities = None;

    if args.pagerank || run_all {
        let engine = FastPageRank::new(iterations, 0.85);
        let (scores, stats) = engine.compute(&view, allowed);
        let mut top: Vec<_> = scores
            .iter()
            .filter(|(_, score)| **score > 0.0)
            .map(|(id, score)| (*id, *score))
            .collect();
        if top.is_empty() {
            top = scores.iter().map(|(id, score)| (*id, *score)).collect();
        }
        top.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        top.truncate(20);
        let top = top
            .iter()
            .map(|(id, score)| json!({ "node": id.to_string(), "pagerank": score }))
            .collect();
        pagerank = Some(MetricsPagerankSection {
            top,
            converged: stats.converged,
            iterations: stats.iterations_run,
            max_delta: stats.max_delta,
        });
    }

    if args.betweenness || run_all {
        let bc = BetweennessCentrality::compute_unbounded(&view, allowed);
        let mut top: Vec<_> = bc.iter().map(|(id, score)| (id, *score)).collect();
        top.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        top.truncate(20);
        betweenness = Some(
            top.iter()
                .map(|(id, s)| json!({ "node": id.to_string(), "score": s }))
                .collect(),
        );
    }

    if args.communities || run_all {
        let detector = CommunityDetector::new();
        let allowed_comm = community_edge_types_for_backend(graph.backend());
        let result = detector.detect_with_view_filtered(&view, &allowed_comm)?;
        communities = Some(MetricsCommunitiesSection {
            count: result.communities.len(),
            modularity: result.modularity,
            assignments: result.assignments.len(),
        });
    }

    let response = build_metrics_response(pagerank, betweenness, communities);
    if let Some(pr) = &response.pagerank {
        println!("PageRank: {:?}", pr);
    }
    if let Some(bc) = &response.betweenness {
        println!("Betweenness top: {:?}", bc);
    }
    if let Some(cm) = &response.communities {
        println!("Communities: {:?}", cm);
    }
    Ok(())
}

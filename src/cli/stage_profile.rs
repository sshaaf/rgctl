//! Discover pipeline stage timing report (`-v` / grep `[profile]`).

use std::time::Duration;

/// Wall-clock seconds for one discover stage.
#[derive(Debug, Clone, Copy, Default)]
pub struct StageTiming {
    pub secs: f64,
}

/// Aggregated discover timings (seconds, wall clock).
#[derive(Debug, Clone, Default)]
pub struct DiscoverStageReport {
    pub wall_total: StageTiming,
    pub index_pipeline: StageTiming,
    pub index_extract: StageTiming,
    pub index_graph_build: StageTiming,
    pub topology: StageTiming,
    pub community: StageTiming,
    pub complexity: StageTiming,
    pub centrality: StageTiming,
    pub dependency: StageTiming,
    pub security: StageTiming,
    pub cfg_total: StageTiming,
    pub cfg_build: StageTiming,
    pub cfg_dominator: StageTiming,
    pub cfg_pdg: StageTiming,
    pub cfg_taint: StageTiming,
    pub cfg_archive: StageTiming,
    pub field_write: StageTiming,
    pub blast_build: StageTiming,
    pub blast_query: StageTiming,
    pub blast_snapshot: StageTiming,
    pub macro_index: StageTiming,
    pub save_analysis: StageTiming,
    pub save_tracker: StageTiming,
    pub save_snapshot: StageTiming,
    pub save_dashboard: StageTiming,
    pub migration_plan: StageTiming,
    pub kantra_eval: StageTiming,
    pub kantra_index: StageTiming,
    pub kantra_load: StageTiming,
    pub kantra_filecontent: StageTiming,
    pub kantra_referenced: StageTiming,
    pub kantra_compose: StageTiming,
    pub kantra_enrich: StageTiming,
    pub kantra_violates: StageTiming,
    /// Absolute high-water RSS for the whole discover run (ingest + analysis).
    pub peak_rss_mb: f64,
    /// Peak RSS during ingest (index → early mmap → complexity → CSR → drop backend).
    pub ingest_peak_rss_mb: f64,
    /// Peak RSS during analysis after fat `CodeGraph` drop (community → save).
    pub analysis_peak_rss_mb: f64,
    pub functions: usize,
    pub nodes: usize,
    pub cfg_enabled: bool,
    pub security_enabled: bool,
}

impl DiscoverStageReport {
    pub fn record(&self) {
        let post_index = self.community.secs
            + self.complexity.secs
            + self.centrality.secs
            + self.dependency.secs
            + self.security.secs
            + self.cfg_total.secs
            + self.cfg_archive.secs
            + self.field_write.secs
            + self.blast_build.secs
            + self.blast_query.secs
            + self.blast_snapshot.secs
            + self.macro_index.secs
            + self.save_analysis.secs
            + self.save_tracker.secs
            + self.save_snapshot.secs
            + self.save_dashboard.secs
            + self.migration_plan.secs
            + self.kantra_index.secs
            + self.kantra_eval.secs
            + self.kantra_enrich.secs
            + self.kantra_violates.secs;

        let stages: &[(&str, f64)] = &[
            ("index_extract", self.index_extract.secs),
            ("index_graph_build", self.index_graph_build.secs),
            ("topology", self.topology.secs),
            ("community", self.community.secs),
            ("complexity", self.complexity.secs),
            ("centrality", self.centrality.secs),
            ("dependency", self.dependency.secs),
            ("security", self.security.secs),
            ("cfg_total", self.cfg_total.secs),
            ("cfg_archive", self.cfg_archive.secs),
            ("field_write", self.field_write.secs),
            ("blast_build", self.blast_build.secs),
            ("blast_query", self.blast_query.secs),
            ("blast_snapshot", self.blast_snapshot.secs),
            ("macro_index", self.macro_index.secs),
            ("save_analysis", self.save_analysis.secs),
            ("save_tracker", self.save_tracker.secs),
            ("save_snapshot", self.save_snapshot.secs),
            ("save_dashboard", self.save_dashboard.secs),
            ("migration_plan", self.migration_plan.secs),
            ("kantra_index", self.kantra_index.secs),
            ("kantra_eval", self.kantra_eval.secs),
            ("kantra_load", self.kantra_load.secs),
            ("kantra_filecontent", self.kantra_filecontent.secs),
            ("kantra_referenced", self.kantra_referenced.secs),
            ("kantra_compose", self.kantra_compose.secs),
            ("kantra_enrich", self.kantra_enrich.secs),
            ("kantra_violates", self.kantra_violates.secs),
        ];

        tracing::info!(
            target: "profile",
            wall_secs = self.wall_total.secs,
            index_secs = self.index_pipeline.secs,
            post_index_secs = post_index,
            peak_rss_mb = self.peak_rss_mb,
            ingest_peak_rss_mb = self.ingest_peak_rss_mb,
            analysis_peak_rss_mb = self.analysis_peak_rss_mb,
            functions = self.functions,
            nodes = self.nodes,
            cfg = self.cfg_enabled,
            security = self.security_enabled,
            "[profile] discover summary"
        );

        for (name, secs) in stages {
            if *secs <= 0.0 {
                continue;
            }
            let pct = if self.wall_total.secs > 0.0 {
                100.0 * secs / self.wall_total.secs
            } else {
                0.0
            };
            tracing::info!(
                target: "profile",
                stage = name,
                secs,
                pct_wall = pct,
                "[profile] stage"
            );
        }

        for (name, secs) in [
            ("cfg_build", self.cfg_build.secs),
            ("cfg_dominator", self.cfg_dominator.secs),
            ("cfg_pdg", self.cfg_pdg.secs),
            ("cfg_taint", self.cfg_taint.secs),
        ] {
            if secs <= 0.0 {
                continue;
            }
            tracing::info!(
                target: "profile",
                stage = name,
                cpu_secs = secs,
                cfg_wall_secs = self.cfg_total.secs,
                "[profile] cfg cpu stage (sum across threads; pct vs cfg_total wall)"
            );
        }
    }
}

pub fn secs(d: Duration) -> f64 {
    d.as_secs_f64()
}

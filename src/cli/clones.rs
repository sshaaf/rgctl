//! `rgctl clones` — exact Type-1 (`code_hash`) and bloom candidates (`token_bloom`).
//!
//! Distinct from `rgctl semantic query` (NL / embedding nearest neighbors).

use super::args::OutputFormat;
use super::context::CliContext;
use anyhow::{Context, Result, bail};
use rgctl_analysis::{
    BloomCloneOptions, CLONE_REPORT_SCHEMA_VERSION, CloneFilters, DEFAULT_BLOOM_THRESHOLD,
    DEFAULT_MIN_LOC, ExactCloneOptions, MODE_BLOOM, MODE_EXACT, bloom_clones_with_cache,
    build_bloom_report, build_exact_report, exact_clones_with_cache, parse_mode, save_sidecar,
};
use rgctl_error::Error as GraphError;
use rgctl_graph::{
    QueryFilters, STRUCTURED_QUERY_SCHEMA_VERSION, StructuredQuery, schema::NodeType,
};

/// CLI arguments for `rgctl clones`.
#[derive(Debug, Clone)]
pub struct ClonesArgs {
    /// Optional symbol seed (clones of this function).
    pub symbol: Option<String>,
    /// Detection mode (`exact` | `bloom`).
    pub mode: String,
    /// Minimum LOC (default [`DEFAULT_MIN_LOC`]).
    pub min_loc: Option<usize>,
    /// Path exclude needles / globs.
    pub exclude: Vec<String>,
    /// Optional language filter.
    pub language: Option<String>,
    /// Bloom min Jaccard (default [`DEFAULT_BLOOM_THRESHOLD`]).
    pub threshold: Option<f64>,
    /// Disambiguation: file glob.
    pub file: Option<String>,
    /// Disambiguation: class name.
    pub class: Option<String>,
    /// Disambiguation: definition line.
    pub line: Option<usize>,
    /// Persist sidecar for full-repo reports (default true).
    pub write: bool,
    /// Skip sidecar cache / write.
    pub no_cache: bool,
}

impl Default for ClonesArgs {
    fn default() -> Self {
        Self {
            symbol: None,
            mode: MODE_EXACT.to_string(),
            min_loc: Some(DEFAULT_MIN_LOC),
            exclude: Vec::new(),
            language: None,
            threshold: None,
            file: None,
            class: None,
            line: None,
            write: true,
            no_cache: false,
        }
    }
}

fn map_sq_err(ctx: &CliContext, err: GraphError) -> anyhow::Error {
    if let GraphError::AmbiguousSymbol {
        name,
        count,
        candidates,
    } = &err
    {
        if ctx.format == OutputFormat::Json {
            let envelope = serde_json::json!({
                "schema_version": STRUCTURED_QUERY_SCHEMA_VERSION,
                "error": "ambiguous_symbol",
                "name": name,
                "count": count,
                "candidates": candidates,
            });
            let _ = ctx.emit_json_value(&envelope);
        } else if !candidates.is_empty() {
            eprintln!("Ambiguous symbol '{name}': {count} matches. Candidates:");
            for c in candidates.iter().take(20) {
                let file = c.file.as_deref().unwrap_or("?");
                let line = c
                    .line
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "?".into());
                eprintln!(
                    "  - id={} type={} file={file}:{line} name={}",
                    c.id, c.node_type, c.name
                );
            }
            eprintln!("Disambiguate with --file, --class, and/or --line.");
        }
    }
    anyhow::anyhow!("{err}")
}

/// Run `rgctl clones`.
pub fn run(ctx: &CliContext, args: ClonesArgs) -> Result<()> {
    let mode = parse_mode(&args.mode).map_err(|e| anyhow::anyhow!("{e}"))?;

    let store = ctx
        .open_snapshot_store()?
        .context("Graph snapshot not found (run `rgctl discover` first)")?;

    let filters = CloneFilters {
        min_loc: args.min_loc.or(Some(DEFAULT_MIN_LOC)),
        exclude: args.exclude,
        language: args.language,
    };

    let seed_id = if let Some(ref symbol) = args.symbol {
        let q = StructuredQuery::new(store.as_ref());
        let qf = QueryFilters {
            node_type: Some(NodeType::Function),
            file_glob: args.file.clone(),
            class: args.class.clone(),
            line: args.line,
            lang: filters.language.clone(),
            ..QueryFilters::default()
        };
        let node = q
            .resolve_symbol(symbol, &qf)
            .map_err(|e| map_sq_err(ctx, e))?;
        Some(node.id)
    } else {
        None
    };

    let report = match mode {
        MODE_EXACT => {
            let opts = ExactCloneOptions {
                filters: filters.clone(),
                seed_id,
            };
            if args.no_cache || seed_id.is_some() {
                let report = build_exact_report(store.as_ref(), opts)?;
                if args.write && seed_id.is_none() && !args.no_cache {
                    save_sidecar(&ctx.repo, &report)?;
                }
                report
            } else {
                exact_clones_with_cache(store.as_ref(), &ctx.repo, opts, args.write)?
            }
        }
        MODE_BLOOM => {
            let threshold = args.threshold.unwrap_or(DEFAULT_BLOOM_THRESHOLD);
            if !(0.0..=1.0).contains(&threshold) {
                bail!("--threshold must be between 0 and 1");
            }
            let opts = BloomCloneOptions {
                filters: filters.clone(),
                seed_id,
                threshold,
                ..BloomCloneOptions::default()
            };
            if args.no_cache || seed_id.is_some() {
                let report = build_bloom_report(store.as_ref(), opts)?;
                if args.write && seed_id.is_none() && !args.no_cache {
                    save_sidecar(&ctx.repo, &report)?;
                }
                report
            } else {
                bloom_clones_with_cache(store.as_ref(), &ctx.repo, opts, args.write)?
            }
        }
        other => bail!("unsupported clone mode '{other}'"),
    };

    if ctx.format == OutputFormat::Json {
        return ctx.emit_json_value(&serde_json::to_value(&report)?);
    }

    let cand = if report.candidates { " candidates" } else { "" };
    ctx.stdout_line(&format!(
        "clone mode={}{} schema_version={} groups={} digest={}",
        report.mode,
        cand,
        report.schema_version.max(CLONE_REPORT_SCHEMA_VERSION),
        report.group_count,
        &report.graph_digest[..report.graph_digest.len().min(12)]
    ))?;
    if let Some(t) = report.threshold {
        ctx.stdout_line(&format!("threshold={t:.3}"))?;
    }
    if let Some(ref seed) = report.seed {
        let file = seed.file.as_deref().unwrap_or("?");
        ctx.stdout_line(&format!(
            "seed: {} @ {}:{}",
            seed.name,
            file,
            seed.start_line.unwrap_or(0)
        ))?;
    }
    for g in &report.groups {
        let score = g
            .score
            .map(|s| format!(" score={s:.3}"))
            .unwrap_or_default();
        let hash = g
            .hash
            .as_deref()
            .map(|h| format!(" hash={}…", &h[..h.len().min(12)]))
            .unwrap_or_default();
        ctx.stdout_line(&format!(
            "\n[{}] size={}{score}{hash}",
            g.mode, g.size
        ))?;
        for m in &g.members {
            let file = m.file.as_deref().unwrap_or("?");
            let line = m.start_line.unwrap_or(0);
            ctx.stdout_line(&format!("  - {}  {}:{}", m.name, file, line))?;
        }
    }
    if report.groups.is_empty() {
        ctx.stdout_line(&format!(
            "(no {} clone groups matched filters)",
            report.mode
        ))?;
    }
    Ok(())
}

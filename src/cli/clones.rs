//! `rgctl clones` — exact (Type-1) clone groups via `code_hash`.
//!
//! Distinct from `rgctl semantic query` (NL / embedding nearest neighbors).
//! Pairwise/group duplicate detection over identical function bodies.

use super::args::OutputFormat;
use super::context::CliContext;
use anyhow::{Context, Result, bail};
use rgctl_analysis::{
    CLONE_REPORT_SCHEMA_VERSION, CloneFilters, DEFAULT_MIN_LOC, ExactCloneOptions, MODE_EXACT,
    build_exact_report, exact_clones_with_cache, parse_mode, save_sidecar,
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
    /// Detection mode (MVP: `exact` only).
    pub mode: String,
    /// Minimum LOC (default [`DEFAULT_MIN_LOC`]).
    pub min_loc: Option<usize>,
    /// Path exclude needles / globs.
    pub exclude: Vec<String>,
    /// Optional language filter.
    pub language: Option<String>,
    /// Disambiguation: file glob.
    pub file: Option<String>,
    /// Disambiguation: class name.
    pub class: Option<String>,
    /// Disambiguation: definition line.
    pub line: Option<usize>,
    /// Persist `.rgctl/clones.json` for full-repo exact reports (default true).
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
    if mode != MODE_EXACT {
        bail!("unsupported clone mode");
    }

    let store = ctx
        .open_snapshot_store()?
        .context("Graph snapshot not found (run `rgctl discover` first)")?;

    let mut filters = CloneFilters {
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
        // Symbol-scoped: do not apply min_loc to the seed's group discovery of the seed itself —
        // still filter other members via min_loc. Keep filters as configured.
        let _ = &mut filters;
        Some(node.id)
    } else {
        None
    };

    let opts = ExactCloneOptions {
        filters: filters.clone(),
        seed_id,
    };

    let report = if args.no_cache || seed_id.is_some() {
        let report = build_exact_report(store.as_ref(), opts)?;
        if args.write && seed_id.is_none() && !args.no_cache {
            save_sidecar(&ctx.repo, &report)?;
        }
        report
    } else {
        exact_clones_with_cache(store.as_ref(), &ctx.repo, opts, args.write)?
    };

    if ctx.format == OutputFormat::Json {
        return ctx.emit_json_value(&serde_json::to_value(&report)?);
    }

    ctx.stdout_line(&format!(
        "clone mode={} schema_version={} groups={} digest={}",
        report.mode,
        report.schema_version.max(CLONE_REPORT_SCHEMA_VERSION),
        report.group_count,
        &report.graph_digest[..report.graph_digest.len().min(12)]
    ))?;
    if let Some(ref seed) = report.seed {
        let file = seed.file.as_deref().unwrap_or("?");
        ctx.stdout_line(&format!("seed: {} @ {}:{}", seed.name, file, seed.start_line.unwrap_or(0)))?;
    }
    for g in &report.groups {
        let hash = g.hash.as_deref().unwrap_or("?");
        ctx.stdout_line(&format!(
            "\n[{}] size={} hash={}…",
            g.mode,
            g.size,
            &hash[..hash.len().min(12)]
        ))?;
        for m in &g.members {
            let file = m.file.as_deref().unwrap_or("?");
            let line = m.start_line.unwrap_or(0);
            ctx.stdout_line(&format!("  - {}  {}:{}", m.name, file, line))?;
        }
    }
    if report.groups.is_empty() {
        ctx.stdout_line("(no exact clone groups matched filters)")?;
    }
    Ok(())
}

//! `rgctl semantic` — opt-in function semantic index and Hamming search.

use super::args::OutputFormat;
use super::context::CliContext;
use super::semantic_output::{
    build_index_response, index_response_to_json, query_response_to_json,
};
use crate::analysis::{
    BlastRadiusEngine, BlastSummaryProvider, CODE_DAEMON_MRL_DIMS, EmbedderChoice,
    SemanticBuildOptions, SemanticExpansion, SemanticIndex, VOCAB_ACCUMULATE_DISTILLED_ID,
    blast_summary_from_result, build_index, default_tokenizer_path, distill_vocab_matrix,
    parse_vocab_token_list, resolve_embedder, try_load_engine, validate_mrl_dimensions,
};
use crate::graph::backend::{GraphBackend, MemoryBackend};
use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum CliEmbedderKind {
    /// Deterministic sign-hash (no model files).
    Hash,
    /// Compiled vocab bag-of-tokens (offline, no ONNX).
    #[default]
    Vocab,
    /// Generic ONNX `--model` (hash tokenization, or `--tokenizer` for SentencePiece).
    Onnx,
    /// [`code-daemon-embed-v1`](https://huggingface.co/faxenoff/code-daemon-embed-v1) code retriever.
    CodeDaemon,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum CliSemanticScope {
    /// Rank function symbols (default).
    Function,
    /// Rank documentation headings (`:Module` with `kind=heading`).
    Docs,
    /// Rank functions and documentation sections.
    All,
    /// Rank communities via pooled member embeddings.
    Community,
}

fn cli_scope_to_build(scope: CliSemanticScope) -> crate::analysis::SemanticIndexScope {
    match scope {
        CliSemanticScope::Function => crate::analysis::SemanticIndexScope::Functions,
        CliSemanticScope::Docs => crate::analysis::SemanticIndexScope::Docs,
        CliSemanticScope::All => crate::analysis::SemanticIndexScope::All,
        CliSemanticScope::Community => crate::analysis::SemanticIndexScope::Functions,
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum CliExpandMode {
    Neighbors,
    Blast,
    Gql,
    All,
}

pub struct SemanticIndexArgs {
    pub dimensions: usize,
    pub incremental: bool,
    pub embedder: CliEmbedderKind,
    pub model: Option<PathBuf>,
    pub tokenizer: Option<PathBuf>,
    pub diffuse: bool,
    pub diffuse_alpha: f64,
    pub diffuse_iters: usize,
    pub diffuse_bidirectional: bool,
    pub scope: CliSemanticScope,
    /// Re-read function source and append body identifier tokens (off by default).
    pub embed_bodies: bool,
}

pub struct SemanticDistillArgs {
    pub output: PathBuf,
    pub tokens: Option<PathBuf>,
    pub embedder: CliEmbedderKind,
    pub dimensions: usize,
    pub batch_size: usize,
    pub model: Option<PathBuf>,
    pub tokenizer: Option<PathBuf>,
}

pub struct SemanticQueryArgs {
    pub query: String,
    pub limit: usize,
    pub expand: Option<CliExpandMode>,
    pub expand_depth: usize,
    pub model: Option<PathBuf>,
    pub tokenizer: Option<PathBuf>,
    pub fusion: bool,
    pub candidate_pool: usize,
    pub keyword_and: bool,
    pub scope: CliSemanticScope,
}

pub fn run_index(ctx: &CliContext, args: SemanticIndexArgs) -> Result<()> {
    run_index_with_emit(ctx, args, true)
}

pub(crate) fn run_index_with_emit(
    ctx: &CliContext,
    args: SemanticIndexArgs,
    emit_cli: bool,
) -> Result<()> {
    if args.dimensions == 0 || !args.dimensions.is_multiple_of(8) {
        bail!("--dimensions must be a positive multiple of 8");
    }
    if args.embedder == CliEmbedderKind::Onnx && args.model.is_none() {
        bail!("--model is required when --embedder onnx");
    }

    let graph = ctx.load_graph()?;
    let graph_digest = ctx.graph_digest()?;
    let path = semantic_index_path(ctx);

    let existing = if args.incremental && path.is_file() {
        Some(SemanticIndex::load(&path).with_context(|| format!("load {}", path.display()))?)
    } else {
        None
    };

    let (model_path, embedder_choice) = resolve_embedder_choice(&ctx.repo, &args)?;
    if args.embedder == CliEmbedderKind::CodeDaemon {
        validate_mrl_dimensions(args.dimensions).with_context(|| {
            format!(
                "code-daemon supports MRL dims {:?} (multiples of 8, max 768)",
                CODE_DAEMON_MRL_DIMS
            )
        })?;
    }

    let embedder = resolve_embedder(&embedder_choice, args.dimensions)?;

    let (stored_model, stored_tokenizer) =
        store_paths(&ctx.repo, &embedder_choice, &model_path, &args.tokenizer);

    let (index, stats) = build_index(
        graph.backend(),
        embedder.as_ref(),
        SemanticBuildOptions {
            dimensions: args.dimensions,
            graph_digest,
            incremental: args.incremental,
            existing,
            model_path: stored_model,
            tokenizer_path: stored_tokenizer,
            repo_root: Some(ctx.repo.clone()),
            embed_bodies: args.embed_bodies,
            diffuse: if args.diffuse {
                Some(crate::analysis::DiffuseConfig {
                    alpha: args.diffuse_alpha,
                    iterations: args.diffuse_iters,
                    mode: if args.diffuse_bidirectional {
                        crate::analysis::DiffuseNeighborMode::Bidirectional
                    } else {
                        crate::analysis::DiffuseNeighborMode::Callees
                    },
                })
            } else {
                None
            },
            scope: cli_scope_to_build(args.scope),
        },
    )?;

    index
        .save(&path)
        .with_context(|| format!("write semantic index {}", path.display()))?;

    let response = build_index_response(
        &index.model_id,
        index.dimensions,
        index.len(),
        &path.display().to_string(),
        index.graph_digest.clone(),
        Some(stats),
    );

    if emit_cli {
        if ctx.format == OutputFormat::Json {
            ctx.emit_json_value(&index_response_to_json(&response))?;
        } else {
            let scope_label = match args.scope {
                CliSemanticScope::Function => "functions",
                CliSemanticScope::Docs => "doc sections",
                CliSemanticScope::All => "entries",
                CliSemanticScope::Community => "functions",
            };
            println!(
                "Indexed {} {} ({}, {} dims) → {}",
                response.functions_indexed,
                scope_label,
                response.model_id,
                response.dimensions,
                response.path
            );
            if let Some(build_stats) = &response.build_stats {
                println!(
                    "  incremental: {} reused, {} embedded, {} removed",
                    build_stats.reused, build_stats.embedded, build_stats.removed
                );
            }
        }
    }

    Ok(())
}

pub fn run_distill(ctx: &CliContext, args: SemanticDistillArgs) -> Result<()> {
    if args.dimensions == 0 || !args.dimensions.is_multiple_of(8) {
        bail!("--dimensions must be a positive multiple of 8");
    }
    if args.embedder == CliEmbedderKind::Vocab {
        bail!("distill teacher cannot be vocab; use --embedder code-daemon (or hash / onnx)");
    }
    if args.embedder == CliEmbedderKind::Onnx && args.model.is_none() {
        bail!("--model is required when --embedder onnx");
    }

    let token_text = if let Some(path) = &args.tokens {
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?
    } else {
        crate::analysis::DEFAULT_VOCAB_TOKEN_LIST.to_string()
    };
    let tokens = parse_vocab_token_list(&token_text);
    if tokens.is_empty() {
        bail!("token list is empty after filtering");
    }

    let index_args = SemanticIndexArgs {
        dimensions: args.dimensions,
        incremental: false,
        embedder: args.embedder,
        model: args.model.clone(),
        tokenizer: args.tokenizer.clone(),
        diffuse: false,
        diffuse_alpha: 0.0,
        diffuse_iters: 0,
        diffuse_bidirectional: false,
        scope: CliSemanticScope::Function,
        embed_bodies: false,
    };
    let (_model_path, embedder_choice) = resolve_embedder_choice(&ctx.repo, &index_args)?;
    if args.embedder == CliEmbedderKind::CodeDaemon {
        validate_mrl_dimensions(args.dimensions).with_context(|| {
            format!(
                "code-daemon supports MRL dims {:?} (multiples of 8, max 768)",
                CODE_DAEMON_MRL_DIMS
            )
        })?;
    }
    let embedder = resolve_embedder(&embedder_choice, args.dimensions)?;
    let blob = distill_vocab_matrix(embedder.as_ref(), &tokens, args.dimensions, args.batch_size)?;
    if let Some(parent) = args.output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
    }
    std::fs::write(&args.output, &blob)
        .with_context(|| format!("write {}", args.output.display()))?;

    if ctx.format == OutputFormat::Json {
        let doc = serde_json::json!({
            "schema_version": 1,
            "path": args.output.display().to_string(),
            "tokens": tokens.len(),
            "dimensions": args.dimensions,
            "teacher_model_id": embedder.model_id(),
            "compiled_model_id": VOCAB_ACCUMULATE_DISTILLED_ID,
        });
        ctx.emit_json_value(&doc)?;
    } else {
        println!(
            "Wrote {} tokens × {}d ({}) via {}",
            tokens.len(),
            args.dimensions,
            args.output.display(),
            embedder.model_id()
        );
        println!(
            "Copy to crates/rgctl-analysis/assets/vocab_matrix.bin and rebuild to activate {VOCAB_ACCUMULATE_DISTILLED_ID}"
        );
    }
    Ok(())
}

pub fn run_query(ctx: &CliContext, args: SemanticQueryArgs) -> Result<()> {
    let path = semantic_index_path(ctx);
    if !path.is_file() {
        bail!(
            "Semantic index not found at {} (run `rgctl semantic index` first)",
            path.display()
        );
    }

    let index = SemanticIndex::load(&path)
        .with_context(|| format!("load semantic index {}", path.display()))?;

    super::semantic_api::validate_index_scope(&index, args.scope)?;

    let graph = ctx.load_graph()?;
    let response = if ctx.format == OutputFormat::Json && args.expand.is_none() {
        let mut session = rgctl_service::Session::new(&ctx.repo);
        let value = rgctl_service::execute(
            &mut session,
            rgctl_service::Command::Search(rgctl_service::SearchArgs {
                text: args.query.clone(),
                scope: match args.scope {
                    CliSemanticScope::Function => rgctl_service::SearchScope::Function,
                    CliSemanticScope::Docs => rgctl_service::SearchScope::Docs,
                    CliSemanticScope::Community => rgctl_service::SearchScope::Community,
                    CliSemanticScope::All => rgctl_service::SearchScope::All,
                },
                limit: Some(args.limit),
            }),
        )?;
        ctx.emit_json_value(&value)?;
        return Ok(());
    } else {
        super::semantic_api::execute_semantic_query(&ctx.repo, &graph, &index, &args)?
    };

    if ctx.format == OutputFormat::Json {
        ctx.emit_json_value(&query_response_to_json(&response))?;
    } else {
        if response.hits.is_empty() {
            println!("No matches for {:?}", args.query);
            return Ok(());
        }
        for hit in &response.hits {
            let label = hit.qualified_name.as_deref().unwrap_or(&hit.name);
            let file = hit
                .file_path
                .as_deref()
                .map(|p| format!(" ({p})"))
                .unwrap_or_default();
            let ranking = hit
                .ranking
                .as_deref()
                .map(|r| format!(" [{r}]"))
                .unwrap_or_default();
            println!(
                "{label}{file}  distance={} score={:.3}{}{}",
                hit.distance,
                hit.score,
                hit.fused_score
                    .map(|score| format!(" fused={score:.3}"))
                    .unwrap_or_default(),
                ranking
            );
        }
        if let Some(exp) = &response.expansion {
            print_expansion_text(exp);
        }
    }

    Ok(())
}

fn resolve_embedder_choice(
    _repo: &Path,
    args: &SemanticIndexArgs,
) -> Result<(PathBuf, EmbedderChoice)> {
    match args.embedder {
        CliEmbedderKind::Hash => Ok((PathBuf::new(), EmbedderChoice::SignHash)),
        CliEmbedderKind::Vocab => Ok((PathBuf::new(), EmbedderChoice::Vocab)),
        CliEmbedderKind::Onnx => {
            let model = args.model.clone().expect("checked");
            Ok((
                model.clone(),
                EmbedderChoice::Onnx {
                    model,
                    tokenizer: args.tokenizer.clone(),
                },
            ))
        }
        CliEmbedderKind::CodeDaemon => {
            let model = args
                .model
                .clone()
                .filter(|path| !path.as_os_str().is_empty() && path.is_file());
            Ok((
                model.clone().unwrap_or_default(),
                EmbedderChoice::CodeDaemon {
                    model,
                    tokenizer: args.tokenizer.clone(),
                },
            ))
        }
    }
}

fn store_paths(
    repo: &Path,
    choice: &EmbedderChoice,
    _model_path: &Path,
    explicit_tokenizer: &Option<PathBuf>,
) -> (Option<String>, Option<String>) {
    match choice {
        EmbedderChoice::SignHash | EmbedderChoice::Vocab => (None, None),
        EmbedderChoice::Onnx { model, tokenizer } => (
            Some(model.display().to_string()),
            tokenizer
                .clone()
                .or_else(|| explicit_tokenizer.clone())
                .map(|p| p.display().to_string()),
        ),
        EmbedderChoice::CodeDaemon { model, .. } => match model {
            Some(model_path) => {
                let tok = explicit_tokenizer
                    .clone()
                    .or_else(|| {
                        let beside = default_tokenizer_path(model_path.parent().unwrap_or(repo));
                        beside.is_file().then_some(beside)
                    })
                    .or_else(|| {
                        let default = default_tokenizer_path(repo);
                        default.is_file().then_some(default)
                    });
                (
                    Some(model_path.display().to_string()),
                    tok.map(|p| p.display().to_string()),
                )
            }
            None => (None, None),
        },
    }
}

pub(crate) struct EngineBlastProvider<'a> {
    pub(crate) repo: &'a Path,
    pub(crate) backend: &'a MemoryBackend,
    pub(crate) graph_digest: Option<String>,
}

impl BlastSummaryProvider for EngineBlastProvider<'_> {
    fn summarize(
        &self,
        anchor_id: Uuid,
    ) -> rgctl_error::Result<Option<crate::analysis::SemanticBlastSummary>> {
        let result = if let Some(digest) = self.graph_digest.as_deref() {
            if let Some(engine) = try_load_engine(self.repo, digest)? {
                engine.analyze(anchor_id)?
            } else {
                BlastRadiusEngine::build(self.backend)?.analyze(anchor_id)?
            }
        } else {
            BlastRadiusEngine::build(self.backend)?.analyze(anchor_id)?
        };

        let node = self
            .backend
            .get_node(anchor_id)?
            .ok_or_else(|| rgctl_error::Error::NodeNotFound(anchor_id.to_string()))?;

        Ok(Some(blast_summary_from_result(
            &crate::analysis::SemanticEntry {
                node_id: anchor_id,
                name: node.name.to_string(),
                qualified_name: node.qualified_name.as_ref().map(|s| s.to_string()),
                file_path: node.file_path.as_ref().map(|s| s.to_string()),
                code_hash: node.code_hash.as_ref().map(|s| s.to_string()),
                node_type: None,
                kind: None,
            },
            result.direct_caller_ids.len(),
            result.impact_zone_ids.len(),
            result.score,
        )))
    }
}

pub(crate) fn expand_gql_neighbors(
    backend: &MemoryBackend,
    hits: &[crate::analysis::SemanticHit],
    depth: usize,
    anchor_limit: usize,
) -> Result<Vec<crate::analysis::SemanticExpandedNode>> {
    use crate::analysis::SemanticExpandedNode;
    use crate::gql::execute;

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for hit in hits.iter().take(anchor_limit) {
        let name = hit.entry.name.replace('\'', "''");
        let query = format!(
            "MATCH (a:Function)-[:CALLS*1..{depth}]->(b:Function) WHERE a.name = '{name}' RETURN b LIMIT 20"
        );
        let result = execute(backend, &query)?;
        for row in result.rows {
            for node in row.values() {
                if !seen.insert(node.id) {
                    continue;
                }
                out.push(SemanticExpandedNode {
                    node_id: node.id.to_string(),
                    name: node.name.to_string(),
                    qualified_name: node.qualified_name.as_ref().map(|s| s.to_string()),
                    file_path: node.file_path.as_ref().map(|s| s.to_string()),
                    relation: "gql_calls".into(),
                    anchor_node_id: Some(hit.entry.node_id.to_string()),
                });
            }
        }
    }
    Ok(out)
}

fn print_expansion_text(exp: &SemanticExpansion) {
    if let Some(neighbors) = &exp.neighbors {
        if !neighbors.is_empty() {
            println!("\nNeighbors:");
            for node in neighbors.iter().take(10) {
                println!("  {} [{}]", node.name, node.relation);
            }
        }
    }
    if let Some(blast) = &exp.blast {
        if !blast.is_empty() {
            println!("\nBlast radius:");
            for summary in blast {
                println!(
                    "  {}  callers={} impact={} score={:.1}",
                    summary.anchor_name, summary.direct_callers, summary.impact_zone, summary.score
                );
            }
        }
    }
    if let Some(gql) = &exp.gql {
        if !gql.is_empty() {
            println!("\nGQL expansion:");
            for node in gql.iter().take(10) {
                println!("  {} [{}]", node.name, node.relation);
            }
        }
    }
}

fn semantic_index_path(ctx: &CliContext) -> PathBuf {
    SemanticIndex::default_path(&ctx.repo)
}

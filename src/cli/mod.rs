//! rgBuilder CLI command definitions and dispatch.

mod args;
mod blast_radius;
pub mod blast_radius_output;
mod check;
pub mod check_output;
mod communities;
mod context;
mod cpg;
mod daemon;
mod discover;
mod discover_cfg;
mod discover_impl;
pub mod discover_output;
mod export;
mod gql;
pub mod gql_output;
mod http_serve;
mod inspect;
pub mod inspect_output;
mod install;
pub mod install_output;
mod markup;
mod mcp_serve;
mod metrics;
pub mod metrics_output;
mod pipeline_session;
pub mod pipeline_status;
mod policy_file;
#[allow(dead_code)]
mod query_daemon;
mod semantic;
mod semantic_api;
pub mod semantic_output;
mod slice;
pub mod slice_output;
mod stage_profile;

pub use args::OutputFormat;

use crate::BUILD_INFO;
use crate::analysis::{DEFAULT_CANDIDATE_POOL, DEFAULT_EMBEDDING_DIMENSIONS};
use args::{
    ExportFormat, InspectLayer, PdgEdgeLayer, ServeMode, SkillHost, SliceDirection, SliceView,
};
use clap::{Parser, Subcommand};
use context::CliContext;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "rgctl")]
#[command(about = "A code knowledge graph built for LLM agents", version = BUILD_INFO)]
pub struct Cli {
    /// Path to the graph cache database
    #[arg(short = 'd', long = "db", global = true)]
    pub db: Option<std::path::PathBuf>,

    /// Target repository root
    #[arg(short = 'r', long = "repo", global = true)]
    pub repo: Option<std::path::PathBuf>,

    /// Output format
    #[arg(short = 'f', long = "format", value_enum, global = true)]
    pub format: Option<OutputFormat>,

    /// Write output to file instead of stdout
    #[arg(short = 'o', long = "output", global = true)]
    pub output: Option<std::path::PathBuf>,

    /// Run in-process; do not contact or start a daemon
    #[arg(long = "no-daemon", global = true)]
    pub no_daemon: bool,

    /// Daemon workspace root (default: $HOME → state under ~/.rgbuilder/)
    #[arg(long = "daemon-home", value_name = "PATH", global = true)]
    pub daemon_home: Option<std::path::PathBuf>,

    /// Fail if no daemon is running; do not auto-start
    #[arg(long = "fail-if-no-daemon", global = true)]
    pub fail_if_no_daemon: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Index and analyze a codebase
    Discover {
        /// Repository path (defaults to --repo or cwd)
        #[arg(value_name = "PATH")]
        path: Option<String>,

        #[arg(short = 'l', long = "languages")]
        languages: Option<String>,

        #[arg(short = 'e', long = "exclude")]
        exclude: Option<String>,

        #[arg(short = 'v', long = "verbose")]
        verbose: bool,

        /// Secret scanning (SecretDetector). Off by default.
        #[arg(long = "with-security", visible_alias = "security")]
        with_security: bool,

        /// Per-function CFG, dominators, and PDG → `.rgbuilder/analysis/` + cfg_pdg archive.
        /// Off by default. Does **not** include discover-time taint (see `--with-taint`).
        #[arg(long = "with-cfg", visible_alias = "cfg")]
        with_cfg: bool,

        /// Discover-time taint analysis (requires CFG/PDG; implies CFG pass if needed).
        /// Off by default. On-demand: `slice ... --taint`.
        #[arg(long = "with-taint")]
        with_taint: bool,

        /// Classify loop-carried data dependencies on the PDG (implies CFG).
        #[arg(long = "with-dfg-loops")]
        with_dfg_loops: bool,

        /// Write coarse AST skeleton archive under `.rgbuilder/analysis/` (implies CFG).
        #[arg(long = "with-ast-skeleton")]
        with_ast_skeleton: bool,

        /// Write legacy JSON graph files (`graph.db` / `graph.json`); default is snapshot-only.
        #[arg(long = "write-json-graph")]
        write_json_graph: bool,

        /// Export the static dashboard bundle under `.rgbuilder/dashboard/`. Off by default.
        #[arg(long = "with-dashboard")]
        with_dashboard: bool,

        /// Write a migration roadmap JSON after analysis (default: `.rgbuilder/migration_plan.json`).
        /// Alias: `--export-migration-plan` (deprecated name).
        #[arg(
            long = "export-migration-hints",
            visible_alias = "export-migration-plan"
        )]
        export_migration_hints: bool,

        /// Compute harmonic centrality (exact or HyperBall). Off by default — needed for
        /// migration ranking; adds ~30s and multi‑GB peak RSS on kernel-scale graphs.
        #[arg(long = "with-harmonic")]
        with_harmonic: bool,

        /// Staged full pipeline: basic discover (queryable snapshot), then CFG + dashboard +
        /// harmonic, then semantic index. Prints a plan first; does not imply taint/security.
        #[arg(long = "full")]
        full: bool,

        /// Strategy preset for migration plan export.
        #[arg(
            long = "migration-preset",
            default_value = "hybrid_default",
            value_parser = ["hybrid_default", "foundational_first", "dense_cluster", "risk_mitigation"]
        )]
        migration_preset: String,

        /// Roadmap sort order for migration plan export: scheduled (dependency-aware) or priority (score rank).
        #[arg(
            long = "migration-order",
            default_value = "scheduled",
            value_parser = ["scheduled", "priority"]
        )]
        migration_order: String,

        /// Flags after `--` (e.g. `discover . -- --full`)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
        extra: Vec<String>,
    },

    /// Execute graph query language
    Gql {
        query: String,

        #[arg(long)]
        explain: bool,

        #[arg(long)]
        macro_name: Option<String>,
    },

    /// Line-level program slice or taint trace
    Slice {
        file: String,

        #[arg(long)]
        line: usize,

        #[arg(long)]
        variable: String,

        #[arg(long)]
        function: Option<String>,

        #[arg(long)]
        language: Option<String>,

        #[arg(long, value_enum, default_value = "backward")]
        direction: SliceDirection,

        #[arg(long)]
        taint: bool,

        #[arg(long, value_enum, default_value = "text")]
        view: SliceView,
    },

    /// Macro impact / blast radius for a symbol
    BlastRadius {
        /// Function symbol name, UUID, or FQN (e.g. `Class::method`)
        #[arg(value_name = "SYMBOL")]
        symbol: String,

        /// Limit upstream impact zone to N incoming call hops (default: full transitive closure)
        #[arg(long, value_name = "N")]
        depth: Option<usize>,

        /// Run statement-level slice hand-off analysis (slow on large graphs)
        #[arg(long)]
        with_slices: bool,

        /// Explicit class or namespace filter
        #[arg(long, value_name = "NAME")]
        class: Option<String>,

        /// Explicit container source file path filter
        #[arg(long, value_name = "PATH")]
        file: Option<String>,

        #[arg(long, value_name = "PATH")]
        policy_file: Option<String>,

        #[arg(long)]
        no_policy: bool,
    },

    /// Inspect raw CFG / PDG / dominance for a function symbol
    Inspect {
        symbol: String,
        #[command(subcommand)]
        layer: InspectLayer,
    },

    /// Network analytics (PageRank, betweenness, communities)
    Metrics {
        #[arg(long)]
        pagerank: bool,

        #[arg(long)]
        betweenness: bool,

        #[arg(long)]
        communities: bool,

        #[arg(long)]
        iterations: Option<usize>,
    },

    /// Opt-in semantic search over function symbols (separate index artifact)
    Semantic {
        #[command(subcommand)]
        action: SemanticCommands,
    },

    /// List or refresh named communities (analysis overlay)
    Communities {
        #[command(subcommand)]
        action: CommunitiesCommands,
    },

    /// Hybrid CPG façade (topology + CFG/PDG archive)
    Cpg {
        #[command(subcommand)]
        action: CpgCommands,
    },

    /// CI policy gateway
    Check {
        #[arg(long)]
        policy_file: String,
    },

    /// Export graph or projections
    Export {
        #[arg(long = "export-format", value_enum)]
        export_format: ExportFormat,

        #[arg(long = "export-output", value_name = "FILE")]
        export_output: String,

        #[arg(long, default_value = "all")]
        query: String,
    },

    /// Serve the analysis dashboard and GQL query API over HTTP.
    ///
    /// Default: dashboard at `/` and query API at `/api/query` (alias `/graphql`).
    /// Starts the full discover pipeline unless `--no-pipeline` or `--daemon`.
    /// Use `--mode mcp` for MCP stdio (no HTTP). `--daemon` is the legacy blast socket.
    Serve {
        /// Repository path to index (defaults to `--repo` or cwd)
        #[arg(value_name = "PATH")]
        path: Option<String>,

        /// `standard` (HTTP, default) or `mcp` (stdio MCP, no HTTP bind)
        #[arg(long, value_enum, default_value_t = ServeMode::Standard)]
        mode: ServeMode,

        /// Do not auto-run discover; fail fast if artifacts are missing (pre-0.4.7 serve)
        #[arg(long = "no-pipeline")]
        no_pipeline: bool,

        /// Bind host [default: 127.0.0.1]
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// HTTP port [default: 8080]
        #[arg(long, default_value_t = 8080)]
        port: u16,

        /// Dashboard directory [default: `<repo>/.rgbuilder/dashboard`]
        #[arg(long, value_name = "DIR")]
        dashboard_dir: Option<std::path::PathBuf>,

        /// Open the dashboard in the default browser
        #[arg(long)]
        open: bool,

        /// Serve the query API only (no dashboard static files)
        #[arg(long)]
        query_only: bool,

        /// Serve the dashboard only (no query API)
        #[arg(long)]
        dashboard_only: bool,

        /// Background HTTP+MCP daemon (replaces the old blast query socket).
        #[arg(long)]
        daemon: bool,

        /// Worker process (hidden; spawned by `serve --daemon` / `daemon start`)
        #[arg(long = "daemon-worker", hide = true)]
        daemon_worker: bool,

        /// Daemon endpoint path (Unix socket or Windows port file; default under `<repo>/.rgbuilder/`)
        #[arg(long, value_name = "PATH")]
        socket: Option<std::path::PathBuf>,

        /// Daemon idle exit in seconds [default: 300]
        #[arg(long, default_value_t = 300)]
        idle_secs: u64,
    },

    /// Install bundled artifacts into a repository
    Install {
        /// Install the rgBuilder agent skill (Claude Code + Cursor project dirs)
        #[arg(long = "skill")]
        skill: bool,

        /// Which agent skill directories to write
        #[arg(long = "host", value_enum, default_value = "all")]
        host: SkillHost,

        /// Overwrite existing skill files that differ from the bundle
        #[arg(long)]
        force: bool,
    },

    /// Control the background HTTP/MCP daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand)]
pub enum DaemonAction {
    /// Start the daemon (idempotent)
    Start {
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
    },
    /// Stop the daemon
    Stop,
    /// Restart the daemon
    Restart,
    /// Show pid, HTTP, MCP
    Status,
    /// List cached repositories
    List,
}

#[derive(Subcommand)]
pub enum SemanticCommands {
    /// Build `.rgbuilder/semantic_index.bin` from function symbols (not run by default discover)
    Index {
        /// Embedding dimensions before sign quantization (multiple of 8) [default: 256]
        #[arg(long, default_value_t = DEFAULT_EMBEDDING_DIMENSIONS)]
        dimensions: usize,

        /// Reuse embeddings for unchanged `code_hash` values [default: true]
        #[arg(long, default_value_t = true)]
        incremental: bool,

        /// Embedder backend: vocab (default, compiled token table), hash, onnx, or code-daemon
        #[arg(long, value_enum, default_value_t = semantic::CliEmbedderKind::Vocab)]
        embedder: semantic::CliEmbedderKind,

        /// Path to ONNX model (required for `--embedder onnx`; optional for code-daemon)
        #[arg(long, value_name = "PATH")]
        model: Option<std::path::PathBuf>,

        /// SentencePiece tokenizer for ONNX embedders (auto-detected beside `--model` when omitted)
        #[arg(long, value_name = "PATH")]
        tokenizer: Option<std::path::PathBuf>,

        /// Re-read function source and append body identifier tokens (off: declaration metadata only)
        #[arg(long, default_value_t = false)]
        embed_bodies: bool,

        /// Diffuse dense embeddings over the call graph before sign quantization
        #[arg(long, default_value_t = false)]
        diffuse: bool,

        /// Disable call-graph diffusion (default; overrides `--diffuse` when both are set)
        #[arg(long, default_value_t = false)]
        no_diffuse: bool,

        /// Jacobi blend weight toward neighbor mean [default: 0.25]
        #[arg(long, default_value_t = 0.25)]
        diffuse_alpha: f64,

        /// Jacobi diffusion iterations [default: 2]
        #[arg(long, default_value_t = 2)]
        diffuse_iters: usize,

        /// Include callers as well as callees in diffusion neighbors
        #[arg(long, default_value_t = false)]
        diffuse_bidirectional: bool,

        /// Index scope: functions (default), docs (headings), or all
        #[arg(long, value_enum, default_value = "function")]
        scope: semantic::CliSemanticScope,
    },

    /// Distill `vocab_tokens.txt` through a teacher embedder into an RBVK matrix
    Distill {
        /// RBVK destination (copy to crates/rgbuilder-analysis/assets/vocab_matrix.bin)
        #[arg(long = "matrix", value_name = "PATH")]
        matrix: std::path::PathBuf,

        /// Token list (one identifier per line). Defaults to analysis crate assets.
        #[arg(long, value_name = "PATH")]
        tokens: Option<std::path::PathBuf>,

        /// Teacher embedder (code-daemon recommended; hash for tests)
        #[arg(long, value_enum, default_value = "code-daemon")]
        embedder: semantic::CliEmbedderKind,

        /// Output dimensions (multiple of 8) [default: 256]
        #[arg(long, default_value_t = DEFAULT_EMBEDDING_DIMENSIONS)]
        dimensions: usize,

        /// Teacher batch size [default: 32]
        #[arg(long, default_value_t = 32)]
        batch_size: usize,

        /// Path to ONNX model (required for `--embedder onnx`; optional for code-daemon)
        #[arg(long, value_name = "PATH")]
        model: Option<std::path::PathBuf>,

        /// SentencePiece tokenizer for ONNX teachers
        #[arg(long, value_name = "PATH")]
        tokenizer: Option<std::path::PathBuf>,
    },

    /// Hamming nearest-neighbor search over the semantic index
    Query {
        /// Natural-language or keyword query
        #[arg(value_name = "TEXT")]
        query: String,

        /// Maximum hits to return [default: 20]
        #[arg(long, default_value_t = 20)]
        limit: usize,

        /// Expand top hits into graph context: neighbors, blast, gql, or all
        #[arg(long, value_enum, value_name = "MODE")]
        expand: Option<semantic::CliExpandMode>,

        /// CALLS hop depth for neighbor/gql expansion [default: 1]
        #[arg(long, default_value_t = 1)]
        expand_depth: usize,

        /// ONNX model path (when index was built with onnx/code-daemon)
        #[arg(long, value_name = "PATH")]
        model: Option<std::path::PathBuf>,

        /// SentencePiece tokenizer path (ONNX/code-daemon indexes)
        #[arg(long, value_name = "PATH")]
        tokenizer: Option<std::path::PathBuf>,

        /// Disable late fusion re-ranking (pure Hamming top-k)
        #[arg(long)]
        no_fusion: bool,

        /// Hamming candidate pool size before late fusion [default: 256]
        #[arg(long, default_value_t = DEFAULT_CANDIDATE_POOL)]
        candidate_pool: usize,

        /// Require all query keywords to match entry metadata (AND filter)
        #[arg(long)]
        keyword_and: bool,

        /// Search functions (default) or pooled communities
        #[arg(long, value_enum, default_value = "function")]
        scope: semantic::CliSemanticScope,
    },
}

#[derive(Subcommand)]
pub enum CommunitiesCommands {
    /// List communities with heuristic labels
    List,
    /// Refresh heuristic labels and write them into analysis_results.bin
    Label {
        /// Persist updated labels (default: true)
        #[arg(long, default_value_t = true)]
        write: bool,
    },
}

#[derive(Subcommand)]
pub enum CpgCommands {
    /// Show L_proc archive readiness (CFG/PDG)
    Status,
    /// Resolve a function in L_repo and whether L_proc exists
    Function {
        #[arg(value_name = "SYMBOL")]
        symbol: String,
    },
    /// CALL neighborhood for a function
    Calls {
        #[arg(value_name = "SYMBOL")]
        symbol: String,
    },
    /// Field mutations for a type (requires discover --with-cfg)
    Mutations {
        /// Type / class name (e.g. OrderDTO)
        #[arg(long = "type", value_name = "NAME")]
        type_name: String,
        /// Exclude constructor / `<init>` writes
        #[arg(long, default_value_t = false)]
        exclude_ctors: bool,
        /// Optional field name filter
        #[arg(long)]
        member: Option<String>,
        /// Include writes whose receiver type could not be resolved
        #[arg(long, default_value_t = false)]
        include_unresolved: bool,
    },
    /// Data/control flows from a variable at a line (wraps slice)
    Flows {
        file: String,
        #[arg(long)]
        line: usize,
        #[arg(long)]
        variable: String,
        /// Enclosing method / function name
        #[arg(long)]
        function: String,
        #[arg(long)]
        language: Option<String>,
        #[arg(long, value_enum, default_value = "forward")]
        direction: SliceDirection,
        /// Expand may-alias names (copies / field bases) — P3 T2 on-demand
        #[arg(long = "with-alias")]
        with_alias: bool,
    },
    /// Show coarse AST skeleton for a function (requires --with-ast-skeleton)
    Ast {
        #[arg(value_name = "SYMBOL")]
        symbol: String,
    },
    /// Export hybrid CPG view (GraphML / GraphSON)
    Export {
        /// graphml | graphson
        #[arg(long = "format", default_value = "graphson")]
        format: String,
        #[arg(long, value_name = "FILE")]
        output: String,
        /// Keep only nodes whose file_path contains this substring
        #[arg(long = "path-contains")]
        path_contains: Option<String>,
        /// Include PDG DATA_FLOW edges from CFG archive
        #[arg(long = "include-l-proc", default_value_t = true)]
        include_l_proc: bool,
        /// Include field-write sites from the mutation index
        #[arg(long = "include-field-writes", default_value_t = true)]
        include_field_writes: bool,
    },
    /// PDG overlay (wraps `inspect pdg`; prefers live rebuild today)
    Pdg {
        #[arg(value_name = "SYMBOL")]
        symbol: String,
        #[arg(long, value_enum, default_value = "all")]
        edge_layer: PdgEdgeLayer,
        #[arg(long)]
        def_use: bool,
    },
    /// Line-level slice (wraps `slice`)
    Slice {
        file: String,
        #[arg(long)]
        line: usize,
        #[arg(long)]
        variable: String,
        #[arg(long)]
        function: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long, value_enum, default_value = "backward")]
        direction: SliceDirection,
        #[arg(long)]
        taint: bool,
        #[arg(long, value_enum, default_value = "text")]
        view: SliceView,
    },
}

impl Cli {
    pub fn run(self) -> anyhow::Result<()> {
        let wall_start = Instant::now();
        let command_label = command_label_for(&self.command);
        let long_running = matches!(self.command, Commands::Serve { .. });

        let verbose = matches!(self.command, Commands::Discover { verbose: true, .. });
        let discover_json = matches!(self.command, Commands::Discover { .. })
            && self.format.as_ref() == Some(&OutputFormat::Json);
        init_logging(verbose, discover_json);

        if !long_running {
            eprintln!("[>] rgctl {command_label}");
        }

        let command_path = match &self.command {
            Commands::Discover { path: Some(p), .. } | Commands::Serve { path: Some(p), .. } => {
                Some(std::path::PathBuf::from(p))
            }
            _ => None,
        };

        let mut ctx = CliContext::new(
            command_path.or(self.repo),
            self.db,
            self.format.unwrap_or_default(),
            self.output,
            verbose,
        );
        ctx.no_daemon = self.no_daemon || std::env::var_os("RGCTL_NO_DAEMON").is_some();
        ctx.daemon_home = self.daemon_home;
        ctx.fail_if_no_daemon = self.fail_if_no_daemon;

        let result = match self.command {
            Commands::Discover {
                path,
                languages,
                exclude,
                verbose: _,
                with_security,
                with_cfg,
                with_taint,
                with_dfg_loops,
                with_ast_skeleton,
                write_json_graph,
                with_dashboard,
                export_migration_hints,
                with_harmonic,
                mut full,
                migration_preset,
                migration_order,
                extra,
            } => {
                if extra.iter().any(|a| a == "--full") {
                    full = true;
                }
                discover::run(
                    &ctx,
                    discover::DiscoverArgs {
                        path,
                        languages,
                        exclude,
                        with_security,
                        with_cfg,
                        with_taint,
                        with_dfg_loops,
                        with_ast_skeleton,
                        write_json_graph,
                        with_dashboard,
                        export_migration_hints,
                        with_harmonic,
                        full,
                        migration_preset,
                        migration_order,
                        artifact_root: None,
                    },
                )
            }
            Commands::Gql {
                query,
                explain,
                macro_name,
            } => gql::run(
                &ctx,
                gql::GqlArgs {
                    query,
                    explain,
                    macro_name,
                },
            ),
            Commands::Slice {
                file,
                line,
                variable,
                function,
                language,
                direction,
                taint,
                view,
            } => slice::run(
                &ctx,
                slice::SliceArgs {
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
            Commands::BlastRadius {
                symbol,
                depth,
                policy_file,
                no_policy,
                with_slices,
                class,
                file,
            } => blast_radius::run(
                &ctx,
                blast_radius::BlastRadiusArgs {
                    symbol,
                    depth,
                    policy_file,
                    no_policy,
                    with_slices,
                    class,
                    file,
                },
            ),
            Commands::Inspect { symbol, layer } => {
                inspect::run(&ctx, inspect::InspectArgs { symbol, layer })
            }
            Commands::Metrics {
                pagerank,
                betweenness,
                communities,
                iterations,
            } => metrics::run(
                &ctx,
                metrics::MetricsArgs {
                    pagerank,
                    betweenness,
                    communities,
                    iterations,
                },
            ),
            Commands::Semantic { action } => match action {
                SemanticCommands::Index {
                    dimensions,
                    incremental,
                    embedder,
                    model,
                    tokenizer,
                    embed_bodies,
                    diffuse,
                    no_diffuse,
                    diffuse_alpha,
                    diffuse_iters,
                    diffuse_bidirectional,
                    scope,
                } => semantic::run_index(
                    &ctx,
                    semantic::SemanticIndexArgs {
                        dimensions,
                        incremental,
                        embedder,
                        model,
                        tokenizer,
                        embed_bodies,
                        diffuse: diffuse && !no_diffuse,
                        diffuse_alpha,
                        diffuse_iters,
                        diffuse_bidirectional,
                        scope,
                    },
                ),
                SemanticCommands::Distill {
                    matrix,
                    tokens,
                    embedder,
                    dimensions,
                    batch_size,
                    model,
                    tokenizer,
                } => semantic::run_distill(
                    &ctx,
                    semantic::SemanticDistillArgs {
                        output: matrix,
                        tokens,
                        embedder,
                        dimensions,
                        batch_size,
                        model,
                        tokenizer,
                    },
                ),
                SemanticCommands::Query {
                    query,
                    limit,
                    expand,
                    expand_depth,
                    model,
                    tokenizer,
                    no_fusion,
                    candidate_pool,
                    keyword_and,
                    scope,
                } => semantic::run_query(
                    &ctx,
                    semantic::SemanticQueryArgs {
                        query,
                        limit,
                        expand,
                        expand_depth,
                        model,
                        tokenizer,
                        fusion: !no_fusion,
                        candidate_pool,
                        keyword_and,
                        scope,
                    },
                ),
            },
            Commands::Communities { action } => match action {
                CommunitiesCommands::List => communities::run_list(&ctx),
                CommunitiesCommands::Label { write } => {
                    communities::run_label(&ctx, communities::CommunitiesLabelArgs { write })
                }
            },
            Commands::Cpg { action } => {
                let mapped = match action {
                    CpgCommands::Status => cpg::CpgAction::Status,
                    CpgCommands::Function { symbol } => cpg::CpgAction::Function { symbol },
                    CpgCommands::Calls { symbol } => cpg::CpgAction::Calls { symbol },
                    CpgCommands::Mutations {
                        type_name,
                        exclude_ctors,
                        member,
                        include_unresolved,
                    } => cpg::CpgAction::Mutations {
                        type_name,
                        exclude_ctors,
                        member,
                        include_unresolved,
                    },
                    CpgCommands::Flows {
                        file,
                        line,
                        variable,
                        function,
                        language,
                        direction,
                        with_alias,
                    } => cpg::CpgAction::Flows {
                        file,
                        line,
                        variable,
                        function,
                        language,
                        direction,
                        with_alias,
                    },
                    CpgCommands::Ast { symbol } => cpg::CpgAction::Ast { symbol },
                    CpgCommands::Export {
                        format,
                        output,
                        path_contains,
                        include_l_proc,
                        include_field_writes,
                    } => cpg::CpgAction::Export {
                        format,
                        output,
                        path_contains,
                        include_l_proc,
                        include_field_writes,
                    },
                    CpgCommands::Pdg {
                        symbol,
                        edge_layer,
                        def_use,
                    } => cpg::CpgAction::Pdg {
                        symbol,
                        edge_layer,
                        def_use,
                    },
                    CpgCommands::Slice {
                        file,
                        line,
                        variable,
                        function,
                        language,
                        direction,
                        taint,
                        view,
                    } => cpg::CpgAction::Slice {
                        file,
                        line,
                        variable,
                        function,
                        language,
                        direction,
                        taint,
                        view,
                    },
                };
                cpg::run(&ctx, mapped)
            }
            Commands::Check { policy_file } => check::run(&ctx, check::CheckArgs { policy_file }),
            Commands::Export {
                export_format,
                export_output,
                query,
            } => export::run(
                &ctx,
                export::ExportArgs {
                    export_format,
                    export_output,
                    query,
                },
            ),
            Commands::Install { skill, host, force } => {
                install::run(&ctx, install::InstallArgs { skill, host, force })
            }
            Commands::Daemon { action } => match action {
                DaemonAction::Start { host, port } => {
                    let home = daemon::resolve_home(ctx.daemon_home.as_deref())?;
                    let pid = daemon::start(&home, host.as_deref(), port)?;
                    eprintln!("rgctl: daemon pid {pid}");
                    Ok(())
                }
                DaemonAction::Stop => {
                    let home = daemon::resolve_home(ctx.daemon_home.as_deref())?;
                    daemon::stop(&home)
                }
                DaemonAction::Restart => {
                    let home = daemon::resolve_home(ctx.daemon_home.as_deref())?;
                    let pid = daemon::restart(&home)?;
                    eprintln!("rgctl: daemon pid {pid}");
                    Ok(())
                }
                DaemonAction::Status => {
                    let home = daemon::resolve_home(ctx.daemon_home.as_deref())?;
                    print!("{}", daemon::status_text(&home)?);
                    Ok(())
                }
                DaemonAction::List => {
                    let home = daemon::resolve_home(ctx.daemon_home.as_deref())?;
                    print!("{}", daemon::list_text(&home)?);
                    Ok(())
                }
            },
            Commands::Serve {
                path,
                mode,
                no_pipeline,
                host,
                port,
                dashboard_dir,
                open,
                query_only,
                dashboard_only,
                daemon,
                daemon_worker,
                socket: _,
                idle_secs: _,
            } => {
                if daemon_worker {
                    let home = daemon::resolve_home(ctx.daemon_home.as_deref())?;
                    daemon::run_worker(home, host, port)
                } else if daemon {
                    let home = daemon::resolve_home(ctx.daemon_home.as_deref())?;
                    // Foreground serve defaults to 127.0.0.1; daemon uses config (0.0.0.0) unless --host was not the foreground default.
                    let daemon_host = if host == "127.0.0.1" {
                        None
                    } else {
                        Some(host.as_str())
                    };
                    let pid = daemon::start(&home, daemon_host, Some(port))?;
                    eprintln!("rgctl: daemon pid {pid} (background HTTP)");
                    Ok(())
                } else if mode == ServeMode::Mcp {
                    if ctx.no_daemon {
                        mcp_serve::serve(&ctx, mcp_serve::McpServeArgs { path, no_pipeline })
                    } else {
                        daemon::stdio_mcp_bridge(&ctx)
                    }
                } else {
                    http_serve::serve(
                        &ctx,
                        http_serve::HttpServeArgs {
                            host,
                            port,
                            dashboard_dir,
                            open,
                            query_only,
                            dashboard_only,
                            no_pipeline,
                            path,
                        },
                    )
                }
            }
        };

        if !long_running {
            log_command_wall_time(command_label, wall_start.elapsed(), result.is_ok());
        }

        result
    }
}

fn command_label_for(command: &Commands) -> &'static str {
    match command {
        Commands::Discover { .. } => "discover",
        Commands::Gql { .. } => "gql",
        Commands::Slice { .. } => "slice",
        Commands::BlastRadius { .. } => "blast-radius",
        Commands::Inspect { .. } => "inspect",
        Commands::Metrics { .. } => "metrics",
        Commands::Semantic { action } => match action {
            SemanticCommands::Index { .. } => "semantic index",
            SemanticCommands::Query { .. } => "semantic query",
            SemanticCommands::Distill { .. } => "semantic distill",
        },
        Commands::Communities { action } => match action {
            CommunitiesCommands::List => "communities list",
            CommunitiesCommands::Label { .. } => "communities label",
        },
        Commands::Cpg { action } => match action {
            CpgCommands::Status => "cpg status",
            CpgCommands::Function { .. } => "cpg function",
            CpgCommands::Calls { .. } => "cpg calls",
            CpgCommands::Mutations { .. } => "cpg mutations",
            CpgCommands::Flows { .. } => "cpg flows",
            CpgCommands::Ast { .. } => "cpg ast",
            CpgCommands::Export { .. } => "cpg export",
            CpgCommands::Pdg { .. } => "cpg pdg",
            CpgCommands::Slice { .. } => "cpg slice",
        },
        Commands::Check { .. } => "check",
        Commands::Export { .. } => "export",
        Commands::Install { .. } => "install",
        Commands::Serve { .. } => "serve",
        Commands::Daemon { .. } => "daemon",
    }
}

fn log_command_wall_time(command: &str, elapsed: Duration, ok: bool) {
    let mark = if ok { "✓" } else { "✗" };
    let duration = format_elapsed(elapsed);
    eprintln!("[{mark}] rgctl {command} finished in {duration}");
}

fn format_elapsed(elapsed: Duration) -> String {
    let secs = elapsed.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 10.0 {
        format!("{:.2}s", secs)
    } else {
        format!("{:.1}s", secs)
    }
}

fn init_logging(verbose: bool, discover_json: bool) {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::fmt::format::FmtSpan;

    if verbose {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("info,rgctl=debug,profile=info")),
            )
            .with_target(true)
            .with_level(true)
            .with_span_events(FmtSpan::CLOSE)
            .init();
    } else if discover_json {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error")),
            )
            .with_target(false)
            .with_level(false)
            .with_ansi(false)
            .without_time()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new("warn,rgctl=info,rgbuilder_extraction=warn,rgbuilder_analysis=warn")
            }))
            .with_target(false)
            .with_level(false)
            .with_ansi(true)
            .without_time()
            .init();
    }
}

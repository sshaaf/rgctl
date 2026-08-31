//! Daemon worker process: HTTP + local control socket.

use super::config::{
    DaemonConfig, DaemonHome, is_blocked_source, sanitize_reponame, unique_reponame,
};
use super::emit_stage;
use super::protocol::{
    ControlRequest, ControlResponse, DiscoverRequest, RepoListEntry, read_control_line,
    write_control_line,
};
use crate::cli::args::OutputFormat;
use crate::cli::context::CliContext;
use crate::cli::discover::{self, DiscoverArgs};
use crate::cli::http_serve;
use anyhow::{Context, Result, bail};
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rgctl_graph::snapshot::SNAPSHOT_FILE;
use rgctl_service::command::{CheckArgs, Command, ImpactArgs, MetricsArgs, QueryArgs};
use rgctl_service::{Session, execute};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower::ServiceExt;

pub struct WorkerState {
    pub home: DaemonHome,
    pub cfg: DaemonConfig,
    pub bind: SocketAddr,
    pub stop: AtomicBool,
    pub catalog: Mutex<HashMap<String, CachedRepo>>,
}

#[derive(Clone, Debug)]
pub struct CachedRepo {
    pub name: String,
    pub source: PathBuf,
    pub cache: PathBuf,
}

pub fn run_worker(home: DaemonHome, host: String, port: u16) -> Result<()> {
    home.ensure_dirs()?;
    let cfg = DaemonConfig::load_or_init(&home)?;
    let bind: SocketAddr = format!("{host}:{port}")
        .parse()
        .with_context(|| format!("invalid bind {host}:{port}"))?;
    let catalog = Mutex::new(scan_catalog(&home, &cfg)?);
    let state = Arc::new(WorkerState {
        home: home.clone(),
        cfg,
        bind,
        stop: AtomicBool::new(false),
        catalog,
    });
    std::fs::write(home.pid_file(), format!("{}", std::process::id()))?;

    let http_state = Arc::clone(&state);
    let http = std::thread::Builder::new()
        .name("rg-ctl-http".into())
        .spawn(move || {
            if let Err(err) = run_http(http_state) {
                eprintln!("rgctl daemon http: {err:#}");
            }
        })?;

    run_control_loop(Arc::clone(&state))?;
    state.stop.store(true, Ordering::SeqCst);
    let _ = http.join();
    let _ = std::fs::remove_file(home.pid_file());
    let _ = std::fs::remove_file(home.control_file());
    let _ = std::fs::remove_file(home.lock_file());
    Ok(())
}

fn scan_catalog(home: &DaemonHome, cfg: &DaemonConfig) -> Result<HashMap<String, CachedRepo>> {
    let mut map = HashMap::new();
    let cache = home.cache_root(cfg);
    if !cache.is_dir() {
        return Ok(map);
    }
    let rd = match std::fs::read_dir(&cache) {
        Ok(rd) => rd,
        Err(_) => return Ok(map),
    };
    for ent in rd.flatten() {
        if !ent.path().is_dir() {
            continue;
        }
        let name = ent.file_name().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(ent.path().join("SOURCE"))
            .unwrap_or_default()
            .trim()
            .to_string();
        map.insert(
            name.clone(),
            CachedRepo {
                name,
                source: PathBuf::from(source),
                cache: ent.path(),
            },
        );
    }
    Ok(map)
}

fn run_http(state: Arc<WorkerState>) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let t0 = Instant::now();
        let mcp_path: &'static str = Box::leak(state.cfg.mcp.path.clone().into_boxed_str());
        let mut app = Router::new()
            .route("/health", get(health))
            .route("/", get(catalog_handler))
            .fallback(nested_repo);
        if state.cfg.mcp.enabled {
            app = app.route(mcp_path, post(mcp_http));
        }
        let app = app.with_state(state.clone());
        let listener = tokio::net::TcpListener::bind(state.bind).await?;
        emit_stage("daemon_bind_http", t0.elapsed().as_secs_f64());
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                while !state.stop.load(Ordering::Relaxed) {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            })
            .await?;
        Ok::<(), anyhow::Error>(())
    })
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn catalog_handler(State(state): State<Arc<WorkerState>>) -> impl IntoResponse {
    let repos = state
        .catalog
        .lock()
        .map(|g| {
            g.values()
                .map(|r| {
                    json!({
                        "name": r.name,
                        "source": r.source,
                        "url": format!("/{}/", r.name)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Json(json!({ "repos": repos }))
}

async fn mcp_http(State(state): State<Arc<WorkerState>>, Json(msg): Json<Value>) -> Response {
    if !state.cfg.mcp.enabled {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "MCP disabled"}))).into_response();
    }
    match handle_mcp_rpc(&state, msg) {
        Ok(value) => Json(value).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

async fn nested_repo(State(state): State<Arc<WorkerState>>, req: Request<Body>) -> Response {
    let path = req.uri().path().to_string();
    let rest = path.trim_start_matches('/');
    let (name, inner) = match rest.split_once('/') {
        Some((n, r)) => (n.to_string(), format!("/{r}")),
        None if rest.is_empty() => {
            return catalog_handler(State(state)).await.into_response();
        }
        None => (rest.to_string(), "/".to_string()),
    };
    let cache = {
        let Ok(g) = state.catalog.lock() else {
            return (StatusCode::INTERNAL_SERVER_ERROR, "lock").into_response();
        };
        match g.get(&name) {
            Some(r) => r.cache.clone(),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "unknown repo", "repo": name})),
                )
                    .into_response();
            }
        }
    };
    let Ok(app_state) = http_serve::try_load_state(cache) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "repo graph not ready", "repo": name})),
        )
            .into_response();
    };
    let router = http_serve::router_for_state(app_state, false, false);
    let (mut parts, body) = req.into_parts();
    parts.uri = inner.parse().unwrap_or_else(|_| "/".parse().expect("slash uri"));
    match router.oneshot(Request::from_parts(parts, body)).await {
        Ok(resp) => resp.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

fn handle_mcp_rpc(state: &WorkerState, msg: Value) -> Result<Value> {
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let repo_arg = msg
        .pointer("/params/arguments/repo")
        .and_then(Value::as_str)
        .or_else(|| msg.pointer("/params/repo").and_then(Value::as_str));
    let cache = if method == "tools/call" {
        match resolve_repo_cache(state, repo_arg) {
            Ok(cache) => cache,
            Err(err) => {
                return Ok(json!({
                    "jsonrpc": "2.0",
                    "id": msg.get("id").cloned().unwrap_or(Value::Null),
                    "error": { "code": -32602, "message": err.to_string() }
                }));
            }
        }
    } else {
        resolve_repo_cache(state, repo_arg).unwrap_or_else(|_| state.home.root().to_path_buf())
    };
    let mut session = Session::new(&cache);
    rgctl_mcp::handle_rpc(&mut session, &msg).context("MCP returned no response")
}

fn resolve_repo_cache(state: &WorkerState, repo: Option<&str>) -> Result<PathBuf> {
    let g = state.catalog.lock().expect("catalog");
    match repo {
        Some(name) if !name.is_empty() => g
            .get(name)
            .map(|r| r.cache.clone())
            .ok_or_else(|| anyhow::anyhow!("unknown repo {name}")),
        _ if g.len() == 1 => Ok(g.values().next().expect("one").cache.clone()),
        _ => {
            if let Some(d) = &state.cfg.default_repo {
                if let Some(r) = g.get(d) {
                    return Ok(r.cache.clone());
                }
            }
            let names: Vec<_> = g.keys().cloned().collect();
            bail!("repo is required; cached: {}", names.join(", "));
        }
    }
}

fn run_control_loop(state: Arc<WorkerState>) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixListener;
        let sock = state.home.control_file();
        if sock.exists() {
            let _ = std::fs::remove_file(&sock);
        }
        let listener =
            UnixListener::bind(&sock).with_context(|| format!("bind {}", sock.display()))?;
        listener.set_nonblocking(true)?;
        while !state.stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    // Accepted sockets inherit the listener's nonblocking mode; large
                    // JSON control responses need blocking writes (see write_all).
                    if let Err(err) = stream.set_nonblocking(false) {
                        eprintln!("rgctl control: {err:#}");
                        continue;
                    }
                    if let Err(err) = handle_control_conn(&state, stream) {
                        eprintln!("rgctl control: {err:#}");
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => return Err(e.into()),
            }
        }
        let _ = std::fs::remove_file(sock);
    }
    #[cfg(windows)]
    {
        let pipe = super::win_pipe::pipe_name(&state.home);
        std::fs::write(state.home.control_file(), &pipe)?;
        while !state.stop.load(Ordering::Relaxed) {
            match super::win_pipe::accept(&state.home) {
                Ok(stream) => {
                    let _ = handle_control_conn(&state, stream);
                }
                Err(e) => {
                    if state.stop.load(Ordering::Relaxed) {
                        break;
                    }
                    eprintln!("rgctl control pipe: {e:#}");
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
        let _ = std::fs::remove_file(state.home.control_file());
    }
    Ok(())
}

fn handle_control_conn(
    state: &WorkerState,
    mut stream: impl Read + Write,
) -> Result<()> {
    let line = read_control_line(&mut stream)?;
    let req: ControlRequest = serde_json::from_str(line.trim())?;
    let t0 = Instant::now();
    let resp = dispatch_control(state, req);
    emit_stage("daemon_execute", t0.elapsed().as_secs_f64());
    write_control_line(&mut stream, &resp)?;
    Ok(())
}

fn dispatch_control(state: &WorkerState, req: ControlRequest) -> ControlResponse {
    match req {
        ControlRequest::Ping => ControlResponse::Pong,
        ControlRequest::Shutdown => {
            state.stop.store(true, Ordering::SeqCst);
            ControlResponse::Ok {
                message: "shutting down".into(),
            }
        }
        ControlRequest::Status => {
            let repos = state.catalog.lock().map(|g| g.len()).unwrap_or(0);
            let mcp = if state.cfg.mcp.enabled {
                format!("http://{}/mcp", state.bind)
            } else {
                "disabled".into()
            };
            ControlResponse::Status {
                pid: std::process::id(),
                http: format!("http://{}", state.bind),
                mcp,
                repos,
            }
        }
        ControlRequest::List => match list_repos(state) {
            Ok(repos) => ControlResponse::List { repos },
            Err(e) => ControlResponse::Err {
                message: e.to_string(),
            },
        },
        ControlRequest::Discover(d) => match run_discover(state, d) {
            Ok(value) => ControlResponse::Json { value },
            Err(e) => ControlResponse::Err {
                message: e.to_string(),
            },
        },
        ControlRequest::Gql {
            repo,
            query,
            explain,
            macro_name,
        } => match run_gql(state, &repo, query, explain, macro_name) {
            Ok(value) => ControlResponse::Json { value },
            Err(e) => ControlResponse::Err {
                message: e.to_string(),
            },
        },
        ControlRequest::Impact {
            repo,
            symbol,
            depth,
            class,
            file,
        } => match run_impact(state, &repo, symbol, depth, class, file) {
            Ok(value) => ControlResponse::Json { value },
            Err(e) => ControlResponse::Err {
                message: e.to_string(),
            },
        },
        ControlRequest::Metrics {
            repo,
            pagerank,
            betweenness,
            communities,
        } => match run_metrics(state, &repo, pagerank, betweenness, communities) {
            Ok(value) => ControlResponse::Json { value },
            Err(e) => ControlResponse::Err {
                message: e.to_string(),
            },
        },
        ControlRequest::Check { repo, policy_file } => match run_check(state, &repo, policy_file) {
            Ok(value) => ControlResponse::Json { value },
            Err(e) => ControlResponse::Err {
                message: e.to_string(),
            },
        },
        ControlRequest::McpCall {
            repo,
            tool,
            arguments,
        } => {
            let mut arguments = arguments;
            if let Some(map) = arguments.as_object_mut() {
                map.insert("repo".into(), json!(repo));
            }
            let msg = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": tool, "arguments": arguments }
            });
            match handle_mcp_rpc(state, msg) {
                Ok(v) => ControlResponse::Json { value: v },
                Err(e) => ControlResponse::Err {
                    message: e.to_string(),
                },
            }
        }
        ControlRequest::McpRpc { message } => match handle_mcp_rpc(state, message) {
            Ok(v) => ControlResponse::Json { value: v },
            Err(e) => ControlResponse::Err {
                message: e.to_string(),
            },
        },
    }
}

fn list_repos(state: &WorkerState) -> Result<Vec<RepoListEntry>> {
    let g = state.catalog.lock().expect("catalog");
    Ok(g.values()
        .map(|r| {
            let snap = rgctl_graph::paths::artifact_path(&r.cache, SNAPSHOT_FILE);
            RepoListEntry {
                name: r.name.clone(),
                source: r.source.display().to_string(),
                status: if snap.is_file() {
                    "graph_ready".into()
                } else {
                    "incomplete".into()
                },
            }
        })
        .collect())
}

fn run_discover(state: &WorkerState, req: DiscoverRequest) -> Result<Value> {
    let source = PathBuf::from(&req.source)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&req.source));
    let explicit = state.cfg.repo.iter().any(|r| {
        PathBuf::from(&r.path)
            .canonicalize()
            .ok()
            .as_ref()
            == Some(&source)
    });
    if is_blocked_source(&source, explicit) {
        bail!(
            "refusing to index {} without an explicit [[repo]] entry",
            source.display()
        );
    }
    let override_name = state.cfg.name_override(&source);
    let base = sanitize_reponame(&source, override_name.as_deref())?;
    let name = unique_reponame(&state.home.cache_root(&state.cfg), &source, &base);
    let cache = state.home.repo_dir(&state.cfg, &name);
    std::fs::create_dir_all(&cache)?;
    std::fs::write(cache.join("SOURCE"), source.to_string_lossy().as_bytes())?;
    let ctx = CliContext::new(Some(cache.clone()), None, OutputFormat::Json, None, false);
    discover::run(
        &ctx,
        DiscoverArgs {
            path: Some(source.to_string_lossy().into_owned()),
            languages: req.languages,
            exclude: req.exclude,
            with_security: req.with_security,
            with_cfg: req.with_cfg,
            with_taint: req.with_taint,
            with_dfg_loops: req.with_dfg_loops,
            with_ast_skeleton: req.with_ast_skeleton,
            write_json_graph: req.write_json_graph,
            with_dashboard: req.with_dashboard,
            export_migration_hints: req.export_migration_hints,
            with_harmonic: req.with_harmonic,
            with_kantra: req.with_kantra,
            kantra_rules: req.kantra_rules,
            kantra_catalog: req.kantra_catalog,
            kantra_target: req.kantra_target,
            kantra_index_only: req.kantra_index_only,
            full: req.full,
            migration_preset: if req.migration_preset.is_empty() {
                "hybrid_default".into()
            } else {
                req.migration_preset
            },
            migration_order: if req.migration_order.is_empty() {
                "scheduled".into()
            } else {
                req.migration_order
            },
            artifact_root: Some(cache.clone()),
        },
    )?;
    state.catalog.lock().expect("catalog").insert(
        name.clone(),
        CachedRepo {
            name: name.clone(),
            source: source.clone(),
            cache: cache.clone(),
        },
    );
    Ok(json!({
        "ok": true,
        "repo": name,
        "source": source,
        "cache": cache
    }))
}


fn run_impact(
    state: &WorkerState,
    repo: &str,
    symbol: String,
    depth: Option<usize>,
    class: Option<String>,
    file: Option<String>,
) -> Result<Value> {
    let cache = resolve_repo_cache(state, Some(repo))?;
    let t0 = Instant::now();
    let mut session = Session::new(&cache);
    emit_stage("daemon_session_open", t0.elapsed().as_secs_f64());
    execute(
        &mut session,
        Command::Impact(ImpactArgs {
            symbol,
            depth,
            class,
            file,
        }),
    )
    .map_err(Into::into)
}

fn run_metrics(
    state: &WorkerState,
    repo: &str,
    pagerank: bool,
    betweenness: bool,
    communities: bool,
) -> Result<Value> {
    let cache = resolve_repo_cache(state, Some(repo))?;
    let mut session = Session::new(&cache);
    execute(
        &mut session,
        Command::Metrics(MetricsArgs {
            pagerank,
            betweenness,
            communities,
        }),
    )
    .map_err(Into::into)
}

fn run_check(state: &WorkerState, repo: &str, policy_file: String) -> Result<Value> {
    let cache = resolve_repo_cache(state, Some(repo))?;
    let mut session = Session::new(&cache);
    execute(
        &mut session,
        Command::Check(CheckArgs { policy_file }),
    )
    .map_err(Into::into)
}

fn run_gql(
    state: &WorkerState,
    repo: &str,
    query: String,
    explain: bool,
    macro_name: Option<String>,
) -> Result<Value> {
    let cache = resolve_repo_cache(state, Some(repo))?;
    let t0 = Instant::now();
    let mut session = Session::new(&cache);
    emit_stage("daemon_session_open", t0.elapsed().as_secs_f64());
    execute(
        &mut session,
        Command::Query(QueryArgs {
            query: if macro_name.is_some() {
                None
            } else {
                Some(query)
            },
            macro_name,
            explain,
            limit: None,
        }),
    )
    .map_err(Into::into)
}

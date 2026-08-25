//! Control-plane daemon: workspace, lifecycle, HTTP catalog, MCP, CLI forwarding.

mod config;
mod protocol;
mod worker;
#[cfg(windows)]
mod win_pipe;

pub use config::DaemonHome;
pub use protocol::{ControlRequest, ControlResponse, DiscoverRequest};
pub use worker::run_worker;

use super::args::OutputFormat;
use super::context::CliContext;
use super::discover::{self, DiscoverArgs};
use super::gql::GqlArgs;
use anyhow::{Context, Result, bail};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn emit_stage(name: &str, secs: f64) {
    tracing::info!(target: "profile", stage = name, secs, "[profile] stage");
}

/// Resolve daemon home: `--daemon-home`, else `RGCTL_HOME`, else `$HOME` (`~/.rgctl/`).
pub fn resolve_home(explicit: Option<&Path>) -> Result<DaemonHome> {
    if let Some(p) = explicit {
        return DaemonHome::from_path(p);
    }
    if let Ok(env) = std::env::var("RGCTL_HOME") {
        if !env.is_empty() {
            return DaemonHome::from_path(Path::new(&env));
        }
    }
    DaemonHome::from_path(&config::default_home_root()?)
}

pub fn is_running(home: &DaemonHome) -> bool {
    match std::fs::read_to_string(home.pid_file()) {
        Ok(s) => s.trim().parse::<u32>().ok().is_some_and(pid_alive),
        Err(_) => false,
    }
}

fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

pub fn start(home: &DaemonHome, host: Option<&str>, port: Option<u16>) -> Result<u32> {
    home.ensure_dirs()?;
    if !is_running(home) {
        let _ = std::fs::remove_file(home.lock_file());
        let _ = std::fs::remove_file(home.pid_file());
    }
    if is_running(home) {
        return Ok(std::fs::read_to_string(home.pid_file())?
            .trim()
            .parse()
            .unwrap_or(0));
    }
    let _lock = match home.try_lock() {
        Ok(f) => f,
        Err(_) if is_running(home) => {
            return Ok(std::fs::read_to_string(home.pid_file())?
                .trim()
                .parse()
                .unwrap_or(0));
        }
        Err(e) => return Err(e),
    };
    let cfg = config::DaemonConfig::load_or_init(home)?;
    let host = host.unwrap_or(&cfg.host);
    let port = port.unwrap_or(cfg.port);
    let exe = std::env::current_exe().context("current_exe")?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.log_file())?;
    let err = log.try_clone()?;
    let t0 = Instant::now();
    Command::new(&exe)
        .args([
            "serve",
            "--daemon-worker",
            "--host",
            host,
            "--port",
            &port.to_string(),
        ])
        .env("RGCTL_HOME", home.root())
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err))
        .spawn()
        .context("spawn rgctl serve --daemon-worker")?;
    wait_ready(home, Duration::from_secs(20))?;
    emit_stage("daemon_spawn", t0.elapsed().as_secs_f64());
    Ok(std::fs::read_to_string(home.pid_file())?
        .trim()
        .parse()
        .unwrap_or(0))
}

fn wait_ready(home: &DaemonHome, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if is_running(home) && ping(home).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    bail!("daemon at {} did not become ready", home.root().display());
}

pub fn stop(home: &DaemonHome) -> Result<()> {
    if is_running(home) {
        let _ = call(home, &ControlRequest::Shutdown);
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline && is_running(home) {
            std::thread::sleep(Duration::from_millis(50));
        }
        if is_running(home) {
            if let Ok(pid) = std::fs::read_to_string(home.pid_file()) {
                if let Ok(pid) = pid.trim().parse::<u32>() {
                    #[cfg(unix)]
                    let _ = Command::new("kill").arg(pid.to_string()).status();
                    #[cfg(windows)]
                    let _ = Command::new("taskkill")
                        .args(["/PID", &pid.to_string(), "/F"])
                        .status();
                }
            }
        }
    }
    let _ = std::fs::remove_file(home.pid_file());
    let _ = std::fs::remove_file(home.control_file());
    let _ = std::fs::remove_file(home.lock_file());
    Ok(())
}

pub fn restart(home: &DaemonHome) -> Result<u32> {
    stop(home)?;
    start(home, None, None)
}

pub fn status_text(home: &DaemonHome) -> Result<String> {
    if !is_running(home) {
        return Ok("daemon: not running\n".into());
    }
    match call(home, &ControlRequest::Status) {
        Ok(ControlResponse::Status {
            pid,
            http,
            mcp,
            repos,
        }) => Ok(format!(
            "daemon: running\npid: {pid}\nhttp: {http}\nmcp: {mcp}\nrepos: {repos}\n"
        )),
        Ok(other) => Ok(format!("daemon: running\n{other:?}\n")),
        Err(err) => Ok(format!("daemon: pid file present; control failed: {err}\n")),
    }
}

pub fn list_text(home: &DaemonHome) -> Result<String> {
    if is_running(home) {
        return match call(home, &ControlRequest::List)? {
            ControlResponse::List { repos } if repos.is_empty() => {
                Ok("no cached repositories\n".into())
            }
            ControlResponse::List { repos } => {
                let mut out = String::new();
                for r in repos {
                    out.push_str(&format!("{}\t{}\t{}\n", r.name, r.source, r.status));
                }
                Ok(out)
            }
            other => bail!("unexpected list response: {other:?}"),
        };
    }
    let cfg = config::DaemonConfig::load_or_init(home).unwrap_or_default();
    let cache = home.cache_root(&cfg);
    if !cache.is_dir() {
        bail!(
            "neither a daemon nor a cache is present at {}",
            home.root().display()
        );
    }
    let mut out = String::new();
    let mut any = false;
    if let Ok(rd) = std::fs::read_dir(&cache) {
        for e in rd.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            any = true;
            let name = e.file_name().to_string_lossy().into_owned();
            let src = std::fs::read_to_string(e.path().join("SOURCE")).unwrap_or_else(|_| "-".into());
            let snap = rgctl_graph::paths::artifact_path(
                &e.path(),
                rgctl_graph::snapshot::SNAPSHOT_FILE,
            );
            out.push_str(&format!(
                "{name}\t{}\t{}\n",
                src.trim(),
                if snap.is_file() {
                    "graph_ready"
                } else {
                    "incomplete"
                }
            ));
        }
    }
    if !any {
        bail!(
            "neither a daemon nor a cache is present at {}",
            home.root().display()
        );
    }
    Ok(out)
}

pub fn ping(home: &DaemonHome) -> Result<()> {
    match call(home, &ControlRequest::Ping)? {
        ControlResponse::Pong => Ok(()),
        other => bail!("unexpected ping response: {other:?}"),
    }
}

pub fn call(home: &DaemonHome, req: &ControlRequest) -> Result<ControlResponse> {
    let t0 = Instant::now();
    let resp = call_inner(home, req)?;
    emit_stage("daemon_connect", t0.elapsed().as_secs_f64());
    Ok(resp)
}

#[cfg(unix)]
fn call_inner(home: &DaemonHome, req: &ControlRequest) -> Result<ControlResponse> {
    use std::os::unix::net::UnixStream;
    let mut stream = UnixStream::connect(home.control_file())
        .with_context(|| format!("connect {}", home.control_file().display()))?;
    stream.set_read_timeout(Some(Duration::from_secs(600)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    writeln!(stream, "{}", serde_json::to_string(req)?)?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str(line.trim()).context("parse daemon control response")
}

#[cfg(windows)]
fn call_inner(home: &DaemonHome, req: &ControlRequest) -> Result<ControlResponse> {
    let mut stream = win_pipe::connect(home)?;
    writeln!(stream, "{}", serde_json::to_string(req)?)?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str(line.trim()).context("parse daemon control response")
}

pub fn route_discover(ctx: &CliContext, args: DiscoverArgs) -> Result<bool> {
    if ctx.no_daemon {
        return Ok(false);
    }
    let home = resolve_home(ctx.daemon_home.as_deref())?;
    if ctx.fail_if_no_daemon && !is_running(&home) {
        eprintln!("rgctl: no daemon found");
        bail!("no daemon found");
    }
    if ctx.daemon_home.is_some() && !is_running(&home) {
        eprintln!("rgctl: daemon not found at {}", home.root().display());
        bail!("daemon not found at {}", home.root().display());
    }
    if !is_running(&home) {
        eprintln!(
            "rgctl: no daemon found; starting (home {})",
            home.root().display()
        );
        let pid = start(&home, None, None)?;
        let http = match call(&home, &ControlRequest::Status) {
            Ok(ControlResponse::Status { http, .. }) => http,
            _ => String::new(),
        };
        eprintln!("rgctl: started daemon pid {pid}{extra}", extra = if http.is_empty() { String::new() } else { format!(" {http}") });
    }
    let source = discover::resolve_session_root(ctx, args.path.as_deref());
    let req = ControlRequest::Discover(DiscoverRequest {
        source,
        languages: args.languages,
        exclude: args.exclude,
        with_security: args.with_security,
        with_cfg: args.with_cfg,
        with_taint: args.with_taint,
        with_dfg_loops: args.with_dfg_loops,
        with_ast_skeleton: args.with_ast_skeleton,
        write_json_graph: args.write_json_graph,
        with_dashboard: args.with_dashboard,
        export_migration_hints: args.export_migration_hints,
        with_harmonic: args.with_harmonic,
        full: args.full,
        migration_preset: args.migration_preset,
        migration_order: args.migration_order,
    });
    match call(&home, &req)? {
        ControlResponse::Json { value } => {
            if ctx.format == OutputFormat::Json {
                ctx.emit_json_value(&value)?;
            } else {
                eprintln!("{value}");
            }
            Ok(true)
        }
        ControlResponse::Ok { message } => {
            eprintln!("{message}");
            Ok(true)
        }
        ControlResponse::Err { message } => bail!("{message}"),
        other => bail!("unexpected discover response: {other:?}"),
    }
}

pub fn route_gql(ctx: &CliContext, args: &GqlArgs) -> Result<bool> {
    if ctx.no_daemon {
        return Ok(false);
    }
    let home = resolve_home(ctx.daemon_home.as_deref())?;
    if ctx.fail_if_no_daemon && !is_running(&home) {
        eprintln!("rgctl: no daemon found");
        bail!("no daemon found");
    }
    if ctx.daemon_home.is_some() && !is_running(&home) {
        eprintln!("rgctl: daemon not found at {}", home.root().display());
        bail!("daemon not found at {}", home.root().display());
    }
    if !is_running(&home) {
        return Ok(false);
    }
    let name = repo_name_for_path(&ctx.repo)?;
    match call(
        &home,
        &ControlRequest::Gql {
            repo: name,
            query: args.query.clone(),
            explain: args.explain,
            macro_name: args.macro_name.clone(),
        },
    )? {
        ControlResponse::Json { value } => {
            ctx.emit_json_value(&value)?;
            Ok(true)
        }
        ControlResponse::Err { message } => bail!("{message}"),
        other => bail!("unexpected gql response: {other:?}"),
    }
}


fn route_service_json(ctx: &CliContext, req: ControlRequest) -> Result<bool> {
    if ctx.no_daemon {
        return Ok(false);
    }
    let home = resolve_home(ctx.daemon_home.as_deref())?;
    if ctx.fail_if_no_daemon && !is_running(&home) {
        eprintln!("rgctl: no daemon found");
        bail!("no daemon found");
    }
    if ctx.daemon_home.is_some() && !is_running(&home) {
        eprintln!("rgctl: daemon not found at {}", home.root().display());
        bail!("daemon not found at {}", home.root().display());
    }
    if !is_running(&home) {
        return Ok(false);
    }
    match call(&home, &req)? {
        ControlResponse::Json { value } => {
            ctx.emit_json_value(&value)?;
            Ok(true)
        }
        ControlResponse::Err { message } => bail!("{message}"),
        other => bail!("unexpected service response: {other:?}"),
    }
}

pub fn route_impact(
    ctx: &CliContext,
    symbol: &str,
    depth: Option<usize>,
    class: Option<String>,
    file: Option<String>,
) -> Result<bool> {
    let name = repo_name_for_path(&ctx.repo)?;
    route_service_json(
        ctx,
        ControlRequest::Impact {
            repo: name,
            symbol: symbol.to_string(),
            depth,
            class,
            file,
        },
    )
}

pub fn route_metrics(
    ctx: &CliContext,
    pagerank: bool,
    betweenness: bool,
    communities: bool,
) -> Result<bool> {
    let name = repo_name_for_path(&ctx.repo)?;
    route_service_json(
        ctx,
        ControlRequest::Metrics {
            repo: name,
            pagerank,
            betweenness,
            communities,
        },
    )
}

pub fn route_check(ctx: &CliContext, policy_file: &str) -> Result<bool> {
    let name = repo_name_for_path(&ctx.repo)?;
    route_service_json(
        ctx,
        ControlRequest::Check {
            repo: name,
            policy_file: policy_file.to_string(),
        },
    )
}

pub fn repo_name_for_path(path: &Path) -> Result<String> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    config::sanitize_reponame(&canon, None)
}

pub fn stdio_mcp_bridge(ctx: &CliContext) -> Result<()> {
    let home = resolve_home(ctx.daemon_home.as_deref())?;
    if ctx.fail_if_no_daemon && !is_running(&home) {
        bail!("no daemon found");
    }
    if !is_running(&home) {
        if ctx.daemon_home.is_some() {
            bail!("daemon not found at {}", home.root().display());
        }
        eprintln!("rgctl: no daemon found; starting for MCP stdio");
        start(&home, None, None)?;
    }
    let cfg = config::DaemonConfig::load_or_init(&home)?;
    if !cfg.mcp.enabled {
        bail!("MCP is disabled in daemon config");
    }
    rgctl_mcp::serve_proxy(|msg| match call(
        &home,
        &ControlRequest::McpRpc {
            message: msg.clone(),
        },
    ) {
        Ok(ControlResponse::Json { value }) => Some(value),
        Ok(ControlResponse::Err { message }) => Some(serde_json::json!({
            "jsonrpc": "2.0",
            "id": msg.get("id"),
            "error": { "code": -32000, "message": message }
        })),
        Err(err) => Some(serde_json::json!({
            "jsonrpc": "2.0",
            "id": msg.get("id"),
            "error": { "code": -32000, "message": err.to_string() }
        })),
        _ => None,
    })
}

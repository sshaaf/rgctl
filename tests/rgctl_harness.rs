//! Shared helpers for rgctl integration tests (daemon, no-daemon, MCP stdio/HTTP).
//!
//! Tier A (fast CI): tiny_polyglot fixture + temp `RGCTL_HOME`.
//! Tier B (ignored): `example/linux` smoke — see `rgctl_no_daemon.rs`.

#![allow(dead_code)]

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

pub fn rgctl() -> PathBuf {
    if let Some(p) = option_env!("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(p);
    }
    if let Ok(out) = Command::new("sh")
        .args(["-c", "command -v rgctl"])
        .output()
    {
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && !p.is_empty() && Path::new(&p).is_file() {
            return PathBuf::from(p);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in ["target/release/rgctl", "target/debug/rgctl"] {
        let p = manifest.join(rel);
        if p.is_file() {
            return p;
        }
    }
    manifest.join("target/debug/rgctl")
}

pub fn fixture_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo")
}

pub fn linux_repo_path() -> PathBuf {
    std::env::var("RGBUILDER_LINUX_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("example/linux"))
}

pub fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for ent in fs::read_dir(src).unwrap() {
        let ent = ent.unwrap();
        let name = ent.file_name();
        if name == ".rgbuilder" || name == ".rbuilder" {
            continue;
        }
        let from = ent.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}

/// Copy tiny_polyglot into a temp dir; returns `(guard, repo_path)`.
pub fn materialize_fixture() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    copy_tree(&fixture_src(), &repo);
    let _ = fs::remove_dir_all(repo.join(".rgbuilder"));
    let _ = fs::remove_dir_all(repo.join(".rbuilder"));
    (tmp, repo)
}

pub fn assert_ok(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed status={:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Run rgctl with cwd = `repo` and `--no-daemon` (Tier A no-daemon pattern).
pub fn run_no_daemon_in_repo(repo: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(rgctl());
    cmd.current_dir(repo)
        .env("RGCTL_NO_DAEMON", "1")
        .arg("--no-daemon");
    cmd.args(args);
    cmd.output().expect("spawn rgctl")
}

pub fn run_no_daemon_json(repo: &Path, args: &[&str]) -> Output {
    let mut full = vec!["-f", "json"];
    full.extend_from_slice(args);
    run_no_daemon_in_repo(repo, &full)
}

pub fn cli_json(repo: &Path, args: &[&str]) -> Value {
    let output = run_no_daemon_json(repo, args);
    assert_ok(&output, &format!("cli {}", args.join(" ")));
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "CLI JSON parse failed ({err}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

pub fn discover_fixture(repo: &Path) {
    assert_ok(
        &run_no_daemon_in_repo(
            repo,
            &["discover", ".", "--languages", "java,rust"],
        ),
        "discover",
    );
}

pub fn assert_no_rgbuilder_under(path: &Path) {
    assert!(
        !path.join(".rgbuilder").exists(),
        "unexpected .rgbuilder under {}",
        path.display()
    );
}

pub fn assert_rgbuilder_snapshot(repo: &Path) {
    assert!(
        repo.join(".rgbuilder/graph.snapshot.bin").is_file(),
        "expected snapshot under {}",
        repo.join(".rgbuilder").display()
    );
}

pub fn remove_rgbuilder(repo: &Path) {
    let rb = repo.join(".rgbuilder");
    if rb.exists() {
        fs::remove_dir_all(&rb).expect("remove .rgbuilder");
    }
}

pub fn wait_http(url: &str, timeout: Duration) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if client
            .get(url)
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

pub fn reserve_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

pub fn toml_escape(path: &Path) -> String {
    format!("\"{}\"", path.display().to_string().replace('\\', "\\\\"))
}

/// Stops the daemon (and optional child) on drop.
pub struct DaemonGuard {
    pub home: PathBuf,
    child_kill: Option<Child>,
}

impl DaemonGuard {
    pub fn new(home: PathBuf) -> Self {
        Self {
            home,
            child_kill: None,
        }
    }

    pub fn with_child(home: PathBuf, child: Child) -> Self {
        Self {
            home,
            child_kill: Some(child),
        }
    }

    pub fn stop(&self) {
        let _ = Command::new(rgctl())
            .args(["--daemon-home", self.home.to_str().unwrap(), "daemon", "stop"])
            .output();
    }

    pub fn assert_not_running(&self) {
        let status = Command::new(rgctl())
            .args(["--daemon-home", self.home.to_str().unwrap(), "daemon", "status"])
            .output()
            .unwrap();
        let st = String::from_utf8_lossy(&status.stdout);
        assert!(
            st.contains("not running"),
            "expected daemon stopped, got: {st}"
        );
        assert!(
            !self.home.join(".rgbuilder/rgctl.pid").exists()
                || fs::read_to_string(self.home.join(".rgbuilder/rgctl.pid"))
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true),
            "stale pid file under {}",
            self.home.display()
        );
    }

    pub fn start_on_port(&self, port: u16) -> Output {
        Command::new(rgctl())
            .args([
                "--daemon-home",
                self.home.to_str().unwrap(),
                "daemon",
                "start",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
            ])
            .output()
            .unwrap()
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        self.stop();
        if let Some(mut c) = self.child_kill.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

pub fn daemon_discover_auto_start(home: &Path, repo: &Path) -> Output {
    Command::new(rgctl())
        .current_dir(repo)
        .env("RGCTL_HOME", home)
        .args(["discover", "."])
        .output()
        .unwrap()
}

pub fn daemon_discover(home: &Path, repo: &Path) -> Output {
    Command::new(rgctl())
        .current_dir(repo)
        .env("RGCTL_HOME", home)
        .args(["--daemon-home", home.to_str().unwrap(), "discover", "."])
        .output()
        .unwrap()
}

pub fn cache_entry_for_repo(home: &Path) -> PathBuf {
    let cache = home.join(".rgbuilder/cache");
    fs::read_dir(&cache)
        .unwrap()
        .flatten()
        .find(|e| e.path().is_dir())
        .map(|e| e.path())
        .unwrap_or_else(|| panic!("no cache entry under {}", cache.display()))
}

// --- MCP stdio ---

pub fn read_mcp_json(reader: &mut BufReader<impl Read>) -> Option<Value> {
    let mut header = String::new();
    let mut content_length: Option<usize> = None;
    loop {
        header.clear();
        if reader.read_line(&mut header).ok()? == 0 {
            return None;
        }
        let lower = header.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse().ok();
        }
        if header.trim().is_empty() {
            break;
        }
        if header.trim_start().starts_with('{') {
            return serde_json::from_str(header.trim()).ok();
        }
    }
    let len = content_length?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

pub struct McpProc {
    child: Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i64,
}

impl Drop for McpProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl McpProc {
    pub fn rpc(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        writeln!(self.stdin, "{msg}").expect("write rpc");
        let resp = read_mcp_json(&mut self.reader).expect("rpc response");
        assert_eq!(resp["id"], id, "{resp}");
        resp
    }

    pub fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.rpc(
            "tools/call",
            serde_json::json!({ "name": name, "arguments": arguments }),
        )
    }
}

pub fn mcp_connect_stdio(repo: &Path) -> McpProc {
    let mut child = Command::new(rgctl())
        .args([
            "serve",
            "--mode",
            "mcp",
            "--no-pipeline",
        ])
        .current_dir(repo)
        .env("RGCTL_NO_DAEMON", "1")
        .arg("--no-daemon")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut proc = McpProc {
        child,
        stdin,
        reader: BufReader::new(stdout),
        next_id: 1,
    };
    let init = proc.rpc(
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" }
        }),
    );
    assert!(init.get("result").is_some(), "initialize failed: {init}");
    proc
}

pub fn mcp_structured(resp: &Value) -> Value {
    assert!(resp.get("error").is_none(), "tool error: {resp}");
    let result = resp.get("result").expect("result");
    let structured = result
        .get("structuredContent")
        .cloned()
        .unwrap_or_else(|| {
            let text = result["content"][0]["text"].as_str().unwrap_or("");
            serde_json::from_str(text).unwrap_or_else(|err| {
                panic!("expected structured JSON ({err}): {resp}")
            })
        });
    let text = result["content"][0]["text"].as_str().expect("content text");
    let parsed: Value = serde_json::from_str(text).expect("content text JSON");
    assert_eq!(parsed, structured, "content text must match structuredContent");
    structured
}

pub fn http_mcp_post(port: u16, body: &Value) -> Value {
    reqwest::blocking::Client::new()
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(body)
        .send()
        .unwrap()
        .json()
        .unwrap()
}

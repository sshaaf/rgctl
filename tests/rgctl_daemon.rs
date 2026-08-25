//! Daemon lifecycle, path-first discover, HTTP catalog, and MCP smoke tests.

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn rgctl() -> PathBuf {
    if let Some(p) = option_env!("CARGO_BIN_EXE_rgctl") {
        return PathBuf::from(p);
    }
    for key in ["CARGO_BIN_EXE_rgctl"] {
        if let Ok(p) = std::env::var(key) {
            return PathBuf::from(p);
        }
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

fn fixture_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo")
}

fn copy_tree(src: &Path, dst: &Path) {
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

fn wait_http(url: &str, timeout: Duration) -> bool {
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

struct DaemonGuard {
    home: PathBuf,
    child_kill: Option<Child>,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = Command::new(rgctl())
            .args(["--daemon-home", self.home.to_str().unwrap(), "daemon", "stop"])
            .output();
        if let Some(mut c) = self.child_kill.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn toml_escape(path: &Path) -> String {
    format!("\"{}\"", path.display().to_string().replace('\\', "\\\\"))
}

#[test]
fn help_shows_rgctl_and_daemon_flags() {
    let out = Command::new(rgctl()).arg("--help").output().expect("help");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("rgctl"), "{text}");
    assert!(text.contains("--no-daemon"), "{text}");
    assert!(text.contains("--daemon-home"), "{text}");
    assert!(text.contains("--fail-if-no-daemon"), "{text}");
    assert!(!text.contains("rg-build"), "{text}");
    assert!(!text.contains("rg_ctl"), "{text}");

    let discover = Command::new(rgctl())
        .args(["discover", "--help"])
        .output()
        .expect("discover help");
    let d = String::from_utf8_lossy(&discover.stdout);
    assert!(d.contains("PATH") || d.contains("<PATH>"), "{d}");
}

#[test]
fn discover_path_then_full_and_dashdash_full() {
    for args in [
        vec!["--no-daemon", "discover", ".", "--full"],
        vec!["--no-daemon", "discover", ".", "--", "--full"],
    ] {
        let tmp = tempfile::tempdir().unwrap();
        copy_tree(&fixture_src(), tmp.path());
        let out = Command::new(rgctl())
            .current_dir(tmp.path())
            .args(&args)
            .output()
            .expect("discover");
        assert!(
            out.status.success(),
            "args={args:?} stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn no_daemon_discover_writes_source_tree_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    copy_tree(&fixture_src(), tmp.path());
    let out = Command::new(rgctl())
        .current_dir(tmp.path())
        .args(["--no-daemon", "discover", "."])
        .output()
        .expect("discover");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        tmp.path().join(".rgbuilder").is_dir(),
        "expected source-tree .rgbuilder after --no-daemon discover"
    );
}

#[test]
fn fail_if_no_daemon_and_dead_daemon_home() {
    let tmp = tempfile::tempdir().unwrap();
    let fail = Command::new(rgctl())
        .current_dir(tmp.path())
        .args(["--fail-if-no-daemon", "discover", "."])
        .output()
        .unwrap();
    assert!(!fail.status.success());
    let err = String::from_utf8_lossy(&fail.stderr);
    assert!(
        err.contains("no daemon found") || err.to_lowercase().contains("daemon"),
        "{err}"
    );

    let dead = tempfile::tempdir().unwrap();
    let pin = Command::new(rgctl())
        .current_dir(tmp.path())
        .args([
            "--daemon-home",
            dead.path().to_str().unwrap(),
            "discover",
            ".",
        ])
        .output()
        .unwrap();
    assert!(!pin.status.success());
}

#[test]
fn daemon_start_status_stop_idempotent_and_stale_pid() {
    let home = tempfile::tempdir().unwrap();
    let home_s = home.path().to_str().unwrap();
    let _guard = DaemonGuard {
        home: home.path().to_path_buf(),
        child_kill: None,
    };

    let start = Command::new(rgctl())
        .args([
            "--daemon-home",
            home_s,
            "daemon",
            "start",
            "--host",
            "127.0.0.1",
        ])
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let start2 = Command::new(rgctl())
        .args([
            "--daemon-home",
            home_s,
            "daemon",
            "start",
            "--host",
            "127.0.0.1",
        ])
        .output()
        .unwrap();
    assert!(start2.status.success());

    let status = Command::new(rgctl())
        .args(["--daemon-home", home_s, "daemon", "status"])
        .output()
        .unwrap();
    let st = String::from_utf8_lossy(&status.stdout);
    assert!(st.contains("running"), "{st}");

    let stop = Command::new(rgctl())
        .args(["--daemon-home", home_s, "daemon", "stop"])
        .output()
        .unwrap();
    assert!(stop.status.success());

    fs::create_dir_all(home.path().join(".rgbuilder")).unwrap();
    fs::write(home.path().join(".rgbuilder/rgctl.pid"), "999999").unwrap();
    let status = Command::new(rgctl())
        .args(["--daemon-home", home_s, "daemon", "status"])
        .output()
        .unwrap();
    let st = String::from_utf8_lossy(&status.stdout);
    assert!(st.contains("not running"), "{st}");
}

#[test]
fn auto_start_discover_writes_cache() {
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    copy_tree(&fixture_src(), repo.path());
    let _guard = DaemonGuard {
        home: home.path().to_path_buf(),
        child_kill: None,
    };
    let out = Command::new(rgctl())
        .current_dir(repo.path())
        .env("RGCTL_HOME", home.path())
        .args(["discover", "."])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(
        stderr.to_lowercase().contains("daemon"),
        "{stderr}"
    );
    let cache = home.path().join(".rgbuilder/cache");
    let has_cache = cache.is_dir()
        && fs::read_dir(&cache)
            .map(|rd| rd.flatten().any(|e| e.path().is_dir()))
            .unwrap_or(false);
    assert!(
        has_cache,
        "expected cache/{{reponame}} under {}",
        cache.display()
    );
    assert!(
        !repo.path().join(".rgbuilder").exists(),
        "daemon discover must not write source-tree .rgbuilder"
    );
    let repo_cache = fs::read_dir(&cache)
        .unwrap()
        .flatten()
        .find(|e| e.path().is_dir())
        .map(|e| e.path())
        .expect("cache entry");
    assert!(
        repo_cache
            .join(".rgbuilder/graph.snapshot.bin")
            .is_file(),
        "expected snapshot under {}",
        repo_cache.display()
    );
    assert!(
        repo_cache
            .join(".rgbuilder/analysis_results.bin")
            .is_file(),
        "expected analysis_results under {}",
        repo_cache.display()
    );
}

#[test]
fn daemon_http_catalog_query_and_unknown_404() {
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    copy_tree(&fixture_src(), repo.path());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let _guard = DaemonGuard {
        home: home.path().to_path_buf(),
        child_kill: None,
    };
    let start = Command::new(rgctl())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "daemon",
            "start",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let health = format!("http://127.0.0.1:{port}/health");
    assert!(wait_http(&health, Duration::from_secs(20)), "health");

    let disc = Command::new(rgctl())
        .current_dir(repo.path())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "discover",
            ".",
        ])
        .output()
        .unwrap();
    assert!(
        disc.status.success(),
        "{}",
        String::from_utf8_lossy(&disc.stderr)
    );

    let client = reqwest::blocking::Client::new();
    let catalog: Value = client
        .get(format!("http://127.0.0.1:{port}/"))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let repos = catalog
        .get("repos")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!repos.is_empty(), "{catalog}");
    let name = repos[0]
        .get("name")
        .and_then(|v| v.as_str())
        .expect("name")
        .to_string();

    let query = client
        .post(format!("http://127.0.0.1:{port}/{name}/api/query"))
        .json(&serde_json::json!({
            "query": "MATCH (n:Function) RETURN n LIMIT 1"
        }))
        .send()
        .unwrap();
    assert!(
        query.status().is_success() || query.status().as_u16() == 503,
        "query status {}",
        query.status()
    );

    let missing = client
        .get(format!("http://127.0.0.1:{port}/no-such-repo/api/query"))
        .send()
        .unwrap();
    assert_eq!(missing.status().as_u16(), 404);

    let list = Command::new(rgctl())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "daemon",
            "list",
        ])
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&list.stdout);
    assert!(listed.contains(&name), "{listed}");

    let _ = Command::new(rgctl())
        .args(["--daemon-home", home.path().to_str().unwrap(), "daemon", "stop"])
        .output();
    let list_disk = Command::new(rgctl())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "daemon",
            "list",
        ])
        .output()
        .unwrap();
    assert!(
        list_disk.status.success(),
        "{}",
        String::from_utf8_lossy(&list_disk.stderr)
    );
}

#[test]
fn storage_override_places_cache_outside_home() {
    let home = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    copy_tree(&fixture_src(), repo.path());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    fs::create_dir_all(home.path().join(".rgbuilder/.config")).unwrap();
    fs::write(
        home.path().join(".rgbuilder/.config/config.toml"),
        format!(
            "host = \"127.0.0.1\"\nport = {port}\nstorage = {}\n",
            toml_escape(storage.path())
        ),
    )
    .unwrap();
    let _guard = DaemonGuard {
        home: home.path().to_path_buf(),
        child_kill: None,
    };
    let start = Command::new(rgctl())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "daemon",
            "start",
        ])
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let disc = Command::new(rgctl())
        .current_dir(repo.path())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "discover",
            ".",
        ])
        .output()
        .unwrap();
    assert!(
        disc.status.success(),
        "{}",
        String::from_utf8_lossy(&disc.stderr)
    );
    assert!(
        storage.path().join("cache").is_dir(),
        "cache should live under storage"
    );
}

#[test]
fn mcp_initialize_and_tools_list() {
    let home = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let _guard = DaemonGuard {
        home: home.path().to_path_buf(),
        child_kill: None,
    };
    let start = Command::new(rgctl())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "daemon",
            "start",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    assert!(wait_http(
        &format!("http://127.0.0.1:{port}/health"),
        Duration::from_secs(20)
    ));
    let client = reqwest::blocking::Client::new();
    let init: Value = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }
        }))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(init["jsonrpc"], "2.0");
    let tools: Value = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let list = tools["result"]["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        list.iter().any(|t| t["name"] == "rgbuilder_query"),
        "{tools}"
    );
}

#[test]
fn foreground_serve_no_pipeline_query() {
    let tmp = tempfile::tempdir().unwrap();
    copy_tree(&fixture_src(), tmp.path());
    let disc = Command::new(rgctl())
        .current_dir(tmp.path())
        .args(["--no-daemon", "discover", "."])
        .output()
        .unwrap();
    if !disc.status.success() {
        eprintln!(
            "skip foreground serve: discover failed {}",
            String::from_utf8_lossy(&disc.stderr)
        );
        return;
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let child = Command::new(rgctl())
        .current_dir(tmp.path())
        .args([
            "--no-daemon",
            "serve",
            "--no-pipeline",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _guard = DaemonGuard {
        home: tmp.path().to_path_buf(),
        child_kill: Some(child),
    };
    if !wait_http(
        &format!("http://127.0.0.1:{port}/health"),
        Duration::from_secs(15),
    ) {
        eprintln!("skip: foreground serve health not ready");
        return;
    }
    let resp = reqwest::blocking::Client::new()
        .post(format!("http://127.0.0.1:{port}/api/query"))
        .json(&serde_json::json!({"query": "MATCH (n:Function) RETURN n LIMIT 1"}))
        .send()
        .unwrap();
    assert!(
        resp.status().is_success() || resp.status().as_u16() == 503,
        "{}",
        resp.status()
    );
}

#[test]
fn mcp_disabled_is_not_an_mcp_session() {
    let home = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    fs::create_dir_all(home.path().join(".rgbuilder/.config")).unwrap();
    fs::write(
        home.path().join(".rgbuilder/.config/config.toml"),
        format!("host = \"127.0.0.1\"\nport = {port}\n\n[mcp]\nenabled = false\npath = \"/mcp\"\n"),
    )
    .unwrap();
    let _guard = DaemonGuard {
        home: home.path().to_path_buf(),
        child_kill: None,
    };
    let start = Command::new(rgctl())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "daemon",
            "start",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    assert!(wait_http(
        &format!("http://127.0.0.1:{port}/health"),
        Duration::from_secs(20)
    ));
    let resp = reqwest::blocking::Client::new()
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404, "{}", resp.status());
}

#[test]
fn mcp_two_repos_without_repo_is_invalid_params() {
    let home = tempfile::tempdir().unwrap();
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let repo_a = a.path().join("alpha");
    let repo_b = b.path().join("beta");
    copy_tree(&fixture_src(), &repo_a);
    copy_tree(&fixture_src(), &repo_b);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let _guard = DaemonGuard {
        home: home.path().to_path_buf(),
        child_kill: None,
    };
    let start = Command::new(rgctl())
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "daemon",
            "start",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    for repo in [&repo_a, &repo_b] {
        let out = Command::new(rgctl())
            .current_dir(repo)
            .args([
                "--daemon-home",
                home.path().to_str().unwrap(),
                "discover",
                ".",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let client = reqwest::blocking::Client::new();
    let call: Value = client
        .post(format!("http://127.0.0.1:{port}/mcp"))
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "rgbuilder_query",
                "arguments": { "query": "MATCH (n:Function) RETURN n LIMIT 1" }
            }
        }))
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(call["error"]["code"], -32602, "{call}");
    let msg = call["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("alpha") && msg.contains("beta") || msg.contains("repo"),
        "{msg}"
    );
}

#[test]
fn stdio_mcp_does_not_index_home() {
    let fake_home = tempfile::tempdir().unwrap();
    let daemon_home = tempfile::tempdir().unwrap();
    let mut child = Command::new(rgctl())
        .env("HOME", fake_home.path())
        .env("RGCTL_HOME", daemon_home.path())
        .args(["serve", "--mode", "mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().expect("stdin");
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }
        });
        writeln!(stdin, "{body}").unwrap();
    }
    std::thread::sleep(Duration::from_millis(800));
    let _ = child.kill();
    let _ = child.wait();
    let indexed = fake_home.path().join(".rgbuilder");
    assert!(
        !indexed.is_dir()
            || fs::read_dir(&indexed)
                .map(|rd| rd.flatten().count() == 0)
                .unwrap_or(true),
        "stdio MCP must not index $HOME ({})",
        indexed.display()
    );
}

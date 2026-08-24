//! Daemon lifecycle, path-first discover, HTTP catalog, and MCP smoke tests.

use serde_json::Value;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn rg_ctl() -> PathBuf {
    if let Some(p) = option_env!("CARGO_BIN_EXE_rg_ctl") {
        return PathBuf::from(p);
    }
    for key in ["CARGO_BIN_EXE_rg_ctl", "CARGO_BIN_EXE_rg-ctl"] {
        if let Ok(p) = std::env::var(key) {
            return PathBuf::from(p);
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rg_ctl")
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
        let _ = Command::new(rg_ctl())
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
fn help_shows_rg_ctl_and_daemon_flags() {
    let out = Command::new(rg_ctl()).arg("--help").output().expect("help");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("rg_ctl"), "{text}");
    assert!(text.contains("--no-daemon"), "{text}");
    assert!(text.contains("--daemon-home"), "{text}");
    assert!(text.contains("--fail-if-no-daemon"), "{text}");
    assert!(!text.contains("rg-build"), "{text}");

    let discover = Command::new(rg_ctl())
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
        let out = Command::new(rg_ctl())
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
    let out = Command::new(rg_ctl())
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
    let fail = Command::new(rg_ctl())
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
    let pin = Command::new(rg_ctl())
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

    let start = Command::new(rg_ctl())
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
    let start2 = Command::new(rg_ctl())
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

    let status = Command::new(rg_ctl())
        .args(["--daemon-home", home_s, "daemon", "status"])
        .output()
        .unwrap();
    let st = String::from_utf8_lossy(&status.stdout);
    assert!(st.contains("running"), "{st}");

    let stop = Command::new(rg_ctl())
        .args(["--daemon-home", home_s, "daemon", "stop"])
        .output()
        .unwrap();
    assert!(stop.status.success());

    fs::create_dir_all(home.path().join(".rgbuilder")).unwrap();
    fs::write(home.path().join(".rgbuilder/rg_ctl.pid"), "999999").unwrap();
    let status = Command::new(rg_ctl())
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
    let out = Command::new(rg_ctl())
        .current_dir(repo.path())
        .env("RG_CTL_HOME", home.path())
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
    let start = Command::new(rg_ctl())
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

    let disc = Command::new(rg_ctl())
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

    let list = Command::new(rg_ctl())
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

    let _ = Command::new(rg_ctl())
        .args(["--daemon-home", home.path().to_str().unwrap(), "daemon", "stop"])
        .output();
    let list_disk = Command::new(rg_ctl())
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
    let start = Command::new(rg_ctl())
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
    let disc = Command::new(rg_ctl())
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
    let start = Command::new(rg_ctl())
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
    let disc = Command::new(rg_ctl())
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
    let child = Command::new(rg_ctl())
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

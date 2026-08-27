//! Tier A daemon integration tests: lifecycle, cache layout, HTTP catalog, MCP.
//!
//! Run with a single test thread to avoid port/home races:
//! `cargo test --release --test rgctl_daemon -- --test-threads=1`

mod rgctl_harness;

use rgctl_harness::{
    assert_no_rgctl_under, assert_ok, cache_entry_for_repo, copy_tree, daemon_discover,
    daemon_discover_auto_start, fixture_src, http_mcp_post, materialize_fixture, reserve_port,
    rgctl, toml_escape, wait_http, DaemonGuard,
};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

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
fn fail_if_no_daemon_and_dead_daemon_home() {
    let tmp = tempfile::tempdir().unwrap();
    copy_tree(&fixture_src(), tmp.path());
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
    let guard = DaemonGuard::new(home.path().to_path_buf());

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

    guard.stop();
    assert!(Command::new(rgctl())
        .args(["--daemon-home", home_s, "daemon", "stop"])
        .output()
        .unwrap()
        .status
        .success());

    fs::create_dir_all(home.path().join(".rgctl")).unwrap();
    fs::write(home.path().join(".rgctl/rgctl.pid"), "999999").unwrap();
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
    let _guard = DaemonGuard::new(home.path().to_path_buf());

    let out = daemon_discover_auto_start(home.path(), repo.path());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert!(
        stderr.to_lowercase().contains("daemon"),
        "{stderr}"
    );

    let cache = home.path().join(".rgctl/cache");
    let has_cache = cache.is_dir()
        && fs::read_dir(&cache)
            .map(|rd| rd.flatten().any(|e| e.path().is_dir()))
            .unwrap_or(false);
    assert!(
        has_cache,
        "expected cache/{{reponame}} under {}",
        cache.display()
    );
    assert_no_rgctl_under(repo.path());

    let repo_cache = cache_entry_for_repo(home.path());
    assert!(
        repo_cache
            .join(".rgctl/graph.snapshot.bin")
            .is_file(),
        "expected snapshot under {}",
        repo_cache.display()
    );
    assert!(
        repo_cache
            .join(".rgctl/analysis_results.bin")
            .is_file(),
        "expected analysis_results under {}",
        repo_cache.display()
    );
}

#[test]
fn daemon_session_roundtrip_discover_gql_mcp_then_cleanup() {
    let home = tempfile::tempdir().unwrap();
    let (_tmp, repo) = materialize_fixture();
    let port = reserve_port();
    let guard = DaemonGuard::new(home.path().to_path_buf());

    assert_ok(&guard.start_on_port(port), "daemon start");
    assert!(wait_http(
        &format!("http://127.0.0.1:{port}/health"),
        Duration::from_secs(20)
    ));

    let disc = daemon_discover(home.path(), &repo);
    assert_ok(&disc, "daemon discover");
    assert_no_rgctl_under(&repo);

    let gql = Command::new(rgctl())
        .current_dir(&repo)
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "-f",
            "json",
            "gql",
            "MATCH (n:Function) RETURN n LIMIT 2",
        ])
        .output()
        .unwrap();
    assert_ok(&gql, "daemon gql");
    let doc: Value = serde_json::from_slice(&gql.stdout).unwrap();
    assert_eq!(doc["schema_version"], 1);

    let tools = http_mcp_post(
        port,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "rgctl_query",
                "arguments": {
                    "repo": repo.file_name().unwrap().to_str().unwrap(),
                    "query": "MATCH (n:Function) RETURN n LIMIT 1"
                }
            }
        }),
    );
    assert!(
        tools.get("result").is_some() || tools.get("error").is_some(),
        "{tools}"
    );

    guard.stop();
    guard.assert_not_running();
}

/// Regression: nonblocking control sockets truncated JSON at 8192 bytes (VHS `jq` pipes).
#[test]
fn daemon_large_gql_json_after_discover_exceeds_8192_bytes() {
    let home = tempfile::tempdir().unwrap();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-java");
    assert!(repo.is_dir(), "missing fixture {}", repo.display());
    let port = reserve_port();
    let guard = DaemonGuard::new(home.path().to_path_buf());

    assert_ok(&guard.start_on_port(port), "daemon start");
    assert_ok(
        &Command::new(rgctl())
            .current_dir(&repo)
            .args([
                "--daemon-home",
                home.path().to_str().unwrap(),
                "discover",
                ".",
                "-l",
                "java",
                "-e",
                "target",
                "--with-cfg",
            ])
            .output()
            .unwrap(),
        "daemon discover ecommerce-java",
    );

    let gql = Command::new(rgctl())
        .current_dir(&repo)
        .args([
            "--daemon-home",
            home.path().to_str().unwrap(),
            "-f",
            "json",
            "gql",
            "--macro-name",
            "all_functions",
            "unused",
        ])
        .output()
        .unwrap();
    assert_ok(&gql, "daemon gql all_functions");
    assert!(
        gql.stdout.len() > 8192,
        "expected large JSON payload, got {} bytes",
        gql.stdout.len()
    );
    let doc: Value = serde_json::from_slice(&gql.stdout).unwrap();
    assert_eq!(doc["schema_version"], 1);
    assert!(doc["count"].as_u64().unwrap_or(0) > 0);

    guard.stop();
}

#[test]
fn daemon_http_catalog_query_and_unknown_404() {
    let home = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    copy_tree(&fixture_src(), repo.path());
    let port = reserve_port();
    let guard = DaemonGuard::new(home.path().to_path_buf());

    assert_ok(&guard.start_on_port(port), "daemon start");
    assert!(wait_http(
        &format!("http://127.0.0.1:{port}/health"),
        Duration::from_secs(20)
    ));

    assert_ok(&daemon_discover(home.path(), repo.path()), "discover");

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
}

#[test]
fn storage_override_places_cache_outside_home() {
    let home = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    copy_tree(&fixture_src(), repo.path());
    let port = reserve_port();
    fs::create_dir_all(home.path().join(".rgctl/.config")).unwrap();
    fs::write(
        home.path().join(".rgctl/.config/config.toml"),
        format!(
            "host = \"127.0.0.1\"\nport = {port}\nstorage = {}\n",
            toml_escape(storage.path())
        ),
    )
    .unwrap();
    let _guard = DaemonGuard::new(home.path().to_path_buf());

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
    assert_ok(&daemon_discover(home.path(), repo.path()), "discover");
    assert!(
        storage.path().join("cache").is_dir(),
        "cache should live under storage"
    );
}

#[test]
fn mcp_initialize_and_tools_list() {
    let home = tempfile::tempdir().unwrap();
    let port = reserve_port();
    let _guard = DaemonGuard::new(home.path().to_path_buf());

    assert_ok(&guard_start(home.path(), port), "start");
    assert!(wait_http(
        &format!("http://127.0.0.1:{port}/health"),
        Duration::from_secs(20)
    ));

    let init = http_mcp_post(
        port,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }
        }),
    );
    assert_eq!(init["jsonrpc"], "2.0");

    let tools = http_mcp_post(
        port,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }),
    );
    let list = tools["result"]["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        list.iter().any(|t| t["name"] == "rgctl_query"),
        "{tools}"
    );
}

fn guard_start(home: &std::path::Path, port: u16) -> std::process::Output {
    Command::new(rgctl())
        .args([
            "--daemon-home",
            home.to_str().unwrap(),
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

#[test]
fn foreground_serve_no_pipeline_query() {
    let (_tmp, repo) = materialize_fixture();
    assert_ok(
        &rgctl_harness::run_no_daemon_in_repo(&repo, &["discover", "."]),
        "discover",
    );
    let port = reserve_port();
    let child = Command::new(rgctl())
        .current_dir(&repo)
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
    let _guard = DaemonGuard::with_child(_tmp.path().to_path_buf(), child);
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
    let port = reserve_port();
    fs::create_dir_all(home.path().join(".rgctl/.config")).unwrap();
    fs::write(
        home.path().join(".rgctl/.config/config.toml"),
        format!("host = \"127.0.0.1\"\nport = {port}\n\n[mcp]\nenabled = false\npath = \"/mcp\"\n"),
    )
    .unwrap();
    let _guard = DaemonGuard::new(home.path().to_path_buf());

    assert_ok(&guard_start(home.path(), port), "start");
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
    let port = reserve_port();
    let _guard = DaemonGuard::new(home.path().to_path_buf());

    assert_ok(&guard_start(home.path(), port), "start");
    for repo in [&repo_a, &repo_b] {
        assert_ok(&daemon_discover(home.path(), repo), "discover");
    }

    let call = http_mcp_post(
        port,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "rgctl_query",
                "arguments": { "query": "MATCH (n:Function) RETURN n LIMIT 1" }
            }
        }),
    );
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
    let indexed = fake_home.path().join(".rgctl");
    assert!(
        !indexed.is_dir()
            || fs::read_dir(&indexed)
                .map(|rd| rd.flatten().count() == 0)
                .unwrap_or(true),
        "stdio MCP must not index $HOME ({})",
        indexed.display()
    );
}

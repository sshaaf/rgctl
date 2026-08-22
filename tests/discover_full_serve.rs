//! Integration tests for `discover --full` and `serve --mode` / auto-pipeline.

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn rgbuilder_bin() -> PathBuf {
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rg-build") {
        return PathBuf::from(bin);
    }
    if let Some(bin) = std::env::var_os("CARGO_BIN_EXE_rg_build") {
        return PathBuf::from(bin);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/rg-build")
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(from, to)?;
        } else {
            fs::copy(from, to)?;
        }
    }
    Ok(())
}

fn materialize() -> (tempfile::TempDir, PathBuf) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_polyglot_repo");
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    copy_dir_all(&fixture, &repo).expect("copy fixture");
    let _ = fs::remove_dir_all(repo.join(".rgbuilder"));
    let _ = fs::remove_dir_all(repo.join(".rbuilder"));
    fs::write(
        repo.join("java/com/example/Cart.java"),
        "package com.example;\npublic class Cart { private int total; }\n",
    )
    .expect("write field fixture");
    (tmp, repo)
}

fn run_in(repo: &Path, args: &[&str]) -> Output {
    Command::new(rgbuilder_bin())
        .args(args)
        .current_dir(repo)
        .output()
        .expect("spawn rg-build")
}

fn assert_ok(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed status={:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn discover_help_mentions_full() {
    let output = Command::new(rgbuilder_bin())
        .args(["discover", "--help"])
        .output()
        .expect("help");
    assert_ok(&output, "discover --help");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("--full"),
        "help should mention --full:\n{text}"
    );
}

#[test]
fn discover_full_writes_snapshot_dashboard_semantic_and_status() {
    let (_tmp, repo) = materialize();
    let repo_s = repo.to_str().unwrap();
    let output = run_in(
        &repo,
        &[
            "-r",
            repo_s,
            "discover",
            ".",
            "--full",
            "--languages",
            "java,rust",
        ],
    );
    assert_ok(&output, "discover --full");

    assert!(repo.join(".rgbuilder/graph.snapshot.bin").is_file());
    assert!(repo.join(".rgbuilder/dashboard/index.html").is_file());
    assert!(repo.join(".rgbuilder/semantic_index.bin").is_file());
    let status: Value = serde_json::from_slice(
        &fs::read(repo.join(".rgbuilder/pipeline_status.json")).expect("status file"),
    )
    .expect("status json");
    let ids: Vec<&str> = status["plan"]
        .as_array()
        .expect("plan")
        .iter()
        .filter_map(|s| s["id"].as_str())
        .collect();
    assert_eq!(ids, ["basic_discover", "deep_pass", "semantic_index"]);
}

#[test]
fn discover_full_json_is_one_object_with_plan() {
    let (_tmp, repo) = materialize();
    let repo_s = repo.to_str().unwrap();
    let output = run_in(
        &repo,
        &[
            "-r",
            repo_s,
            "-f",
            "json",
            "discover",
            ".",
            "--full",
            "--languages",
            "java,rust",
        ],
    );
    assert_ok(&output, "discover --full json");
    let doc: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "stdout was not a single JSON object ({err}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    assert_eq!(doc["schema_version"], 2);
    assert_eq!(doc["command"], "discover");
    assert_eq!(doc["full"], true);
    assert_eq!(doc["plan"].as_array().expect("plan").len(), 3);
}

#[test]
fn discover_full_does_not_require_taint() {
    let (_tmp, repo) = materialize();
    let repo_s = repo.to_str().unwrap();
    let output = run_in(
        &repo,
        &[
            "-r",
            repo_s,
            "discover",
            ".",
            "--full",
            "--languages",
            "java,rust",
        ],
    );
    assert_ok(&output, "discover --full without taint");
    let taint = repo.join(".rgbuilder/dashboard/taint_index.json");
    if taint.is_file() {
        let doc: Value = serde_json::from_slice(&fs::read(taint).unwrap()).unwrap();
        assert_ne!(doc["available"].as_bool(), Some(true));
    }
}

#[test]
fn discover_full_second_run_and_missing_dashboard() {
    let (_tmp, repo) = materialize();
    let repo_s = repo.to_str().unwrap();
    let args = [
        "-r",
        repo_s,
        "-f",
        "json",
        "discover",
        ".",
        "--full",
        "--languages",
        "java,rust",
    ];
    assert_ok(&run_in(&repo, &args), "first --full");
    let second = run_in(&repo, &args);
    assert_ok(&second, "second --full");
    let doc: Value = serde_json::from_slice(&second.stdout).expect("json");
    let status = doc["plan"].as_array().unwrap()[0]["status"]
        .as_str()
        .unwrap_or("");
    assert!(status == "skipped" || status == "complete");

    fs::remove_file(repo.join(".rgbuilder/dashboard/index.html")).ok();
    let third = run_in(&repo, &args);
    assert_ok(&third, "third --full after deleting dashboard");
    let doc: Value = serde_json::from_slice(&third.stdout).expect("json");
    assert_ne!(
        doc["plan"].as_array().unwrap()[1]["status"].as_str(),
        Some("skipped")
    );
    assert!(repo.join(".rgbuilder/dashboard/index.html").is_file());
}

#[test]
fn discover_full_overlapping_lock_fails() {
    let (_tmp, repo) = materialize();
    fs::create_dir_all(repo.join(".rgbuilder")).unwrap();
    fs::write(repo.join(".rgbuilder/pipeline.lock"), b"99999\n").unwrap();
    let repo_s = repo.to_str().unwrap();
    let output = run_in(
        &repo,
        &[
            "-r",
            repo_s,
            "discover",
            ".",
            "--full",
            "--languages",
            "java,rust",
        ],
    );
    assert!(!output.status.success());
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.to_ascii_lowercase().contains("already running") || err.contains("lock"),
        "{err}"
    );
}

fn pick_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn wait_for_health(base: &str, timeout: Duration) -> bool {
    let client = reqwest::blocking::Client::new();
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(resp) = client.get(format!("{base}/api/health")).send()
            && resp.status().is_success()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn serve_without_dashboard_binds_preparing_and_status() {
    let (_tmp, repo) = materialize();
    let port = pick_port();
    let base = format!("http://127.0.0.1:{port}");
    let mut child = Command::new(rgbuilder_bin())
        .args([
            "-r",
            repo.to_str().unwrap(),
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .current_dir(&repo)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn serve");
    let ok = wait_for_health(&base, Duration::from_secs(20));
    let client = reqwest::blocking::Client::new();
    let home = if ok {
        client.get(format!("{base}/")).send().ok()
    } else {
        None
    };
    let status = if ok {
        client.get(format!("{base}/api/status")).send().ok()
    } else {
        None
    };
    let _ = child.kill();
    let _ = child.wait();
    assert!(
        ok,
        "serve should become healthy without a prebuilt dashboard"
    );
    let body = home.expect("GET /").text().unwrap_or_default();
    assert!(
        body.to_ascii_lowercase().contains("prepar") || body.contains("rgBuilder"),
        "GET / unexpected: {body}"
    );
    let status = status.expect("GET /api/status");
    assert!(status.status().is_success());
    let doc: Value = status.json().expect("status json");
    assert_eq!(doc["schema_version"], 1);
    assert_eq!(doc["command"], "pipeline_status");
}

#[test]
fn serve_no_pipeline_without_dashboard_exits() {
    let (_tmp, repo) = materialize();
    let output = run_in(
        &repo,
        &[
            "-r",
            repo.to_str().unwrap(),
            "serve",
            "--no-pipeline",
            "--port",
            "18080",
        ],
    );
    assert!(!output.status.success());
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("dashboard") || err.contains("discover"),
        "{err}"
    );
}

#[test]
fn serve_help_mode_and_unknown_mode() {
    let help = Command::new(rgbuilder_bin())
        .args(["serve", "--help"])
        .output()
        .expect("help");
    assert_ok(&help, "serve --help");
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(text.contains("--mode"), "{text}");
    assert!(text.contains("mcp") && text.contains("standard"), "{text}");

    let bad = Command::new(rgbuilder_bin())
        .args(["serve", "--mode", "ftp"])
        .output()
        .expect("bad mode");
    assert!(!bad.status.success());
}

#[test]
fn mcp_initialize_and_status_tool() {
    let (_tmp, repo) = materialize();
    let mut child = Command::new(rgbuilder_bin())
        .args(["-r", repo.to_str().unwrap(), "serve", "--mode", "mcp"])
        .current_dir(&repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");
    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);

    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "0" }
        }
    });
    writeln!(stdin, "{init}").expect("write initialize");
    let init_resp = read_mcp_json(&mut reader).expect("initialize response");
    assert_eq!(init_resp["id"], 1);
    assert!(init_resp.get("result").is_some(), "{init_resp}");

    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": { "name": "rgbuilder_status", "arguments": {} }
    });
    writeln!(stdin, "{call}").expect("write tools/call");
    let call_resp = read_mcp_json(&mut reader).expect("tools/call response");
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(call_resp["id"], 2);
    let result = &call_resp["result"];
    if let Some(structured) = result.get("structuredContent") {
        assert_eq!(structured["schema_version"], 1);
        assert_eq!(structured["command"], "pipeline_status");
    } else {
        let text = result["content"][0]["text"].as_str().unwrap_or("");
        let doc: Value = serde_json::from_str(text).expect("status text json");
        assert_eq!(doc["schema_version"], 1);
    }
}

fn read_mcp_json(reader: &mut BufReader<impl Read>) -> Option<Value> {
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

//! Tier C: OpenCode host smoke — runs `scripts/integration/opencode-mcp-smoke.sh`.
//!
//! Fast path (no opencode): exits 0 with skip message.
//! Full smoke (manual / nightly):
//!
//! ```text
//! cargo test --release --test opencode_mcp_smoke -- --ignored --nocapture
//! RGCTL_REQUIRE_OPENCODE=1 ./scripts/integration/opencode-mcp-smoke.sh
//! ```

use std::path::PathBuf;
use std::process::Command;

fn smoke_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/integration/opencode-mcp-smoke.sh")
}

fn run_smoke(extra_env: &[(&str, &str)]) -> (i32, String) {
    let script = smoke_script();
    assert!(
        script.is_file(),
        "missing smoke script at {}",
        script.display()
    );
    let mut cmd = Command::new("bash");
    cmd.arg(&script);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    if let Ok(bin) = std::env::var("CARGO_BIN_EXE_rgctl") {
        cmd.env("RGCTL_RGCTL", bin);
    }
    let output = cmd.output().expect("run opencode smoke script");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.code().unwrap_or(-1), combined)
}

#[test]
fn opencode_smoke_script_skips_when_cli_missing() {
    let which = Command::new("sh")
        .args(["-c", "command -v opencode >/dev/null 2>&1"])
        .status()
        .expect("which opencode");
    if which.success() {
        eprintln!("skip: opencode is installed; use ignored test for full smoke");
        return;
    }
    let (code, out) = run_smoke(&[]);
    assert_eq!(code, 0, "expected skip exit 0:\n{out}");
    assert!(
        out.contains("skip: opencode not on PATH"),
        "expected skip message:\n{out}"
    );
}

#[test]
#[ignore = "manual: requires opencode CLI; runs full MCP connect smoke"]
fn opencode_mcp_list_smoke_stdio() {
    let (code, out) = run_smoke(&[
        ("RGCTL_REQUIRE_OPENCODE", "1"),
        ("RGCTL_OPENCODE_MODE", "stdio"),
    ]);
    assert_eq!(code, 0, "stdio smoke failed:\n{out}");
    assert!(
        out.contains("OK") && out.contains("rgctl"),
        "expected success marker:\n{out}"
    );
}

#[test]
#[ignore = "manual: requires opencode CLI; daemon HTTP MCP remote mode"]
fn opencode_mcp_list_smoke_daemon() {
    let (code, out) = run_smoke(&[
        ("RGCTL_REQUIRE_OPENCODE", "1"),
        ("RGCTL_OPENCODE_MODE", "daemon"),
    ]);
    assert_eq!(code, 0, "daemon smoke failed:\n{out}");
    assert!(
        out.contains("OK") && out.contains("rgctl"),
        "expected success marker:\n{out}"
    );
}

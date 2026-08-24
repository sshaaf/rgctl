//! User-guide scenario suite — docs/user-guide/scenarios via scripts/user-guide-scenarios.py

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn user_guide_scenarios_pass() {
    let root = repo_root();
    let script = root.join("scripts/user-guide-scenarios.py");
    assert!(script.is_file(), "missing {}", script.display());

    let mut cmd = Command::new("python3");
    cmd.arg(&script).current_dir(&root);
    if let Ok(bin) = std::env::var("CARGO_BIN_EXE_rg_ctl") {
        cmd.env("CARGO_BIN_EXE_rg_ctl", bin);
    }
    let out = cmd.output().expect("spawn python3");
    if !out.status.success() {
        panic!(
            "user-guide scenarios failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

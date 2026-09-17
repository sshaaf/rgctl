//! Ruby CFG discover integration on ecommerce-ruby.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn discover_with_cfg_indexes_ruby() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-ruby");
    if !repo.is_dir() {
        return;
    }
    let bin = std::env::var("CARGO_BIN_EXE_rgctl")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/rgctl")
        });
    let _ = std::fs::remove_dir_all(repo.join(".rgctl"));
    let out = Command::new(&bin)
        .args(["discover", ".", "-l", "ruby", "--with-cfg"])
        .current_dir(&repo)
        .output()
        .expect("discover");
    assert!(
        out.status.success(),
        "discover --with-cfg failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let cfg_index = repo.join(".rgctl/dashboard/cfg_index.json");
    if cfg_index.is_file() {
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&cfg_index).unwrap()).unwrap();
        assert_eq!(v["available"], true);
    }
}

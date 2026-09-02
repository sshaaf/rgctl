//! PHP CFG analysis against the ecommerce-php fixture.

use rgctl::analysis::{
    ProgramDependenceGraph, build_cfg_for_function, cfg_language_id_from_path,
};
use std::path::Path;

fn php_repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("rgctl-tests/ecommerce-php")
}

#[test]
fn php_cfg_language_profile_maps_extension() {
    let path = Path::new("src/Service/AuthService.php");
    assert_eq!(cfg_language_id_from_path(path), Some("php"));
}

#[test]
fn php_cfg_builds_auth_login_from_fixture() {
    let repo = php_repo();
    let file = repo.join("src/Service/AuthService.php");
    assert!(file.is_file(), "fixture missing at {}", file.display());

    let source = std::fs::read_to_string(&file).unwrap();
    let cfg = build_cfg_for_function("php", &source, "login").expect("login CFG");
    assert!(cfg.blocks.len() >= 3, "expected branching CFG for login");

    let pdg = ProgramDependenceGraph::build(&cfg, source.as_bytes()).expect("login PDG");
    assert!(!pdg.nodes.is_empty());
}

#[test]
fn php_cfg_builds_process_order_field_write() {
    let repo = php_repo();
    let file = repo.join("src/Service/AuthService.php");
    let source = std::fs::read_to_string(&file).unwrap();
    let cfg = build_cfg_for_function("php", &source, "processOrder").expect("processOrder CFG");
    assert!(!cfg.blocks.is_empty());
}

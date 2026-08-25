//! C# CFG analysis against the ecommerce-csharp fixture.

use rgctl::analysis::{
    ProgramDependenceGraph, build_cfg_for_function, cfg_language_id_from_path,
};
use std::path::{Path, PathBuf};

fn csharp_repo() -> PathBuf {
    std::env::var("RGCTL_CSHARP_REPO")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rgctl-tests/ecommerce-csharp")
        })
}

#[test]
fn csharp_cfg_language_profile_maps_extension() {
    let path = Path::new("src/Ecommerce/Services/OrderService.cs");
    assert_eq!(cfg_language_id_from_path(path), Some("csharp"));
}

#[test]
fn csharp_cfg_builds_checkout_from_fixture() {
    let repo = csharp_repo();
    let file = repo.join("src/Ecommerce/Services/OrderService.cs");
    if !file.is_file() {
        eprintln!("skip: csharp fixture not found at {}", file.display());
        return;
    }

    let source = std::fs::read_to_string(&file).unwrap();
    let cfg =
        build_cfg_for_function("csharp", &source, "CheckoutAsync").expect("CheckoutAsync CFG");
    assert!(
        cfg.blocks.len() >= 3,
        "expected branching CFG for CheckoutAsync"
    );

    let pdg = ProgramDependenceGraph::build(&cfg, source.as_bytes()).expect("CheckoutAsync PDG");
    assert!(!pdg.nodes.is_empty());
}

#[test]
fn csharp_switch_cfg_has_multiple_branches() {
    let code = r#"
public class Demo {
    public string Classify(int v) {
        switch (v) {
            case 1:
                return "one";
            case 2:
                return "two";
            default:
                return "other";
        }
    }
}
"#;
    let cfg = build_cfg_for_function("csharp", code, "Classify").expect("Classify CFG");
    assert!(cfg.blocks.len() >= 5, "switch should fan out blocks");
}

#[test]
fn csharp_loop_has_cycle() {
    let code = r#"
public class Demo {
    public int Sum(int n) {
        var total = 0;
        for (int i = 0; i < n; i++) {
            total += i;
        }
        return total;
    }
}
"#;
    let cfg = build_cfg_for_function("csharp", code, "Sum").expect("Sum CFG");
    assert!(cfg.has_cycle());
}

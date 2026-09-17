use rgctl_agent_pack_codegen::generate;
use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let repo_root = PathBuf::from(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf();
    let pack_root = repo_root.join("agent-pack");
    let out_dir = pack_root.join("out");
    let version = env::var("RGCTL_VERSION").unwrap_or_else(|_| "0.0.0-dev".to_string());
    generate(&pack_root, &out_dir, &version).unwrap_or_else(|e| panic!("agent-pack codegen: {e}"));
    eprintln!("generated {}", out_dir.display());
}

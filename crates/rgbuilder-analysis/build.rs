//! Emit `OUT_DIR/vocab_matrix.bin` from `assets/vocab_tokens.txt`.
//!
//! Layout (little-endian):
//! - magic: b"RBVK"
//! - u32 version (=1)
//! - u32 n_tokens
//! - u32 dimensions (=256)
//! - for each token: u16 len + utf8 bytes (len > 0)
//! - i8 weights: n_tokens * dimensions (row-major)

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

const MAGIC: &[u8; 4] = b"RBVK";
const VERSION: u32 = 1;
const DIMENSIONS: u32 = 256;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let tokens_path = manifest_dir.join("assets/vocab_tokens.txt");
    println!("cargo:rerun-if-changed={}", tokens_path.display());
    let distilled_path = manifest_dir.join("assets/vocab_matrix.bin");
    println!("cargo:rerun-if-changed={}", distilled_path.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = out_dir.join("vocab_matrix.bin");

    if distilled_path.is_file() {
        let bytes = fs::read(&distilled_path).expect("read distilled vocab_matrix.bin");
        assert!(
            bytes.len() >= 16 && bytes.starts_with(MAGIC),
            "assets/vocab_matrix.bin is not an RBVK blob; regenerate with `rgctl semantic distill`"
        );
        fs::write(&out_path, bytes).expect("write distilled vocab_matrix.bin");
        println!("cargo:rustc-env=RGBUILDER_VOCAB_MODEL_ID=vocab-accumulate-v2");
        println!(
            "cargo:rustc-env=RGBUILDER_VOCAB_MATRIX={}",
            out_path.display()
        );
        return;
    }

    let text = fs::read_to_string(&tokens_path).unwrap_or_else(|err| {
        panic!(
            "missing vocab_tokens.txt at {}: {err}",
            tokens_path.display()
        )
    });

    let mut tokens: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_ascii_lowercase)
        .filter(|t| (3..=48).contains(&t.len()))
        .collect();
    tokens.sort_unstable();
    tokens.dedup();
    if tokens.len() > 40_000 {
        tokens.truncate(40_000);
    }
    assert!(
        !tokens.is_empty(),
        "vocab_tokens.txt produced empty vocabulary"
    );

    let mut out = fs::File::create(&out_path).expect("create vocab_matrix.bin");

    out.write_all(MAGIC).unwrap();
    out.write_all(&VERSION.to_le_bytes()).unwrap();
    out.write_all(&(tokens.len() as u32).to_le_bytes()).unwrap();
    out.write_all(&DIMENSIONS.to_le_bytes()).unwrap();

    for token in &tokens {
        let bytes = token.as_bytes();
        let len = u16::try_from(bytes.len()).expect("token len fits u16");
        out.write_all(&len.to_le_bytes()).unwrap();
        out.write_all(bytes).unwrap();
    }

    let dims = DIMENSIONS as usize;
    for token in &tokens {
        let base = fnv1a(token.as_bytes());
        for d in 0..dims {
            let mixed = base
                .wrapping_add((d as u64).wrapping_mul(0x9E3779B97F4A7C15))
                .wrapping_mul(0x100000001b3);
            let centered = (mixed % 251) as i32 - 125; // -125..=125
            out.write_all(&[centered as i8 as u8]).unwrap();
        }
    }

    println!("cargo:rustc-env=RGBUILDER_VOCAB_MODEL_ID=vocab-accumulate-v1");
    println!(
        "cargo:rustc-env=RGBUILDER_VOCAB_MATRIX={}",
        out_path.display()
    );
}

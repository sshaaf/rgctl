//! Bundled [`code-daemon-embed-v1`](https://huggingface.co/faxenoff/code-daemon-embed-v1) weights.
//!
//! Model and tokenizer bytes are compiled into the rgctl binary so semantic indexing
//! works without a separate download step.

use rgctl_error::{Error, Result};
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::sync::OnceLock;

/// ONNX graph definition (external weights in [`EMBEDDED_MODEL_DATA`]).
pub const EMBEDDED_MODEL_ONNX: &[u8] = include_bytes!("../assets/code-daemon-embed-v1/model.onnx");

/// FP32 weight blob referenced by the ONNX graph.
pub const EMBEDDED_MODEL_DATA: &[u8] =
    include_bytes!("../assets/code-daemon-embed-v1/model.onnx.data");

/// SentencePiece tokenizer bundled with code-daemon.
pub const EMBEDDED_TOKENIZER: &[u8] =
    include_bytes!("../assets/code-daemon-embed-v1/sentencepiece.bpe.model");

/// Filename referenced inside the ONNX graph for external initializers.
pub const EMBEDDED_MODEL_DATA_NAME: &str = "model.onnx.data";

static TOKENIZER_PATH: OnceLock<StdResult<PathBuf, String>> = OnceLock::new();

/// Materialize the bundled SentencePiece model once and return its path.
///
/// SentencePiece loads from disk; we write the embedded bytes to a stable temp path
/// on first use and reuse that file for the process lifetime.
pub fn embedded_tokenizer_path() -> Result<&'static Path> {
    let stored = TOKENIZER_PATH.get_or_init(materialize_tokenizer);
    stored
        .as_ref()
        .map(|path| path.as_path())
        .map_err(|err| Error::ConfigError(err.clone()))
}

fn materialize_tokenizer() -> StdResult<PathBuf, String> {
    let dir = std::env::temp_dir().join("rgctl-code-daemon-embed-v1");
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("create tokenizer cache {}: {err}", dir.display()))?;
    let path = dir.join("sentencepiece.bpe.model");
    if !path.is_file() {
        std::fs::write(&path, EMBEDDED_TOKENIZER)
            .map_err(|err| format!("write bundled tokenizer {}: {err}", path.display()))?;
    }
    Ok(path)
}

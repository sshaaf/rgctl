//! [`faxenoff/code-daemon-embed-v1`](https://huggingface.co/faxenoff/code-daemon-embed-v1) embedder.

#[cfg(feature = "semantic-onnx")]
use crate::semantic_onnx::{Postprocess, SharedOnnxEmbedder};
#[cfg(feature = "semantic-onnx")]
use crate::semantic_onnx_tokenizer::{OnnxTokenizer, resolve_sentencepiece_path};
use rgctl_error::{Error, Result};
use std::path::{Path, PathBuf};

/// Stable model id stored in semantic indexes built with code-daemon.
pub const CODE_DAEMON_MODEL_ID: &str = "code-daemon:v1";

/// Native embedding width before MRL truncation.
pub const CODE_DAEMON_NATIVE_DIMS: usize = 768;

/// Default max sequence length for code-daemon.
pub const CODE_DAEMON_MAX_SEQ_LEN: usize = 128;

/// Recommended MRL truncation sizes (must be multiples of 8 for binary quant).
pub const CODE_DAEMON_MRL_DIMS: [usize; 3] = [256, 512, 768];

/// Default ONNX filename in the model bundle directory (FP32; INT8 requires newer ORT ops).
pub const CODE_DAEMON_ONNX_FILE: &str = "model.onnx";

/// Default SentencePiece filename in the model bundle directory.
pub const CODE_DAEMON_TOKENIZER_FILE: &str = "sentencepiece.bpe.model";

/// Default model directory under a repository root.
pub fn default_model_dir(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".rgctl")
        .join("models")
        .join("code-daemon-embed-v1")
}

/// Default ONNX path under a repository root.
pub fn default_model_path(repo_root: &Path) -> PathBuf {
    default_model_dir(repo_root).join(CODE_DAEMON_ONNX_FILE)
}

/// Default SentencePiece path under a repository root.
pub fn default_tokenizer_path(repo_root: &Path) -> PathBuf {
    default_model_dir(repo_root).join(CODE_DAEMON_TOKENIZER_FILE)
}

/// Validate MRL dimensions for code-daemon indexes.
pub fn validate_mrl_dimensions(dimensions: usize) -> Result<()> {
    if dimensions > CODE_DAEMON_NATIVE_DIMS {
        return Err(Error::ConfigError(format!(
            "code-daemon supports at most {CODE_DAEMON_NATIVE_DIMS} dimensions (MRL); got {dimensions}"
        )));
    }
    if dimensions % 8 != 0 {
        return Err(Error::ConfigError(
            "code-daemon dimensions must be a multiple of 8 for binary quantization".into(),
        ));
    }
    Ok(())
}

/// Load code-daemon ONNX embedder with SentencePiece + MRL + L2 normalization.
#[cfg(feature = "semantic-onnx")]
pub fn load_code_daemon_embedder(
    model_path: &Path,
    tokenizer_path: Option<&Path>,
    dimensions: usize,
) -> Result<SharedOnnxEmbedder> {
    validate_mrl_dimensions(dimensions)?;
    let sp_path = resolve_sentencepiece_path(model_path, tokenizer_path)?;
    load_code_daemon_with_tokenizer(&sp_path, dimensions, |tokenizer| {
        SharedOnnxEmbedder::load_with(
            model_path,
            CODE_DAEMON_MODEL_ID,
            dimensions,
            CODE_DAEMON_NATIVE_DIMS,
            tokenizer,
            Postprocess::CodeDaemonMrl,
        )
    })
}

/// Load the bundled code-daemon embedder compiled into the rgctl binary.
#[cfg(feature = "semantic-onnx")]
pub fn load_embedded_code_daemon_embedder(dimensions: usize) -> Result<SharedOnnxEmbedder> {
    use crate::semantic_embedded::{
        EMBEDDED_MODEL_DATA, EMBEDDED_MODEL_DATA_NAME, EMBEDDED_MODEL_ONNX,
    };

    validate_mrl_dimensions(dimensions)?;
    let sp_path = crate::semantic_embedded::embedded_tokenizer_path()?;
    load_code_daemon_with_tokenizer(sp_path, dimensions, |tokenizer| {
        SharedOnnxEmbedder::load_from_embedded(
            EMBEDDED_MODEL_ONNX,
            EMBEDDED_MODEL_DATA_NAME,
            EMBEDDED_MODEL_DATA,
            CODE_DAEMON_MODEL_ID,
            dimensions,
            CODE_DAEMON_NATIVE_DIMS,
            tokenizer,
            Postprocess::CodeDaemonMrl,
        )
    })
}

#[cfg(feature = "semantic-onnx")]
fn load_code_daemon_with_tokenizer<F>(
    sp_path: &Path,
    dimensions: usize,
    load: F,
) -> Result<SharedOnnxEmbedder>
where
    F: FnOnce(OnnxTokenizer) -> Result<SharedOnnxEmbedder>,
{
    validate_mrl_dimensions(dimensions)?;
    let tokenizer = OnnxTokenizer::SentencePiece {
        path: sp_path.to_path_buf(),
        max_seq_len: CODE_DAEMON_MAX_SEQ_LEN,
        bos_id: 2,
        eos_id: 3,
        pad_id: 0,
    };
    load(tokenizer)
}

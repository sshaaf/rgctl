//! Pluggable semantic embedders (compiled vocab default; code-daemon / sign-hash fallbacks).

use rgctl_error::{Error, Result};
use std::path::{Path, PathBuf};

use crate::semantic_search::{
    SIGN_HASH_MODEL_ID, embedder_model_id, quantize_binary, sign_hash_embed,
};
use crate::semantic_vocab::{VOCAB_ACCUMULATE_MODEL_ID, VocabAccumulateEmbedder};

/// Embed text into a float vector before binary quantization.
pub trait SemanticEmbedder: Send + Sync {
    /// Stable model identifier stored in [`crate::semantic_search::SemanticIndex`].
    fn model_id(&self) -> &str;
    /// Output float dimensions (must match index configuration).
    fn dimensions(&self) -> usize;
    /// Embed one document/query string.
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    /// Preferred `embed_batch` width for index/distill chunking.
    ///
    /// CPU embedders can take large chunks. ONNX should stay small enough that
    /// a padded `[N, seq]` tensor fits comfortably in cache.
    fn preferred_batch_size(&self) -> usize {
        1024
    }
    /// Embed many texts. Default runs [`Self::embed`] in parallel (order-preserving).
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        use rayon::prelude::*;
        // Collect `Vec<Result<_>>` first. `collect::<Result<Vec<_>>>()` on a
        // parallel iterator is not indexed and can reorder rows.
        texts
            .par_iter()
            .map(|text| self.embed(text))
            .collect::<Vec<_>>()
            .into_iter()
            .collect()
    }
    /// Embed and sign-quantize to bit-packed bytes.
    fn embed_binary(&self, text: &str) -> Result<Vec<u8>> {
        Ok(quantize_binary(&self.embed(text)?))
    }
    /// Sign-quantize a batch of embeddings.
    fn embed_binary_batch(&self, texts: &[&str]) -> Result<Vec<Vec<u8>>> {
        Ok(self
            .embed_batch(texts)?
            .into_iter()
            .map(|floats| quantize_binary(&floats))
            .collect())
    }
}

/// Deterministic hash embedder (always available).
#[derive(Debug, Clone)]
pub struct SignHashEmbedder {
    dimensions: usize,
}

impl SignHashEmbedder {
    /// Create a sign-hash embedder with the given output width.
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl SemanticEmbedder for SignHashEmbedder {
    fn model_id(&self) -> &str {
        SIGN_HASH_MODEL_ID
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        Ok(sign_hash_embed(text, self.dimensions))
    }
}

/// How the caller wants to embed during `semantic index`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbedderChoice {
    /// Built-in sign-hash (`sign-hash-v1`).
    SignHash,
    /// Compiled vocab bag-of-tokens (`vocab-accumulate-v1` / distilled `v2`).
    Vocab,
    /// Generic ONNX `--model` (hash tokenization, or SentencePiece with `--tokenizer`).
    Onnx {
        /// Path to the ONNX model file.
        model: PathBuf,
        /// Optional SentencePiece / tokenizer path.
        tokenizer: Option<PathBuf>,
    },
    /// [`faxenoff/code-daemon-embed-v1`](https://huggingface.co/faxenoff/code-daemon-embed-v1).
    /// When `model` is `None`, loads the embedder bundled in the rgctl binary.
    CodeDaemon {
        /// Optional on-disk model path; `None` uses the embedded model.
        model: Option<PathBuf>,
        /// Optional SentencePiece tokenizer path.
        tokenizer: Option<PathBuf>,
    },
}

/// Options when reloading an ONNX-backed index at query time.
#[derive(Debug, Clone, Default)]
pub struct OnnxReloadOptions {
    /// Override ONNX model path from the persisted index.
    pub model_path: Option<PathBuf>,
    /// Override tokenizer path from the persisted index.
    pub tokenizer_path: Option<PathBuf>,
}

/// Resolve an embedder for indexing or querying.
pub fn resolve_embedder(
    choice: &EmbedderChoice,
    dimensions: usize,
) -> Result<Box<dyn SemanticEmbedder>> {
    match choice {
        EmbedderChoice::SignHash => Ok(Box::new(SignHashEmbedder::new(dimensions))),
        EmbedderChoice::Vocab => Ok(Box::new(VocabAccumulateEmbedder::new(dimensions)?)),
        EmbedderChoice::Onnx { model, tokenizer } => {
            onnx_embedder(model, dimensions, tokenizer.as_deref())
        }
        EmbedderChoice::CodeDaemon { model, tokenizer } => {
            code_daemon_embedder(model.as_deref(), dimensions, tokenizer.as_deref())
        }
    }
}

/// Resolve embedder from a persisted index (and optional ONNX path overrides).
pub fn embedder_for_index(
    index: &crate::semantic_search::SemanticIndex,
    reload: &OnnxReloadOptions,
) -> Result<Box<dyn SemanticEmbedder>> {
    let model_id = embedder_model_id(&index.model_id);
    if model_id == SIGN_HASH_MODEL_ID {
        return Ok(Box::new(SignHashEmbedder::new(index.dimensions)));
    }
    if model_id == VOCAB_ACCUMULATE_MODEL_ID {
        return Ok(Box::new(VocabAccumulateEmbedder::new(index.dimensions)?));
    }
    if model_id == crate::semantic_code_daemon::CODE_DAEMON_MODEL_ID {
        let model = reload
            .model_path
            .as_deref()
            .or_else(|| index.model_path.as_deref().map(Path::new))
            .filter(|path| !path.as_os_str().is_empty() && path.is_file());
        let tokenizer = reload
            .tokenizer_path
            .as_deref()
            .or_else(|| index.tokenizer_path.as_deref().map(Path::new));
        return code_daemon_embedder(model, index.dimensions, tokenizer);
    }
    if model_id.starts_with("onnx:") {
        let model = reload
            .model_path
            .clone()
            .or_else(|| index.model_path.clone().map(PathBuf::from))
            .ok_or_else(|| {
                Error::ConfigError(
                    "ONNX index requires --model path (or rebuild index with model_path stored)"
                        .into(),
                )
            })?;
        let tokenizer = reload
            .tokenizer_path
            .clone()
            .or_else(|| index.tokenizer_path.clone().map(PathBuf::from));
        return onnx_embedder(&model, index.dimensions, tokenizer.as_deref());
    }
    Err(Error::ConfigError(format!(
        "unknown semantic model_id: {}",
        model_id
    )))
}

fn onnx_embedder(
    path: &Path,
    dimensions: usize,
    tokenizer: Option<&Path>,
) -> Result<Box<dyn SemanticEmbedder>> {
    #[cfg(feature = "semantic-onnx")]
    {
        Ok(Box::new(
            super::semantic_onnx::SharedOnnxEmbedder::load_with_optional_tokenizer(
                path, dimensions, tokenizer,
            )?,
        ))
    }
    #[cfg(not(feature = "semantic-onnx"))]
    {
        let _ = (path, dimensions, tokenizer);
        Err(Error::ConfigError(
            "ONNX embedder requires building with `--features semantic-onnx`".into(),
        ))
    }
}

fn code_daemon_embedder(
    path: Option<&Path>,
    dimensions: usize,
    tokenizer: Option<&Path>,
) -> Result<Box<dyn SemanticEmbedder>> {
    #[cfg(feature = "semantic-onnx")]
    {
        Ok(Box::new(match path {
            Some(model_path) => super::semantic_code_daemon::load_code_daemon_embedder(
                model_path, tokenizer, dimensions,
            )?,
            None => super::semantic_code_daemon::load_embedded_code_daemon_embedder(dimensions)?,
        }))
    }
    #[cfg(not(feature = "semantic-onnx"))]
    {
        let _ = (path, dimensions, tokenizer);
        Err(Error::ConfigError(
            "code-daemon embedder requires building with `--features semantic-onnx`".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_hash_embedder_round_trip() {
        let embedder = SignHashEmbedder::new(64);
        let floats = embedder.embed("authenticate token").unwrap();
        assert_eq!(floats.len(), 64);
        let bits = embedder.embed_binary("authenticate token").unwrap();
        assert_eq!(bits.len(), 8);
    }

    #[test]
    fn embedder_for_index_accepts_bodies_suffix() {
        let index = crate::semantic_search::SemanticIndex {
            schema_version: crate::semantic_search::SEMANTIC_INDEX_SCHEMA_VERSION,
            model_id: crate::semantic_search::persist_semantic_model_id(SIGN_HASH_MODEL_ID, true),
            dimensions: 64,
            graph_digest: None,
            model_path: None,
            tokenizer_path: None,
            entries: vec![],
            binary_embeddings: vec![],
        };
        let embedder = embedder_for_index(
            &index,
            &OnnxReloadOptions {
                model_path: None,
                tokenizer_path: None,
            },
        )
        .unwrap();
        assert_eq!(embedder.model_id(), SIGN_HASH_MODEL_ID);
    }
}

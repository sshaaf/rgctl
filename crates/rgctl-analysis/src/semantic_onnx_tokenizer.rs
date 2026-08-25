//! Tokenizers for ONNX semantic embedders.

use rayon::prelude::*;
use rgctl_error::{Error, Result};
use std::path::{Path, PathBuf};

/// How ONNX inputs are tokenized before inference.
#[derive(Debug, Clone)]
pub enum OnnxTokenizer {
    /// Hash-based token IDs (generic fallback for unknown ONNX models).
    Hash {
        /// Maximum sequence length for padding/truncation.
        max_seq_len: usize,
        /// Vocabulary size used when hashing tokens.
        vocab_size: usize,
    },
    /// SentencePiece model (e.g. code-daemon-embed-v1).
    SentencePiece {
        /// Path to the SentencePiece model file.
        path: PathBuf,
        /// Maximum sequence length for padding/truncation.
        max_seq_len: usize,
        /// Beginning-of-sequence token id.
        bos_id: i64,
        /// End-of-sequence token id.
        eos_id: i64,
        /// Padding token id.
        pad_id: i64,
    },
}

impl OnnxTokenizer {
    /// Padding token id used when packing a batch to a common sequence length.
    pub fn pad_id(&self) -> i64 {
        match self {
            Self::Hash { .. } => 0,
            Self::SentencePiece { pad_id, .. } => *pad_id,
        }
    }

    /// Tokenize one string into `(input_ids, attention_mask)` for batch size 1.
    pub fn encode(&self, text: &str) -> Result<(Vec<i64>, Vec<i64>)> {
        match self {
            Self::Hash {
                max_seq_len,
                vocab_size,
            } => Ok(hash_tokenize(text, *max_seq_len, *vocab_size)),
            Self::SentencePiece {
                path,
                max_seq_len,
                bos_id,
                eos_id,
                pad_id,
            } => sentencepiece_encode(text, path, *max_seq_len, *bos_id, *eos_id, *pad_id),
        }
    }

    /// Tokenize many strings. Hash tokenization is parallel; SentencePiece opens the model once.
    pub fn encode_batch(&self, texts: &[&str]) -> Result<Vec<(Vec<i64>, Vec<i64>)>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        match self {
            Self::Hash {
                max_seq_len,
                vocab_size,
            } => Ok(texts
                .par_iter()
                .map(|text| hash_tokenize(text, *max_seq_len, *vocab_size))
                .collect()),
            Self::SentencePiece {
                path,
                max_seq_len,
                bos_id,
                eos_id,
                pad_id,
            } => sentencepiece_encode_batch(texts, path, *max_seq_len, *bos_id, *eos_id, *pad_id),
        }
    }
}

/// Pack variable-length `(ids, mask)` rows into row-major `[batch, seq]` tensors.
pub fn pad_encoded_batch(
    encoded: &[(Vec<i64>, Vec<i64>)],
    pad_id: i64,
) -> (usize, usize, Vec<i64>, Vec<i64>) {
    let batch = encoded.len();
    let seq = encoded
        .iter()
        .map(|(ids, _)| ids.len())
        .max()
        .unwrap_or(1)
        .max(1);
    let mut ids = vec![pad_id; batch.saturating_mul(seq)];
    let mut mask = vec![0i64; batch.saturating_mul(seq)];
    for (row, (row_ids, row_mask)) in encoded.iter().enumerate() {
        let start = row * seq;
        let n = row_ids.len().min(seq);
        ids[start..start + n].copy_from_slice(&row_ids[..n]);
        let m = row_mask.len().min(n);
        mask[start..start + m].copy_from_slice(&row_mask[..m]);
    }
    (batch, seq, ids, mask)
}

/// Resolve SentencePiece path: explicit path, else sibling of the ONNX model.
pub fn resolve_sentencepiece_path(model_path: &Path, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if !path.is_file() {
            return Err(Error::ConfigError(format!(
                "tokenizer not found: {}",
                path.display()
            )));
        }
        return Ok(path.to_path_buf());
    }

    let parent = model_path
        .parent()
        .ok_or_else(|| Error::ConfigError("model path has no parent directory".into()))?;

    for name in ["sentencepiece.bpe.model", "tokenizer.model", "spiece.model"] {
        let candidate = parent.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(Error::ConfigError(format!(
        "no SentencePiece tokenizer beside {}; pass --tokenizer PATH",
        model_path.display()
    )))
}

fn hash_tokenize(text: &str, max_seq_len: usize, vocab_size: usize) -> (Vec<i64>, Vec<i64>) {
    let mut ids = Vec::with_capacity(max_seq_len);
    for token in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if token.is_empty() {
            continue;
        }
        if ids.len() >= max_seq_len {
            break;
        }
        ids.push((hash_token(token) % vocab_size as u64) as i64);
    }
    if ids.is_empty() {
        ids.push(0);
    }
    let mask = vec![1i64; ids.len()];
    (ids, mask)
}

fn hash_token(token: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in token.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(feature = "semantic-onnx")]
fn sentencepiece_encode(
    text: &str,
    model_path: &Path,
    max_seq_len: usize,
    bos_id: i64,
    eos_id: i64,
    pad_id: i64,
) -> Result<(Vec<i64>, Vec<i64>)> {
    let sp = open_sentencepiece(model_path)?;
    sentencepiece_encode_with(&sp, text, max_seq_len, bos_id, eos_id, pad_id)
}

#[cfg(feature = "semantic-onnx")]
fn sentencepiece_encode_batch(
    texts: &[&str],
    model_path: &Path,
    max_seq_len: usize,
    bos_id: i64,
    eos_id: i64,
    pad_id: i64,
) -> Result<Vec<(Vec<i64>, Vec<i64>)>> {
    let sp = open_sentencepiece(model_path)?;
    texts
        .iter()
        .map(|text| sentencepiece_encode_with(&sp, text, max_seq_len, bos_id, eos_id, pad_id))
        .collect()
}

#[cfg(feature = "semantic-onnx")]
fn open_sentencepiece(model_path: &Path) -> Result<sentencepiece_rs::SentencePieceProcessor> {
    sentencepiece_rs::SentencePieceProcessor::open(model_path).map_err(|err| {
        Error::ConfigError(format!(
            "load SentencePiece {}: {err}",
            model_path.display()
        ))
    })
}

#[cfg(feature = "semantic-onnx")]
fn sentencepiece_encode_with(
    sp: &sentencepiece_rs::SentencePieceProcessor,
    text: &str,
    max_seq_len: usize,
    bos_id: i64,
    eos_id: i64,
    pad_id: i64,
) -> Result<(Vec<i64>, Vec<i64>)> {
    let mut ids: Vec<i64> = sp
        .encode_to_ids(text)
        .map_err(|err| Error::ConfigError(format!("tokenize: {err}")))?
        .into_iter()
        .map(|id| id as i64)
        .take(max_seq_len.saturating_sub(2))
        .collect();

    let mut input_ids = vec![bos_id];
    input_ids.append(&mut ids);
    input_ids.push(eos_id);

    let attention_mask: Vec<i64> = input_ids
        .iter()
        .map(|&id| if id == pad_id { 0 } else { 1 })
        .collect();

    Ok((input_ids, attention_mask))
}

#[cfg(not(feature = "semantic-onnx"))]
fn sentencepiece_encode(
    _text: &str,
    _model_path: &Path,
    _max_seq_len: usize,
    _bos_id: i64,
    _eos_id: i64,
    _pad_id: i64,
) -> Result<(Vec<i64>, Vec<i64>)> {
    Err(Error::ConfigError(
        "SentencePiece tokenization requires `--features semantic-onnx`".into(),
    ))
}

#[cfg(not(feature = "semantic-onnx"))]
fn sentencepiece_encode_batch(
    _texts: &[&str],
    _model_path: &Path,
    _max_seq_len: usize,
    _bos_id: i64,
    _eos_id: i64,
    _pad_id: i64,
) -> Result<Vec<(Vec<i64>, Vec<i64>)>> {
    Err(Error::ConfigError(
        "SentencePiece tokenization requires `--features semantic-onnx`".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_tokenize_never_empty() {
        let (ids, mask) = hash_tokenize("", 16, 1000);
        assert_eq!(ids.len(), 1);
        assert_eq!(mask.len(), ids.len());
    }

    #[test]
    fn pad_encoded_batch_aligns_rows() {
        let encoded = vec![(vec![1, 2], vec![1, 1]), (vec![3], vec![1])];
        let (batch, seq, ids, mask) = pad_encoded_batch(&encoded, 0);
        assert_eq!((batch, seq), (2, 2));
        assert_eq!(ids, vec![1, 2, 3, 0]);
        assert_eq!(mask, vec![1, 1, 1, 0]);
    }

    #[test]
    fn hash_encode_batch_preserves_order() {
        let tok = OnnxTokenizer::Hash {
            max_seq_len: 16,
            vocab_size: 1000,
        };
        let batch = tok.encode_batch(&["alpha", "beta"]).unwrap();
        assert_eq!(batch[0], tok.encode("alpha").unwrap());
        assert_eq!(batch[1], tok.encode("beta").unwrap());
    }
}

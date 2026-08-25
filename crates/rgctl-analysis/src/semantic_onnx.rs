//! ONNX Runtime embedder (feature `semantic-onnx`).

use crate::semantic_embedder::SemanticEmbedder;
use crate::semantic_onnx_tokenizer::{OnnxTokenizer, pad_encoded_batch};
use ndarray::{Array2, ArrayD, ArrayView2, Axis};
use ort::inputs;
use ort::session::Session;
use ort::session::builder::SessionBuilder;
use ort::value::TensorRef;
use rgctl_error::{Error, Result};
use std::borrow::Cow;
use std::path::Path;
use std::sync::Mutex;
use tracing::warn;

const DEFAULT_MAX_SEQ_LEN: usize = 128;
const DEFAULT_VOCAB_SIZE: usize = 30_522;
/// Padded `[N, seq]` width for index/distill. Small enough for CPU cache; large enough to amortize `session.run`.
const ONNX_INDEX_BATCH: usize = 32;

/// Post-inference vector processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Postprocess {
    /// Truncate/pad to target width (generic ONNX).
    Resize,
    /// MRL truncate + L2 normalize (code-daemon).
    CodeDaemonMrl,
}

/// Mutex-backed ONNX embedder safe to share across query threads.
pub struct SharedOnnxEmbedder {
    model_id: String,
    dimensions: usize,
    native_dims: usize,
    input_ids_name: String,
    attention_mask_name: Option<String>,
    tokenizer: OnnxTokenizer,
    postprocess: Postprocess,
    session: Mutex<Session>,
}

impl SharedOnnxEmbedder {
    /// Load a generic ONNX model (hash tokenization unless tokenizer supplied).
    pub fn load(path: &Path, dimensions: usize) -> Result<Self> {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("onnx");
        Self::load_with(
            path,
            &format!("onnx:{stem}"),
            dimensions,
            dimensions,
            OnnxTokenizer::Hash {
                max_seq_len: DEFAULT_MAX_SEQ_LEN,
                vocab_size: DEFAULT_VOCAB_SIZE,
            },
            Postprocess::Resize,
        )
    }

    /// Load with explicit tokenizer and post-processing.
    pub fn load_with(
        path: &Path,
        model_id: &str,
        dimensions: usize,
        native_dims: usize,
        tokenizer: OnnxTokenizer,
        postprocess: Postprocess,
    ) -> Result<Self> {
        if !path.is_file() {
            return Err(Error::ConfigError(format!(
                "ONNX model not found: {}",
                path.display()
            )));
        }

        let mut builder = Session::builder().map_err(map_ort)?;
        let session = builder.commit_from_file(path).map_err(map_ort)?;
        Self::from_session(
            model_id,
            dimensions,
            native_dims,
            tokenizer,
            postprocess,
            session,
        )
    }

    /// Load ONNX graph + external weight blob from compiled-in bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn load_from_embedded(
        model_bytes: &'static [u8],
        external_data_name: &'static str,
        external_data_bytes: &'static [u8],
        model_id: &str,
        dimensions: usize,
        native_dims: usize,
        tokenizer: OnnxTokenizer,
        postprocess: Postprocess,
    ) -> Result<Self> {
        let mut builder = Session::builder().map_err(map_ort)?;
        builder = builder
            .with_external_initializer_file_in_memory(
                Path::new(external_data_name),
                Cow::Borrowed(external_data_bytes),
            )
            .map_err(map_builder)?;
        let session = builder.commit_from_memory(model_bytes).map_err(map_ort)?;
        Self::from_session(
            model_id,
            dimensions,
            native_dims,
            tokenizer,
            postprocess,
            session,
        )
    }

    fn from_session(
        model_id: &str,
        dimensions: usize,
        native_dims: usize,
        tokenizer: OnnxTokenizer,
        postprocess: Postprocess,
        session: Session,
    ) -> Result<Self> {
        let input_ids_name = session
            .inputs()
            .first()
            .ok_or_else(|| Error::ConfigError("ONNX model has no inputs".into()))?
            .name()
            .to_string();

        let attention_mask_name = session
            .inputs()
            .iter()
            .map(|input| input.name())
            .find(|name| name.contains("attention"))
            .map(str::to_string);

        Ok(Self {
            model_id: model_id.to_string(),
            dimensions,
            native_dims,
            input_ids_name,
            attention_mask_name,
            tokenizer,
            postprocess,
            session: Mutex::new(session),
        })
    }

    /// Load generic ONNX with optional SentencePiece tokenizer (auto-detect beside model).
    pub fn load_with_optional_tokenizer(
        path: &Path,
        dimensions: usize,
        tokenizer_path: Option<&Path>,
    ) -> Result<Self> {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("onnx");

        let tokenizer = if let Some(explicit) = tokenizer_path {
            let sp_path =
                crate::semantic_onnx_tokenizer::resolve_sentencepiece_path(path, Some(explicit))?;
            OnnxTokenizer::SentencePiece {
                path: sp_path,
                max_seq_len: DEFAULT_MAX_SEQ_LEN,
                bos_id: 2,
                eos_id: 3,
                pad_id: 0,
            }
        } else if let Ok(sp_path) =
            crate::semantic_onnx_tokenizer::resolve_sentencepiece_path(path, None)
        {
            OnnxTokenizer::SentencePiece {
                path: sp_path,
                max_seq_len: DEFAULT_MAX_SEQ_LEN,
                bos_id: 2,
                eos_id: 3,
                pad_id: 0,
            }
        } else {
            OnnxTokenizer::Hash {
                max_seq_len: DEFAULT_MAX_SEQ_LEN,
                vocab_size: DEFAULT_VOCAB_SIZE,
            }
        };

        Self::load_with(
            path,
            &format!("onnx:{stem}"),
            dimensions,
            dimensions,
            tokenizer,
            Postprocess::Resize,
        )
    }
}

impl SemanticEmbedder for SharedOnnxEmbedder {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn preferred_batch_size(&self) -> usize {
        ONNX_INDEX_BATCH
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.infer_one(text)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        if texts.len() == 1 {
            return Ok(vec![self.infer_one(texts[0])?]);
        }
        match self.infer_batch(texts) {
            Ok(rows) => Ok(rows),
            Err(err) => {
                warn!(
                    error = %err,
                    batch = texts.len(),
                    "ONNX batched session.run failed; falling back to serial embeds"
                );
                texts.iter().map(|text| self.infer_one(text)).collect()
            }
        }
    }
}

impl SharedOnnxEmbedder {
    fn infer_one(&self, text: &str) -> Result<Vec<f32>> {
        let (ids, mask) = self.tokenizer.encode(text)?;
        let seq_len = ids.len();
        let input_ids = Array2::from_shape_vec((1, seq_len), ids).map_err(map_shape)?;
        let attention = Array2::from_shape_vec((1, seq_len), mask.clone()).map_err(map_shape)?;
        let tensor = self.run_session(input_ids, attention)?;
        let mut rows = vectors_from_output(tensor, 1, self.native_dims, &[mask])?;
        apply_postprocess(
            rows.pop().ok_or_else(|| {
                Error::ConfigError("ONNX produced no vectors for a single input".into())
            })?,
            self.dimensions,
            self.postprocess,
        )
    }

    fn infer_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let encoded = self.tokenizer.encode_batch(texts)?;
        if encoded.len() != texts.len() {
            return Err(Error::ConfigError(format!(
                "encode_batch returned {} rows for {} texts",
                encoded.len(),
                texts.len()
            )));
        }
        let pad_id = self.tokenizer.pad_id();
        let (batch, seq, ids, mask) = pad_encoded_batch(&encoded, pad_id);
        let input_ids = Array2::from_shape_vec((batch, seq), ids).map_err(map_shape)?;
        let attention = Array2::from_shape_vec((batch, seq), mask.clone()).map_err(map_shape)?;
        let tensor = self.run_session(input_ids, attention)?;
        let row_masks: Vec<Vec<i64>> = mask.chunks(seq).map(<[i64]>::to_vec).collect();
        let rows = vectors_from_output(tensor, batch, self.native_dims, &row_masks)?;
        rows.into_iter()
            .map(|row| apply_postprocess(row, self.dimensions, self.postprocess))
            .collect()
    }

    fn run_session(&self, input_ids: Array2<i64>, attention: Array2<i64>) -> Result<ArrayD<f32>> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| Error::GraphError("ONNX session lock poisoned".into()))?;

        let outputs = if let Some(mask_name) = &self.attention_mask_name {
            session
                .run(inputs![
                    self.input_ids_name.as_str() =>
                        TensorRef::from_array_view(input_ids.view()).map_err(map_ort)?,
                    mask_name.as_str() =>
                        TensorRef::from_array_view(attention.view()).map_err(map_ort)?
                ])
                .map_err(map_ort)?
        } else {
            session
                .run(inputs![
                    self.input_ids_name.as_str() =>
                        TensorRef::from_array_view(input_ids.view()).map_err(map_ort)?
                ])
                .map_err(map_ort)?
        };

        outputs[0]
            .try_extract_array::<f32>()
            .map_err(map_ort)
            .map(|tensor| tensor.to_owned())
    }
}

fn apply_postprocess(
    mut values: Vec<f32>,
    dimensions: usize,
    mode: Postprocess,
) -> Result<Vec<f32>> {
    match mode {
        Postprocess::Resize => Ok(resize_or_truncate(&values, dimensions)),
        Postprocess::CodeDaemonMrl => {
            if values.len() > dimensions {
                values.truncate(dimensions);
            }
            l2_normalize(&mut values);
            Ok(values)
        }
    }
}

fn vectors_from_output(
    tensor: ArrayD<f32>,
    batch: usize,
    native_dims: usize,
    masks: &[Vec<i64>],
) -> Result<Vec<Vec<f32>>> {
    match tensor.ndim() {
        1 => {
            if batch != 1 {
                return Err(Error::ConfigError(format!(
                    "ONNX rank-1 output cannot serve batch {batch}"
                )));
            }
            Ok(vec![resize_or_truncate(
                tensor
                    .as_slice()
                    .ok_or_else(|| Error::ConfigError("ONNX output not contiguous".into()))?,
                native_dims,
            )])
        }
        2 => {
            let mut out = Vec::with_capacity(batch);
            for i in 0..batch {
                let row = tensor.index_axis(Axis(0), i);
                let owned: Vec<f32> = row.iter().copied().collect();
                out.push(resize_or_truncate(&owned, native_dims));
            }
            Ok(out)
        }
        3 => {
            let mut out = Vec::with_capacity(batch);
            for i in 0..batch {
                let seq = tensor.index_axis(Axis(0), i);
                let seq2 = seq.into_dimensionality::<ndarray::Ix2>().map_err(|_| {
                    Error::ConfigError("ONNX rank-3 row is not [seq, hidden]".into())
                })?;
                let mask = masks.get(i).map(Vec::as_slice).unwrap_or(&[]);
                out.push(mean_pool_masked(seq2, mask, native_dims));
            }
            Ok(out)
        }
        other => Err(Error::ConfigError(format!(
            "unsupported ONNX output rank {other}"
        ))),
    }
}

fn mean_pool_masked(seq_hidden: ArrayView2<f32>, mask: &[i64], native_dims: usize) -> Vec<f32> {
    let seq_len = seq_hidden.shape()[0];
    let hidden = seq_hidden.shape()[1];
    let mut pooled = vec![0f32; hidden];
    let mut count = 0f32;
    for seq_idx in 0..seq_len {
        if mask.get(seq_idx).copied().unwrap_or(1) == 0 {
            continue;
        }
        let row = seq_hidden.index_axis(Axis(0), seq_idx);
        for (slot, value) in pooled.iter_mut().zip(row.iter()) {
            *slot += *value;
        }
        count += 1.0;
    }
    if count > 0.0 {
        for value in &mut pooled {
            *value /= count;
        }
    }
    resize_or_truncate(&pooled, native_dims)
}

fn resize_or_truncate(values: &[f32], dimensions: usize) -> Vec<f32> {
    if values.len() == dimensions {
        return values.to_vec();
    }
    if values.len() > dimensions {
        return values[..dimensions].to_vec();
    }
    let mut out = values.to_vec();
    out.resize(dimensions, 0.0);
    out
}

fn l2_normalize(values: &mut [f32]) {
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for value in values {
            *value /= norm;
        }
    }
}

fn map_ort(err: ort::Error) -> Error {
    Error::ConfigError(format!("ONNX Runtime: {err}"))
}

fn map_builder(err: ort::Error<SessionBuilder>) -> Error {
    Error::ConfigError(format!("ONNX session builder: {err}"))
}

fn map_shape(err: ndarray::ShapeError) -> Error {
    Error::ConfigError(format!("tensor shape: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank2_batch_keeps_row_order() {
        let tensor = ArrayD::from_shape_vec(vec![2, 2], vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let rows = vectors_from_output(tensor, 2, 2, &[]).unwrap();
        assert_eq!(rows[0], vec![1.0, 2.0]);
        assert_eq!(rows[1], vec![3.0, 4.0]);
    }

    #[test]
    fn rank3_mean_pool_skips_masked_tokens() {
        let tensor = ArrayD::from_shape_vec(vec![1, 2, 1], vec![10.0, 999.0]).unwrap();
        let rows = vectors_from_output(tensor, 1, 1, &[vec![1, 0]]).unwrap();
        assert_eq!(rows[0][0], 10.0);
    }
}

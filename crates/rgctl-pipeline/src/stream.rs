//! Bounded-channel streaming between parallel extractors and sequential graph merge.

use crate::parallel::with_pool;
use crossbeam::channel::{Receiver, bounded};
use rayon::prelude::*;
use rgctl_error::Result;
use rgctl_extraction::{ExtractionTail, Extractor, FileExtraction, GraphBuilder};
use rgctl_registry::LanguageRegistry;
use std::path::PathBuf;
use std::sync::Arc;

/// Default in-flight extraction cap (~1024 file buffers max between extract and merge).
pub const DEFAULT_STREAM_CHANNEL_CAPACITY: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionFailure {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StreamStats {
    pub files_processed: usize,
    pub extraction_failures: Vec<ExtractionFailure>,
}

/// Run parallel extractors into a bounded channel while the caller consumes on the main thread.
pub fn start_parallel_extraction(
    thread_count: Option<usize>,
    registry: Arc<LanguageRegistry>,
    files: Arc<Vec<PathBuf>>,
    capacity: usize,
    on_file_done: impl Fn() + Send + Sync + 'static,
) -> Receiver<std::result::Result<FileExtraction, ExtractionFailure>> {
    let (tx, rx) = bounded(capacity);
    std::thread::spawn(move || {
        with_pool(thread_count, || {
            files.par_iter().for_each(|path| {
                let extractor = Extractor::new(Arc::clone(&registry));
                match extractor.extract_file(path) {
                    Ok(extraction) => {
                        let _ = tx.send(Ok(extraction));
                    }
                    Err(err) => {
                        let _ = tx.send(Err(ExtractionFailure {
                            path: path.clone(),
                            error: err.to_string(),
                        }));
                    }
                }
                on_file_done();
            });
        });
    });
    rx
}

/// Extract in parallel, merge pass-1 immediately, and retain only relation tails for pass 2.
pub fn stream_into_graph(
    thread_count: Option<usize>,
    extractor: &Extractor,
    registry: Arc<LanguageRegistry>,
    files: &[PathBuf],
    capacity: usize,
    builder: &mut GraphBuilder,
    on_file_done: impl Fn() + Send + Sync + 'static,
) -> Result<(StreamStats, Vec<ExtractionTail>)> {
    let files = Arc::new(files.to_vec());
    let file_count = files.len();
    let rx = start_parallel_extraction(thread_count, registry, files, capacity, on_file_done);

    let mut tails = Vec::with_capacity(file_count);
    let mut stats = StreamStats::default();
    while let Ok(result) = rx.recv() {
        match result {
            Ok(mut extraction) => {
                tails.push(extractor.populate_pass1(&mut extraction, builder)?);
                stats.files_processed += 1;
            }
            Err(failure) => {
                stats.extraction_failures.push(failure);
            }
        }
    }

    Ok((stats, tails))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn stream_reports_extraction_failures() {
        let registry = Arc::new(rgctl_languages::default_registry());
        let extractor = Extractor::new(Arc::clone(&registry));
        let mut builder = GraphBuilder::new();
        let missing = TempDir::new().unwrap().path().join("missing.rs");

        let (stats, tails) = stream_into_graph(
            Some(1),
            &extractor,
            registry,
            &[missing.clone()],
            8,
            &mut builder,
            || {},
        )
        .unwrap();

        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.extraction_failures.len(), 1);
        assert_eq!(stats.extraction_failures[0].path, missing);
        assert!(tails.is_empty());
    }
}

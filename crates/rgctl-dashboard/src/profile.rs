//! Stage timing helpers for save-path profiling (`RUST_LOG=profile=info`).

use std::time::Instant;

/// Run `f`, log wall seconds under `[profile] save_dashboard stage`.
pub fn profile_stage<T, F>(stage: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let out = f();
    tracing::info!(
        target: "profile",
        stage,
        secs = start.elapsed().as_secs_f64(),
        "[profile] save_dashboard stage"
    );
    out
}

//! Parallel execution helpers (Phase 8.1)

use rayon::ThreadPool;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

static POOL_REGISTRY: OnceLock<Mutex<HashMap<Option<usize>, Arc<ThreadPool>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<Option<usize>, Arc<ThreadPool>>> {
    POOL_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Return a process-wide Rayon pool for `thread_count` (`None` = global default pool).
pub fn thread_pool(thread_count: Option<usize>) -> Arc<ThreadPool> {
    let mut lock = registry()
        .lock()
        .expect("rayon pool registry lock poisoned");
    if let Some(pool) = lock.get(&thread_count) {
        return Arc::clone(pool);
    }
    let mut builder =
        rayon::ThreadPoolBuilder::new().thread_name(|idx| format!("rgctl-worker-{idx}"));
    if let Some(n) = thread_count {
        builder = builder.num_threads(n);
    }
    let pool = Arc::new(builder.build().expect("failed to build rayon thread pool"));
    lock.insert(thread_count, Arc::clone(&pool));
    pool
}

/// Run `f` on the pooled Rayon executor for `thread_count`.
pub fn with_pool<R, F>(thread_count: Option<usize>, f: F) -> R
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    thread_pool(thread_count).install(f)
}

/// Run a parallel iterator over `items`, optionally on a dedicated thread pool.
pub fn par_map<T, R, F>(thread_count: Option<usize>, items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync + Send,
{
    with_pool(thread_count, || items.par_iter().map(f).collect())
}

/// Run a parallel iterator and keep only `Some` results.
pub fn par_filter_map<T, R, F>(thread_count: Option<usize>, items: &[T], f: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> Option<R> + Sync + Send,
{
    with_pool(thread_count, || items.par_iter().filter_map(f).collect())
}

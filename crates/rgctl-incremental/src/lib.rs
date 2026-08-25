//! Incremental graph updates and change detection

pub mod changes;
pub mod file_tracker;
pub mod updater;

pub use changes::{ChangeDetail, ChangeDetectionResult, ChangeDetector, ChangeSummary};
pub use file_tracker::{ChangeSet, FileTracker, changes_for_paths, normalize_path_str};
pub use updater::{IncrementalUpdater, UpdateOptions, UpdateResult};

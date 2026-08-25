//! On-disk artifact directory and environment helpers for rgBuilder.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Canonical artifact directory name under a repository root.
pub const ARTIFACT_DIR_NAME: &str = ".rgbuilder";

/// Legacy artifact directory (pre-rename).
pub const LEGACY_ARTIFACT_DIR_NAME: &str = ".rbuilder";

/// Return `<repo>/.rgbuilder`, migrating from `.rbuilder` when needed.
///
/// Migration rules:
/// - If `.rgbuilder` is missing and `.rbuilder` exists → rename (one-shot).
/// - If both exist → prefer `.rgbuilder` and print a warning (no merge).
pub fn artifact_dir(repo_root: &Path) -> PathBuf {
    let _ = ensure_artifact_dir_migrated(repo_root);
    repo_root.join(ARTIFACT_DIR_NAME)
}

/// Ensure legacy `.rbuilder` is migrated; returns the active artifact path.
pub fn ensure_artifact_dir_migrated(repo_root: &Path) -> PathBuf {
    let neu = repo_root.join(ARTIFACT_DIR_NAME);
    let old = repo_root.join(LEGACY_ARTIFACT_DIR_NAME);
    let neu_exists = neu.exists();
    let old_exists = old.exists();

    if neu_exists && old_exists {
        eprintln!(
            "[rgctl] warning: both {} and {} exist; using {}",
            LEGACY_ARTIFACT_DIR_NAME,
            ARTIFACT_DIR_NAME,
            neu.display()
        );
        return neu;
    }

    if !neu_exists && old_exists {
        match std::fs::rename(&old, &neu) {
            Ok(()) => {
                eprintln!("[rgctl] migrated {} → {}", old.display(), neu.display());
            }
            Err(e) => {
                eprintln!(
                    "[rgctl] warning: could not migrate {} → {}: {e}; re-run `rgctl discover`",
                    old.display(),
                    neu.display()
                );
                // Fall back to legacy path so reads can still work until re-discover.
                return old;
            }
        }
    }

    neu
}

/// Join a relative path under the artifact directory (after migration).
pub fn artifact_path(repo_root: &Path, relative: impl AsRef<Path>) -> PathBuf {
    artifact_dir(repo_root).join(relative)
}

/// Read `RGBUILDER_{suffix}`, falling back to legacy `RBUILDER_{suffix}` when unset.
pub fn env_var(suffix: &str) -> Option<String> {
    env_var_os(suffix).and_then(|v| v.into_string().ok())
}

/// OS-string variant of [`env_var`].
pub fn env_var_os(suffix: &str) -> Option<OsString> {
    let neu = format!("RGBUILDER_{suffix}");
    if let Some(v) = env::var_os(&neu) {
        return Some(v);
    }
    let legacy = format!("RBUILDER_{suffix}");
    env::var_os(legacy)
}

/// True when `RGBUILDER_{suffix}` or legacy `RBUILDER_{suffix}` is set (any value).
pub fn env_flag_set(suffix: &str) -> bool {
    env_var_os(suffix).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn migrates_legacy_dir() {
        let tmp = TempDir::new().unwrap();
        let old = tmp.path().join(LEGACY_ARTIFACT_DIR_NAME);
        std::fs::create_dir_all(old.join("dashboard")).unwrap();
        std::fs::write(old.join("marker.txt"), b"ok").unwrap();

        let neu = ensure_artifact_dir_migrated(tmp.path());
        assert_eq!(neu, tmp.path().join(ARTIFACT_DIR_NAME));
        assert!(neu.join("marker.txt").is_file());
        assert!(!old.exists());
    }

    #[test]
    fn prefers_new_when_both_exist() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(ARTIFACT_DIR_NAME)).unwrap();
        std::fs::create_dir_all(tmp.path().join(LEGACY_ARTIFACT_DIR_NAME)).unwrap();
        let neu = ensure_artifact_dir_migrated(tmp.path());
        assert_eq!(neu, tmp.path().join(ARTIFACT_DIR_NAME));
        assert!(tmp.path().join(LEGACY_ARTIFACT_DIR_NAME).exists());
    }

    #[test]
    fn env_prefers_new_prefix() {
        let suffix = "TEST_RENAME_ENV_UNIQUE";
        unsafe {
            env::remove_var(format!("RGBUILDER_{suffix}"));
            env::remove_var(format!("RBUILDER_{suffix}"));
            env::set_var(format!("RBUILDER_{suffix}"), "legacy");
        }
        assert_eq!(env_var(suffix).as_deref(), Some("legacy"));
        unsafe {
            env::set_var(format!("RGBUILDER_{suffix}"), "new");
        }
        assert_eq!(env_var(suffix).as_deref(), Some("new"));
        unsafe {
            env::remove_var(format!("RGBUILDER_{suffix}"));
            env::remove_var(format!("RBUILDER_{suffix}"));
        }
    }
}

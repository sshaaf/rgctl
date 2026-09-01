//! On-disk artifact directory and environment helpers for rgctl.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Canonical artifact directory name under a repository root.
pub const ARTIFACT_DIR_NAME: &str = ".rgctl";

/// Previous artifact directory (rgBuilder era).
pub const LEGACY_RGBUILDER_DIR_NAME: &str = ".rgbuilder";

/// Oldest artifact directory (pre-rgBuilder rename).
pub const LEGACY_RBUILDER_DIR_NAME: &str = ".rbuilder";

/// Return `<repo>/.rgctl`, migrating from legacy dirs when needed.
pub fn artifact_dir(repo_root: &Path) -> PathBuf {
    let _ = ensure_artifact_dir_migrated(repo_root);
    repo_root.join(ARTIFACT_DIR_NAME)
}

fn try_rename(from: &Path, to: &Path) -> bool {
    if !from.exists() || to.exists() {
        return false;
    }
    match std::fs::rename(from, to) {
        Ok(()) => {
            eprintln!("[rgctl] migrated {} → {}", from.display(), to.display());
            true
        }
        Err(e) => {
            eprintln!(
                "[rgctl] warning: could not migrate {} → {}: {e}",
                from.display(),
                to.display()
            );
            false
        }
    }
}

/// Ensure legacy artifact dirs are migrated; returns the active artifact path.
pub fn ensure_artifact_dir_migrated(repo_root: &Path) -> PathBuf {
    let neu = repo_root.join(ARTIFACT_DIR_NAME);
    let rgb = repo_root.join(LEGACY_RGBUILDER_DIR_NAME);
    let old = repo_root.join(LEGACY_RBUILDER_DIR_NAME);

    if neu.exists() {
        if rgb.exists() {
            eprintln!(
                "[rgctl] warning: both {} and {} exist; using {}",
                LEGACY_RGBUILDER_DIR_NAME,
                ARTIFACT_DIR_NAME,
                neu.display()
            );
        }
        if old.exists() {
            eprintln!(
                "[rgctl] warning: both {} and {} exist; using {}",
                LEGACY_RBUILDER_DIR_NAME,
                ARTIFACT_DIR_NAME,
                neu.display()
            );
        }
        return neu;
    }

    // Chain: .rbuilder → .rgbuilder → .rgctl
    if !rgb.exists() && old.exists() {
        let _ = try_rename(&old, &rgb);
    }
    if rgb.exists() {
        if try_rename(&rgb, &neu) {
            return neu;
        }
        return rgb;
    }
    if old.exists() {
        if try_rename(&old, &neu) {
            return neu;
        }
        return old;
    }

    neu
}

/// Join a relative path under the artifact directory (after migration).
pub fn artifact_path(repo_root: &Path, relative: impl AsRef<Path>) -> PathBuf {
    artifact_dir(repo_root).join(relative)
}

/// Read `RGCTL_{suffix}`, falling back to legacy `RGBUILDER_{suffix}` then `RBUILDER_{suffix}`.
pub fn env_var(suffix: &str) -> Option<String> {
    env_var_os(suffix).and_then(|v| v.into_string().ok())
}

/// OS-string variant of [`env_var`].
pub fn env_var_os(suffix: &str) -> Option<OsString> {
    let canonical = format!("RGCTL_{suffix}");
    if let Some(v) = env::var_os(&canonical) {
        return Some(v);
    }
    let rgb = format!("RGBUILDER_{suffix}");
    if let Some(v) = env::var_os(&rgb) {
        return Some(v);
    }
    let legacy = format!("RBUILDER_{suffix}");
    env::var_os(legacy)
}

/// True when any recognized env prefix is set for `suffix`.
pub fn env_flag_set(suffix: &str) -> bool {
    env_var_os(suffix).is_some()
}

/// Default user home for legacy daemon cache layout (`$HOME` / `%USERPROFILE%`).
pub fn default_user_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(p) = env::var("USERPROFILE") {
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
    }
    env::var("HOME")
        .ok()
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
}

/// Legacy daemon workspace root (`RGCTL_HOME` or user home).
pub fn legacy_daemon_home() -> Option<PathBuf> {
    if let Ok(p) = env::var("RGCTL_HOME") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    default_user_home()
}

/// One-shot migrate `~/.rgbuilder/` → `~/.rgctl/` when the new dir is absent.
pub fn migrate_legacy_daemon_home(home_root: &Path) -> std::io::Result<()> {
    let neu = home_root.join(".rgctl");
    let old = home_root.join(".rgbuilder");
    if neu.exists() || !old.exists() {
        return Ok(());
    }
    match std::fs::rename(&old, &neu) {
        Ok(()) => {
            eprintln!(
                "[rgctl] migrated daemon home {} → {}",
                old.display(),
                neu.display()
            );
        }
        Err(e) => {
            eprintln!(
                "[rgctl] warning: could not migrate daemon home {} → {}: {e}",
                old.display(),
                neu.display()
            );
        }
    }
    Ok(())
}

/// Path to cached artifacts for a daemon-era repo name: `~/.rgctl/cache/{name}/.rgctl/`.
pub fn daemon_cache_artifacts(name: &str) -> Option<PathBuf> {
    let home = legacy_daemon_home()?;
    let _ = migrate_legacy_daemon_home(&home);
    let root = home.join(".rgctl").join("cache").join(name);
    Some(root.join(ARTIFACT_DIR_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn migrates_rbuilder_to_rgctl() {
        let tmp = TempDir::new().unwrap();
        let old = tmp.path().join(LEGACY_RBUILDER_DIR_NAME);
        std::fs::create_dir_all(old.join("dashboard")).unwrap();
        std::fs::write(old.join("marker.txt"), b"ok").unwrap();

        let neu = ensure_artifact_dir_migrated(tmp.path());
        assert_eq!(neu, tmp.path().join(ARTIFACT_DIR_NAME));
        assert!(neu.join("marker.txt").is_file());
        assert!(!old.exists());
    }

    #[test]
    fn migrates_rgbuilder_to_rgctl() {
        let tmp = TempDir::new().unwrap();
        let mid = tmp.path().join(LEGACY_RGBUILDER_DIR_NAME);
        std::fs::create_dir_all(mid.join("dashboard")).unwrap();
        std::fs::write(mid.join("marker.txt"), b"ok").unwrap();

        let neu = ensure_artifact_dir_migrated(tmp.path());
        assert_eq!(neu, tmp.path().join(ARTIFACT_DIR_NAME));
        assert!(neu.join("marker.txt").is_file());
        assert!(!mid.exists());
    }

    #[test]
    fn prefers_new_when_both_exist() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(ARTIFACT_DIR_NAME)).unwrap();
        std::fs::create_dir_all(tmp.path().join(LEGACY_RGBUILDER_DIR_NAME)).unwrap();
        let neu = ensure_artifact_dir_migrated(tmp.path());
        assert_eq!(neu, tmp.path().join(ARTIFACT_DIR_NAME));
        assert!(tmp.path().join(LEGACY_RGBUILDER_DIR_NAME).exists());
    }

    #[test]
    fn env_prefers_canonical_prefix() {
        let suffix = "TEST_RENAME_ENV_UNIQUE";
        unsafe {
            env::remove_var(format!("RGCTL_{suffix}"));
            env::remove_var(format!("RGBUILDER_{suffix}"));
            env::remove_var(format!("RBUILDER_{suffix}"));
            env::set_var(format!("RBUILDER_{suffix}"), "legacy");
        }
        assert_eq!(env_var(suffix).as_deref(), Some("legacy"));
        unsafe {
            env::set_var(format!("RGBUILDER_{suffix}"), "rgb");
        }
        assert_eq!(env_var(suffix).as_deref(), Some("rgb"));
        unsafe {
            env::set_var(format!("RGCTL_{suffix}"), "new");
        }
        assert_eq!(env_var(suffix).as_deref(), Some("new"));
        unsafe {
            env::remove_var(format!("RGCTL_{suffix}"));
            env::remove_var(format!("RGBUILDER_{suffix}"));
            env::remove_var(format!("RBUILDER_{suffix}"));
        }
    }
}

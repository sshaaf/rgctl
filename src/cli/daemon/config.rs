//! Daemon home layout and config.toml.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Filesystem layout for one daemon workspace.
#[derive(Clone, Debug)]
pub struct DaemonHome {
    root: PathBuf,
}

/// Default daemon workspace root: `$HOME` (Unix) or `%USERPROFILE%` (Windows).
/// Daemon state is stored under `{root}/.rgctl/`.
pub fn default_home_root() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(p) = std::env::var("USERPROFILE") {
            if !p.is_empty() {
                return Ok(PathBuf::from(p));
            }
        }
    }
    if let Ok(p) = std::env::var("HOME") {
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    bail!("cannot resolve user home directory (set RGCTL_HOME or pass --daemon-home)")
}

impl DaemonHome {
    pub fn from_path(path: &Path) -> Result<Self> {
        let root = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn rgctl_dir(&self) -> PathBuf {
        self.root.join(".rgctl")
    }

    pub fn config_path(&self) -> PathBuf {
        self.rgctl_dir().join(".config").join("config.toml")
    }

    pub fn cache_root(&self, cfg: &DaemonConfig) -> PathBuf {
        if let Some(storage) = &cfg.storage {
            PathBuf::from(storage).join("cache")
        } else {
            self.rgctl_dir().join("cache")
        }
    }

    pub fn repo_dir(&self, cfg: &DaemonConfig, name: &str) -> PathBuf {
        self.cache_root(cfg).join(name)
    }

    pub fn pid_path(&self) -> PathBuf {
        self.rgctl_dir().join("rgctl.pid")
    }

    pub fn pid_file(&self) -> PathBuf {
        self.pid_path()
    }

    pub fn log_path(&self) -> PathBuf {
        self.rgctl_dir().join("rgctl.log")
    }

    pub fn log_file(&self) -> PathBuf {
        self.log_path()
    }

    pub fn lock_path(&self) -> PathBuf {
        self.rgctl_dir().join("rgctl.lock")
    }

    pub fn lock_file(&self) -> PathBuf {
        self.lock_path()
    }

    #[cfg(unix)]
    pub fn control_path(&self) -> PathBuf {
        self.rgctl_dir().join("rgctl.sock")
    }

    #[cfg(windows)]
    pub fn control_path(&self) -> PathBuf {
        self.rgctl_dir().join("rgctl.pipe")
    }

    pub fn control_file(&self) -> PathBuf {
        self.control_path()
    }

    pub fn ensure_layout(&self) -> Result<()> {
        migrate_legacy_daemon_home(&self.root)?;
        fs::create_dir_all(self.rgctl_dir().join(".config"))?;
        fs::create_dir_all(self.rgctl_dir().join("cache"))?;
        Ok(())
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        self.ensure_layout()
    }

    /// Exclusive create of lock file (single instance).
    pub fn try_lock(&self) -> Result<fs::File> {
        self.ensure_layout()?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.lock_path())
        {
            Ok(f) => Ok(f),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                bail!("daemon lock exists at {}", self.lock_path().display());
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub storage: Option<String>,
    #[serde(default)]
    pub default_repo: Option<String>,
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub repo: Vec<RepoEntry>,
}

fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    8080
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            storage: None,
            default_repo: None,
            mcp: McpConfig::default(),
            repo: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_mcp_path")]
    pub path: String,
    #[serde(default)]
    pub clients: Vec<McpClient>,
}

fn default_true() -> bool {
    true
}
fn default_mcp_path() -> String {
    "/mcp".into()
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_mcp_path(),
            clients: vec![McpClient {
                name: "cursor".into(),
                transport: "http".into(),
                url: "http://127.0.0.1:8080/mcp".into(),
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClient {
    pub name: String,
    #[serde(default = "default_http")]
    pub transport: String,
    pub url: String,
}

fn default_http() -> String {
    "http".into()
}

/// One-shot migrate `~/.rgbuilder/` → `~/.rgctl/` when the new dir is absent.
fn migrate_legacy_daemon_home(home_root: &Path) -> Result<()> {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    pub path: String,
    #[serde(default)]
    pub name: Option<String>,
}

impl DaemonConfig {
    pub fn load_or_init(home: &DaemonHome) -> Result<Self> {
        home.ensure_layout()?;
        let path = home.config_path();
        if !path.is_file() {
            let cfg = Self::default();
            let body = toml::to_string_pretty(&cfg).context("serialize default daemon config")?;
            fs::write(&path, body)?;
            return Ok(cfg);
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
    }

    pub fn name_override(&self, source: &Path) -> Option<String> {
        self.name_for_path(source)
    }

    pub fn name_for_path(&self, source: &Path) -> Option<String> {
        let canon = source.canonicalize().unwrap_or_else(|_| source.to_path_buf());
        for r in &self.repo {
            let p = PathBuf::from(&r.path);
            let pc = p.canonicalize().unwrap_or(p);
            if pc == canon {
                if let Some(n) = &r.name {
                    return Some(n.clone());
                }
            }
        }
        None
    }
}

pub fn sanitize_reponame(source: &Path, override_name: Option<&str>) -> Result<String> {
    if let Some(n) = override_name {
        return validate_name(n);
    }
    let file = source
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("repo");
    validate_name(file)
}

pub fn validate_name(name: &str) -> Result<String> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\')
    {
        bail!("illegal reponame {name:?}");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        bail!("illegal reponame {name:?}");
    }
    Ok(name.to_string())
}

pub fn is_blocked_source(path: &Path, explicit: bool) -> bool {
    is_forbidden_source(path, explicit)
}

pub fn is_forbidden_source(path: &Path, explicit: bool) -> bool {
    if explicit {
        return false;
    }
    if path == Path::new("/") {
        return true;
    }
    if let Ok(home) = std::env::var("HOME") {
        if path == Path::new(&home) {
            return true;
        }
    }
    false
}

/// Disambiguate if `name` already maps to a different source.
pub fn unique_reponame(cache_dir: &Path, source: &Path, base: &str) -> String {
    let marker = cache_dir.join(base).join("SOURCE");
    if let Ok(existing) = fs::read_to_string(&marker) {
        let existing = existing.trim();
        let want = source.to_string_lossy();
        if existing == want {
            return base.to_string();
        }
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        want.hash(&mut hasher);
        return format!("{base}-{:08x}", hasher.finish());
    }
    base.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_home_root_uses_home_env() {
        let home = std::env::var("HOME").expect("HOME for test");
        let root = default_home_root().unwrap();
        assert_eq!(root, PathBuf::from(home));
    }

    #[test]
    fn rgctl_dir_under_default_home() {
        let home = default_home_root().unwrap();
        let dh = DaemonHome::from_path(&home).unwrap();
        assert_eq!(dh.rgctl_dir(), home.join(".rgctl"));
    }
}

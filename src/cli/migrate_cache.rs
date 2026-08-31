//! `rgctl migrate-cache` — copy daemon-era cache artifacts into a repo tree.

use super::context::CliContext;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

pub struct MigrateCacheArgs {
    pub name: Option<String>,
    pub from: Option<PathBuf>,
    pub force: bool,
}

pub fn run(ctx: &CliContext, args: MigrateCacheArgs) -> Result<()> {
    let repo = ctx.repo.canonicalize().unwrap_or_else(|_| ctx.repo.clone());
    let dest = rgctl_graph::paths::artifact_dir(&repo);
    if dest.exists() && !args.force {
        bail!(
            "destination {} already exists (pass --force to overwrite)",
            dest.display()
        );
    }

    let source = match args.from {
        Some(p) => p,
        None => {
            let name = args
                .name
                .or_else(|| {
                    repo.file_name()
                        .and_then(|s| s.to_str())
                        .map(str::to_string)
                })
                .context("pass --name or use a repo path with a directory name")?;
            rgctl_graph::paths::daemon_cache_artifacts(&name)
                .with_context(|| format!("cannot resolve cache for {name:?} (set RGCTL_HOME or HOME)"))?
        }
    };

    if !source.is_dir() {
        bail!("cache source not found: {}", source.display());
    }

    if dest.exists() {
        fs::remove_dir_all(&dest)
            .with_context(|| format!("remove {}", dest.display()))?;
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    copy_dir_recursive(&source, &dest)?;
    eprintln!(
        "[rgctl] migrated cache {} → {}",
        source.display(),
        dest.display()
    );
    Ok(())
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = to.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

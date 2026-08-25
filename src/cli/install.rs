//! `rgctl install --skill` — copy the bundled agent skill into a repo.

use super::args::{OutputFormat, SkillHost};
use super::context::CliContext;
use super::install_output::{
    InstallJsonResponse, InstallWrite, InstallWriteHost, InstallWriteStatus, build_install_response,
};
use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};
use std::fs;
use std::path::{Path, PathBuf};

static SKILL_BUNDLE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills/rgctl");

const SKILL_DIR_NAME: &str = "rgctl";

pub struct InstallArgs {
    pub skill: bool,
    pub host: SkillHost,
    pub force: bool,
}

pub fn run(ctx: &CliContext, args: InstallArgs) -> Result<()> {
    if !args.skill {
        bail!("pass --skill to install the rgctl agent skill (see `rgctl install --help`)");
    }

    let repo = abs_path(&ctx.repo);
    let files = bundled_skill_files();
    if files.is_empty() {
        bail!("embedded rgctl skill bundle is empty");
    }

    let hosts = host_targets(args.host);
    let mut writes = Vec::new();
    for host in hosts {
        let dest_dir = skill_dir_for(&repo, *host);
        for (rel, contents) in &files {
            let dest = dest_dir.join(rel);
            let status = classify_write(&dest, contents, args.force)?;
            writes.push(InstallWrite {
                host: *host,
                path: dest.display().to_string(),
                status,
            });
        }
    }

    let blocked = writes
        .iter()
        .any(|w| w.status == InstallWriteStatus::SkippedExists);
    // All-or-nothing across hosts: do not write some files if any dest would be skipped.
    if !blocked {
        for write in &writes {
            if matches!(
                write.status,
                InstallWriteStatus::Created | InstallWriteStatus::Overwritten
            ) {
                let rel = dest_rel_for(write, &repo)?;
                let contents = file_contents_for(&rel)?;
                atomic_write(Path::new(&write.path), contents)?;
            }
        }
    }

    let response = build_install_response(&repo.display().to_string(), args.force, writes);
    emit_response(ctx, &response)?;

    if response
        .writes
        .iter()
        .any(|w| w.status == InstallWriteStatus::SkippedExists)
    {
        bail!("skill file exists and differs (pass --force to overwrite)");
    }
    Ok(())
}

fn host_targets(host: SkillHost) -> &'static [InstallWriteHost] {
    match host {
        SkillHost::All => &[InstallWriteHost::Claude, InstallWriteHost::Cursor],
        SkillHost::Claude => &[InstallWriteHost::Claude],
        SkillHost::Cursor => &[InstallWriteHost::Cursor],
    }
}

fn skill_dir_for(repo: &Path, host: InstallWriteHost) -> PathBuf {
    let agent_dir = match host {
        InstallWriteHost::Claude => ".claude",
        InstallWriteHost::Cursor => ".cursor",
    };
    repo.join(agent_dir).join("skills").join(SKILL_DIR_NAME)
}

fn bundled_skill_files() -> Vec<(PathBuf, &'static [u8])> {
    let mut out = Vec::new();
    collect_dir(&SKILL_BUNDLE, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn collect_dir(dir: &Dir<'static>, out: &mut Vec<(PathBuf, &'static [u8])>) {
    for file in dir.files() {
        out.push((file.path().to_path_buf(), file.contents()));
    }
    for sub in dir.dirs() {
        collect_dir(sub, out);
    }
}

fn file_contents_for(rel: &Path) -> Result<&'static [u8]> {
    SKILL_BUNDLE
        .get_file(rel)
        .map(|f| f.contents())
        .with_context(|| format!("missing bundled skill file {}", rel.display()))
}

fn dest_rel_for(write: &InstallWrite, repo: &Path) -> Result<PathBuf> {
    let dest = Path::new(&write.path);
    let dir = skill_dir_for(repo, write.host);
    dest.strip_prefix(&dir)
        .map(Path::to_path_buf)
        .with_context(|| format!("dest {} is not under {}", dest.display(), dir.display()))
}

fn classify_write(dest: &Path, contents: &[u8], force: bool) -> Result<InstallWriteStatus> {
    let meta = match fs::symlink_metadata(dest) {
        Ok(m) => m,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(InstallWriteStatus::Created);
        }
        Err(err) => {
            return Err(err).with_context(|| format!("stat {}", dest.display()));
        }
    };
    let is_symlink = meta.file_type().is_symlink();
    let existing = fs::read(dest).with_context(|| format!("read {}", dest.display()))?;
    if existing.as_slice() == contents {
        if is_symlink {
            return Ok(InstallWriteStatus::Overwritten);
        }
        return Ok(InstallWriteStatus::Unchanged);
    }
    if force {
        Ok(InstallWriteStatus::Overwritten)
    } else {
        Ok(InstallWriteStatus::SkippedExists)
    }
}

fn atomic_write(dest: &Path, contents: &[u8]) -> Result<()> {
    if let Ok(meta) = fs::symlink_metadata(dest) {
        if meta.file_type().is_symlink() {
            fs::remove_file(dest).with_context(|| format!("remove symlink {}", dest.display()))?;
        }
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let tmp_name = dest.file_name().map_or_else(
        || ".skill.rg-install.tmp".to_string(),
        |name| format!(".{}.rg-install.tmp", name.to_string_lossy()),
    );
    let tmp = dest.with_file_name(tmp_name);
    fs::write(&tmp, contents).with_context(|| format!("write {}", tmp.display()))?;
    if dest.exists() {
        let _ = fs::remove_file(dest);
    }
    fs::rename(&tmp, dest)
        .with_context(|| format!("rename {} -> {}", tmp.display(), dest.display()))?;
    Ok(())
}

fn abs_path(path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    fs::canonicalize(&joined).unwrap_or(joined)
}

fn emit_response(ctx: &CliContext, response: &InstallJsonResponse) -> Result<()> {
    if ctx.format == OutputFormat::Json {
        ctx.emit_json_value(&serde_json::to_value(response)?)?;
        return Ok(());
    }
    let mut lines = vec!["Installed rgctl skill:".to_string()];
    for write in &response.writes {
        lines.push(format!(
            "  {:<16} {}",
            status_label(write.status),
            write.path
        ));
    }
    ctx.emit(&lines.join("\n"))?;
    Ok(())
}

fn status_label(status: InstallWriteStatus) -> &'static str {
    match status {
        InstallWriteStatus::Created => "created",
        InstallWriteStatus::Unchanged => "unchanged",
        InstallWriteStatus::Overwritten => "overwritten",
        InstallWriteStatus::SkippedExists => "skipped_exists",
    }
}

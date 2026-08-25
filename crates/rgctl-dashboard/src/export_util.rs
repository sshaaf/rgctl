//! Shared dashboard export helpers.

use serde::Serialize;
use std::fs;
use std::path::Path;

pub fn write_json_compact(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(unix)]
pub fn link_or_copy(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if dst.exists() {
        fs::remove_file(dst).map_err(|e| e.to_string())?;
    }
    fs::hard_link(src, dst).or_else(|_| {
        fs::copy(src, dst).map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[cfg(not(unix))]
pub fn link_or_copy(src: &Path, dst: &Path) -> Result<(), String> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(src, dst).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn link_or_copy_makes_readable_dest() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");
        fs::write(&src, b"payload").unwrap();
        link_or_copy(&src, &dst).unwrap();
        assert_eq!(fs::read(&dst).unwrap(), b"payload");
    }
}

//! Extract embedded Vite build output into `.rgctl/dashboard/`.

use include_dir::{Dir, include_dir};
use std::fs;
use std::path::{Path, PathBuf};

static DASHBOARD_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../dashboard/dist");

/// Root directory name under `.rgctl/`.
pub const DASHBOARD_DIR_NAME: &str = "dashboard";

fn workspace_dist_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dashboard/dist")
}

/// True when a usable dashboard build exists (disk tree in dev, or full embed for release).
pub fn dist_embedded() -> bool {
    let disk = workspace_dist_dir();
    disk_dist_complete(&disk) || embedded_dist_complete(&DASHBOARD_DIST)
}

fn disk_dist_complete(dist: &Path) -> bool {
    dist.join("index.html").is_file() && dist.join("assets").is_dir()
}

fn embedded_dist_complete(dist: &Dir<'_>) -> bool {
    dist.get_file("index.html").is_some()
        && dist
            .get_dir("assets")
            .is_some_and(|assets| embedded_file_count(assets) > 0)
}

fn embedded_file_count(dir: &Dir<'_>) -> usize {
    dir.files().count() + dir.dirs().map(embedded_file_count).sum::<usize>()
}

/// Write all files from embedded `dashboard/dist` into `out_dir`.
pub fn extract_static_assets(out_dir: &Path) -> Result<(), String> {
    extract_static_assets_from(out_dir, &workspace_dist_dir(), &DASHBOARD_DIST)
}

fn extract_static_assets_from(
    out_dir: &Path,
    disk: &Path,
    embedded: &Dir<'_>,
) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    if disk_dist_complete(disk) {
        copy_dir_recursive(disk, out_dir)?;
        return Ok(());
    }

    if !embedded_dist_complete(embedded) {
        return Err(
            "dashboard/dist incomplete — run: ./scripts/build-dashboard.sh && cargo build --release"
                .into(),
        );
    }

    write_embedded_dir(out_dir, embedded)?;
    Ok(())
}

fn write_embedded_dir(out_dir: &Path, dir: &Dir<'_>) -> Result<(), String> {
    for file in dir.files() {
        write_embedded_file(out_dir, file.path(), file.contents())?;
    }
    for child in dir.dirs() {
        write_embedded_dir(out_dir, child)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            fs::create_dir_all(&to).map_err(|e| e.to_string())?;
            copy_dir_recursive(&from, &to)?;
        } else {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn write_embedded_file(out_dir: &Path, rel: &Path, contents: &[u8]) -> Result<(), String> {
    let dest = out_dir.join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&dest, contents).map_err(|e| e.to_string())
}

/// Inject manifest JSON into index.html for offline `file://` bootstrap.
pub fn inject_manifest_bootstrap(out_dir: &Path, manifest_json: &str) -> Result<(), String> {
    let index_path = out_dir.join("index.html");
    let html = fs::read_to_string(&index_path).map_err(|e| format!("read index.html: {e}"))?;
    const MARKER: &str = "</head>";
    let script = format!(
        r#"<script id="rgctl-manifest" type="application/json">{manifest_json}</script>"#
    );
    if html.contains("id=\"rgctl-manifest\"") {
        return Ok(());
    }
    let updated = html.replace(MARKER, &format!("{script}\n  {MARKER}"));
    if updated == html {
        return Err("index.html missing </head> — cannot inject manifest".into());
    }
    fs::write(&index_path, updated).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn default_dashboard_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".rgctl").join(DASHBOARD_DIR_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dist_has_index_and_assets_when_built() {
        if !dist_embedded() {
            eprintln!("skip: dashboard/dist not built");
            return;
        }
        let disk = workspace_dist_dir();
        assert!(disk.join("index.html").is_file());
        assert!(disk.join("assets").is_dir());
    }

    #[test]
    fn embedded_dist_has_no_double_nested_assets() {
        fn assert_paths(dir: &Dir<'_>) {
            for file in dir.files() {
                let p = file.path().to_string_lossy();
                assert!(
                    !p.contains("assets/assets/"),
                    "double-nested asset path in embed: {p}"
                );
            }
            for child in dir.dirs() {
                assert_paths(child);
            }
        }

        if embedded_file_count(&DASHBOARD_DIST) == 0 {
            return;
        }
        assert_paths(&DASHBOARD_DIST);
    }

    #[test]
    fn embedded_bundle_extracts_without_workspace_dist() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let missing_disk = tmp.path().join("missing-dashboard-dist");
        let out = tmp.path().join("extracted");

        extract_static_assets_from(&out, &missing_disk, &DASHBOARD_DIST)
            .expect("extract embedded dashboard");

        assert!(out.join("index.html").is_file());
        let assets = out.join("assets");
        assert!(assets.is_dir());
        assert!(
            fs::read_dir(assets)
                .expect("read extracted assets")
                .next()
                .is_some(),
            "embedded dashboard assets should be extracted recursively"
        );
    }

    /// Worker bundle must reference a `rgctl_wasm_bg-*.wasm` asset that exists in dist.
    #[test]
    fn dist_worker_wasm_asset_exists() {
        if !dist_embedded() {
            eprintln!("skip: dashboard/dist not built");
            return;
        }
        let assets = workspace_dist_dir().join("assets");
        let worker = fs::read_dir(&assets)
            .map_err(|e| e.to_string())
            .unwrap()
            .filter_map(Result::ok)
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("worker-")
                    && e.path().extension().is_some_and(|ext| ext == "js")
            })
            .map(|e| e.path())
            .expect("worker-*.js in dashboard/dist/assets");
        let worker_src = fs::read_to_string(&worker).expect("read worker bundle");
        let wasm_name = worker_src
            .split("rgctl_wasm_bg-")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .map(|hash| format!("rgctl_wasm_bg-{hash}"))
            .expect("worker references rgctl_wasm_bg-*.wasm");
        assert!(
            assets.join(&wasm_name).is_file(),
            "missing WASM asset {wasm_name} referenced by {}",
            worker
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        );
    }
}

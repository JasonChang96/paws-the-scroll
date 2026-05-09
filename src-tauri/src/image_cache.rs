//! On-disk PNG cache for cat sprites. Keyed by a stable hash of
//! (`cat_id`, mood, `independence_tier`, `accessory_set_hash`) so sprite
//! evolution can re-use previous combinations without re-paying the
//! `gpt-image-1` latency budget.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, Runtime};

const CACHE_DIR_NAME: &str = "cat-sprites";

pub fn cache_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    let base = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data directory for sprite cache")?;
    let dir = base.join(CACHE_DIR_NAME);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create sprite cache dir {}", dir.display()))?;
    Ok(dir)
}

pub fn make_key(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(16).fold(String::new(), |mut acc, byte| {
        use std::fmt::Write;
        let _ = write!(acc, "{byte:02x}");
        acc
    });
    hex
}

pub fn path_for_key<R: Runtime>(app: &AppHandle<R>, key: &str) -> Result<PathBuf> {
    Ok(cache_dir(app)?.join(format!("cat-{key}.png")))
}

pub fn read_cached(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

pub fn write_cached(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes)
        .with_context(|| format!("failed to write sprite cache at {}", path.display()))
}

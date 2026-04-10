pub mod diff;
pub mod git;
pub mod tar;
pub mod hardlink;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Serialize, Deserialize};

/// Layer metadata stored per-layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerMeta {
    pub created_at: u64,
    pub description: String,
    pub layer_number: u32,
    /// Human-readable stats
    pub stats: LayerStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayerStats {
    pub files_changed: u32,
    pub files_added: u32,
    pub files_deleted: u32,
    pub bytes_written: u64,
}

/// Common trait for all layer backends.
///
/// Each backend implements a different strategy for tracking and applying
/// rootfs changes. See `docs/zh/backend-comparison.md` for analysis.
pub trait LayerBackend: Send + Sync {
    /// Backend name (e.g., "diff", "git", "tar", "hardlink")
    fn name(&self) -> &str;

    /// Initialize layer tracking for a rootfs (snapshot base state).
    fn init(&self, rootfs: &Path, layers_dir: &Path) -> Result<()>;

    /// Commit current rootfs changes as a new layer.
    fn commit(&self, rootfs: &Path, layers_dir: &Path, description: &str) -> Result<LayerMeta>;

    /// List all committed layers.
    fn list(&self, layers_dir: &Path) -> Result<Vec<LayerMeta>>;

    /// Apply a specific layer on top of a rootfs.
    fn apply(&self, rootfs: &Path, layers_dir: &Path, layer_number: u32) -> Result<()>;

    /// Rebase: replace base and re-apply all layers.
    fn rebase(&self, rootfs: &Path, layers_dir: &Path, new_base: &Path) -> Result<()>;

    /// Get total size of all layers on disk.
    fn layer_disk_size(&self, layers_dir: &Path) -> Result<u64> {
        Ok(dir_size(layers_dir))
    }
}

/// File manifest: relative_path -> sha256
pub type Manifest = HashMap<String, String>;

/// Generate SHA256 manifest for all files in a directory.
pub fn generate_manifest(rootfs: &Path) -> Result<Manifest> {
    let mut manifest = HashMap::new();
    walk_and_hash(rootfs, rootfs, &mut manifest)?;
    Ok(manifest)
}

fn walk_and_hash(dir: &Path, rootfs: &Path, manifest: &mut Manifest) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.file_type()?;
        if meta.is_dir() {
            walk_and_hash(&path, rootfs, manifest)?;
        } else if meta.is_file() {
            let rel = path.strip_prefix(rootfs)?.to_string_lossy().to_string();
            let hash = sha256_file(&path)?;
            manifest.insert(format!("/{}", rel), hash);
        }
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Sha256, Digest};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Save manifest to file (sha256  /path)
pub fn save_manifest(manifest: &Manifest, path: &Path) -> Result<()> {
    let mut entries: Vec<_> = manifest.iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    let content: String = entries.iter()
        .map(|(k, v)| format!("{}  {}", v, k))
        .collect::<Vec<_>>()
        .join("\n") + "\n";
    std::fs::write(path, content)?;
    Ok(())
}

/// Load manifest from file
pub fn load_manifest(path: &Path) -> Result<Manifest> {
    let content = std::fs::read_to_string(path)?;
    let mut manifest = HashMap::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(2, "  ").collect();
        if parts.len() == 2 {
            manifest.insert(parts[1].to_string(), parts[0].to_string());
        }
    }
    Ok(manifest)
}

/// Compute diff between two manifests
pub fn diff_manifests(base: &Manifest, current: &Manifest) -> DiffResult {
    let mut changed = vec![];
    let mut added = vec![];
    let mut deleted = vec![];

    for (path, hash) in current {
        match base.get(path) {
            Some(h) if h != hash => changed.push(path.clone()),
            None => added.push(path.clone()),
            _ => {}
        }
    }
    for path in base.keys() {
        if !current.contains_key(path) {
            deleted.push(path.clone());
        }
    }

    DiffResult { changed, added, deleted }
}

pub struct DiffResult {
    pub changed: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
}

/// Get next layer number in a layers directory
pub fn next_layer_number(layers_dir: &Path) -> Result<u32> {
    let mut max_num = 0;
    if layers_dir.exists() {
        for entry in std::fs::read_dir(layers_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(num_str) = name.strip_prefix("layer-") {
                if let Ok(num) = num_str.parse::<u32>() {
                    max_num = max_num.max(num);
                }
            }
        }
    }
    Ok(max_num + 1)
}

/// Save layer metadata
pub fn save_layer_meta(meta: &LayerMeta, layer_dir: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(meta)?;
    std::fs::write(layer_dir.join("meta.json"), &json)?;
    Ok(())
}

/// Load layer metadata
pub fn load_layer_meta(layer_dir: &Path) -> Result<LayerMeta> {
    let json = std::fs::read_to_string(layer_dir.join("meta.json"))?;
    Ok(serde_json::from_str(&json)?)
}

/// Get now as unix timestamp
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Copy directory recursively
pub fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        if !dst.exists() { std::fs::create_dir_all(dst)?; }
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = dst.parent() {
            if !parent.exists() { std::fs::create_dir_all(parent)?; }
        }
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

/// Clear directory contents
pub fn clear_dir(dir: &Path) -> Result<()> {
    if !dir.exists() { return Ok(()); }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// Get directory total size
pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

/// List layer directories in order
pub fn list_layer_dirs(layers_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = vec![];
    if !layers_dir.exists() { return Ok(dirs); }
    for entry in std::fs::read_dir(layers_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("layer-") {
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    Ok(dirs)
}

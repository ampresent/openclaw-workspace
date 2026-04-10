//! Diff backend — file-level diff with SHA256 manifest.
//!
//! Original implementation. Each layer stores only changed/new files
//! and a deleted.txt for removed files.
//!
//! **Pros**: Simple, predictable, small layers (only changes)
//! **Cons**: No dedup across layers, file-level granularity only

use std::path::Path;
use anyhow::{Result, anyhow};
use log::info;

use super::*;

pub struct DiffBackend;

impl DiffBackend {
    pub fn new() -> Self { DiffBackend }
}

impl LayerBackend for DiffBackend {
    fn name(&self) -> &str { "diff" }

    fn init(&self, rootfs: &Path, layers_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(layers_dir)?;
        let manifest = generate_manifest(rootfs)?;
        save_manifest(&manifest, &layers_dir.join("base.sha256"))?;
        println!("[diff] Base manifest: {} files", manifest.len());
        Ok(())
    }

    fn commit(&self, rootfs: &Path, layers_dir: &Path, description: &str) -> Result<LayerMeta> {
        let base_path = layers_dir.join("base.sha256");
        if !base_path.exists() {
            return Err(anyhow!("[diff] No base manifest. Run init-layer first."));
        }
        let base = load_manifest(&base_path)?;
        let current = generate_manifest(rootfs)?;
        let diff = diff_manifests(&base, &current);

        let num = next_layer_number(layers_dir)?;
        let layer_dir = layers_dir.join(format!("layer-{:03}", num));
        let files_dir = layer_dir.join("files");
        std::fs::create_dir_all(&files_dir)?;

        let mut bytes_written: u64 = 0;

        // Copy changed and added files
        for path in diff.changed.iter().chain(diff.added.iter()) {
            let src = rootfs.join(path.trim_start_matches('/'));
            let dst = files_dir.join(path.trim_start_matches('/'));
            if let Some(p) = dst.parent() { std::fs::create_dir_all(p)?; }
            bytes_written += std::fs::copy(&src, &dst)? as u64;
        }

        // Record deletions
        if !diff.deleted.is_empty() {
            std::fs::write(
                layer_dir.join("deleted.txt"),
                diff.deleted.join("\n") + "\n",
            )?;
        }

        let meta = LayerMeta {
            created_at: now(),
            description: description.to_string(),
            layer_number: num,
            stats: LayerStats {
                files_changed: diff.changed.len() as u32,
                files_added: diff.added.len() as u32,
                files_deleted: diff.deleted.len() as u32,
                bytes_written,
            },
        };
        save_layer_meta(&meta, &layer_dir)?;

        // Update base manifest
        save_manifest(&current, &base_path)?;

        println!("[diff] Layer {:03}: {} changed, {} added, {} deleted, {} bytes",
            num, diff.changed.len(), diff.added.len(), diff.deleted.len(), bytes_written);
        Ok(meta)
    }

    fn list(&self, layers_dir: &Path) -> Result<Vec<LayerMeta>> {
        let mut layers = vec![];
        for dir in list_layer_dirs(layers_dir)? {
            if dir.join("meta.json").exists() {
                layers.push(load_layer_meta(&dir)?);
            }
        }
        Ok(layers)
    }

    fn apply(&self, rootfs: &Path, layers_dir: &Path, layer_number: u32) -> Result<()> {
        let layer_dir = layers_dir.join(format!("layer-{:03}", layer_number));
        let files_dir = layer_dir.join("files");
        if files_dir.exists() {
            copy_recursive(&files_dir, rootfs)?;
        }
        let deleted_path = layer_dir.join("deleted.txt");
        if deleted_path.exists() {
            for line in std::fs::read_to_string(&deleted_path)?.lines() {
                let p = rootfs.join(line.trim().trim_start_matches('/'));
                if p.exists() { std::fs::remove_file(&p)?; }
            }
        }
        Ok(())
    }

    fn rebase(&self, rootfs: &Path, layers_dir: &Path, new_base: &Path) -> Result<()> {
        let layers = list_layer_dirs(layers_dir)?;
        clear_dir(rootfs)?;
        copy_recursive(new_base, rootfs)?;
        self.init(rootfs, layers_dir)?;
        for dir in &layers {
            let meta = load_layer_meta(dir)?;
            println!("[diff] Replaying layer {:03}", meta.layer_number);
            self.apply(rootfs, layers_dir, meta.layer_number)?;
        }
        // Finalize base manifest
        let manifest = generate_manifest(rootfs)?;
        save_manifest(&manifest, &layers_dir.join("base.sha256"))?;
        Ok(())
    }
}

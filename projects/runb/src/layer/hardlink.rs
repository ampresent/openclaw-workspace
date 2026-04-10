//! Hardlink backend — space-efficient snapshot using hardlinks.
//!
//! Each layer is a full directory snapshot, but unchanged files are
//! hardlinked to the previous layer (zero additional disk space).
//! Only changed files take new space.
//!
//! **Pros**: Fast random access, no extraction needed, space-efficient via sharing
//! **Cons**: Hardlinks break on some filesystems, layer dirs look like full copies,
//!          cross-device issues, deletion tracking needs extra manifest

use std::path::Path;
use anyhow::{Result, anyhow};
use log::info;

use super::*;

pub struct HardlinkBackend;

impl HardlinkBackend {
    pub fn new() -> Self { HardlinkBackend }

    fn snapshot_dir(&self, src: &Path, dst: &Path, prev: Option<&Path>) -> Result<(u32, u32, u64)> {
        // Create dst, then for each file in src:
        // - If prev has same file with same hash → hardlink
        // - Otherwise → copy
        if !dst.exists() { std::fs::create_dir_all(dst)?; }

        let mut changed = 0u32;
        let mut added = 0u32;
        let mut bytes_written: u64 = 0;

        let src_manifest = generate_manifest(src)?;
        let prev_manifest = match prev {
            Some(p) => generate_manifest(p).unwrap_or_default(),
            None => std::collections::HashMap::new(),
        };

        for (path, hash) in &src_manifest {
            let rel = path.trim_start_matches('/');
            let src_file = src.join(rel);
            let dst_file = dst.join(rel);

            if let Some(parent) = dst_file.parent() {
                if !parent.exists() { std::fs::create_dir_all(parent)?; }
            }

            let is_same = prev_manifest.get(path) == Some(hash);
            let prev_file = prev.map(|p| p.join(rel));

            if is_same && prev_file.as_ref().map(|p| p.exists()).unwrap_or(false) {
                // Hardlink from previous snapshot
                let _ = std::fs::hard_link(prev_file.unwrap(), &dst_file);
                // If hardlink fails (cross-device etc), fall through to copy
                if dst_file.exists() { continue; }
            }

            // Copy the file
            bytes_written += std::fs::copy(&src_file, &dst_file)? as u64;
            if prev_manifest.contains_key(path) {
                changed += 1;
            } else {
                added += 1;
            }
        }

        // Create parent dirs for deleted file tracking
        // (We track deletions via manifest diff at apply time)

        Ok((changed, added, bytes_written))
    }
}

impl LayerBackend for HardlinkBackend {
    fn name(&self) -> &str { "hardlink" }

    fn init(&self, rootfs: &Path, layers_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(layers_dir)?;
        let manifest = generate_manifest(rootfs)?;
        save_manifest(&manifest, &layers_dir.join("base.sha256"))?;

        // Store base as a snapshot
        let base_dir = layers_dir.join("base");
        if base_dir.exists() { std::fs::remove_dir_all(&base_dir)?; }
        copy_recursive(rootfs, &base_dir)?;

        println!("[hardlink] Base snapshot created: {} files", manifest.len());
        Ok(())
    }

    fn commit(&self, rootfs: &Path, layers_dir: &Path, description: &str) -> Result<LayerMeta> {
        let base_path = layers_dir.join("base.sha256");
        if !base_path.exists() {
            return Err(anyhow!("[hardlink] No base manifest. Run init-layer first."));
        }

        let base = load_manifest(&base_path)?;
        let current = generate_manifest(rootfs)?;
        let diff = diff_manifests(&base, &current);

        let num = next_layer_number(layers_dir)?;
        let layer_dir = layers_dir.join(format!("layer-{:03}", num));

        // Previous snapshot for hardlinking
        let prev_snapshot = if num == 1 {
            Some(layers_dir.join("base"))
        } else {
            Some(layers_dir.join(format!("layer-{:03}", num - 1)).join("snapshot"))
        };

        let snapshot_dir = layer_dir.join("snapshot");
        let (changed, added, bytes_written) = self.snapshot_dir(
            rootfs,
            &snapshot_dir,
            prev_snapshot.as_deref(),
        )?;

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
                files_changed: changed,
                files_added: added,
                files_deleted: diff.deleted.len() as u32,
                bytes_written,
            },
        };
        save_layer_meta(&meta, &layer_dir)?;

        // Update base manifest
        save_manifest(&current, &base_path)?;

        println!("[hardlink] Layer {:03}: {} changed, {} added, {} deleted, {} new bytes (rest hardlinked)",
            num, changed, added, diff.deleted.len(), bytes_written);
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
        let snapshot = layer_dir.join("snapshot");

        if snapshot.exists() {
            // Copy snapshot into rootfs (overwrites existing)
            copy_recursive(&snapshot, rootfs)?;
        }

        // Handle deletions
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
            println!("[hardlink] Replaying layer {:03}", meta.layer_number);
            self.apply(rootfs, layers_dir, meta.layer_number)?;
        }
        let manifest = generate_manifest(rootfs)?;
        save_manifest(&manifest, &layers_dir.join("base.sha256"))?;
        Ok(())
    }

    fn layer_disk_size(&self, layers_dir: &Path) -> Result<u64> {
        // Actual disk usage (accounting for hardlinks) is hard to measure.
        // Report apparent size.
        Ok(dir_size(layers_dir))
    }
}

//! Tar backend — each layer is a tar archive.
//!
//! Similar to Docker's layer format. Each layer is a tar.gz containing
//! only the changed files. Manifest tracks file checksums.
//!
//! **Pros**: Portable, good compression, Docker-compatible format, single file per layer
//! **Cons**: Need to extract on apply, slower random access, no in-place updates

use std::path::Path;
use anyhow::{Result, anyhow};
use log::info;

use super::*;

pub struct TarBackend;

impl TarBackend {
    pub fn new() -> Self { TarBackend }
}

impl LayerBackend for TarBackend {
    fn name(&self) -> &str { "tar" }

    fn init(&self, rootfs: &Path, layers_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(layers_dir)?;
        let manifest = generate_manifest(rootfs)?;
        save_manifest(&manifest, &layers_dir.join("base.sha256"))?;

        // Also save base as a reference tar (for deletion tracking)
        let output = std::process::Command::new("tar")
            .args(["-cf", layers_dir.join("base.tar").to_str().unwrap(),
                   "-C", rootfs.to_str().unwrap(), "."])
            .output()?;
        if !output.status.success() {
            return Err(anyhow!("[tar] Failed to create base tar"));
        }

        println!("[tar] Base manifest: {} files (+ base.tar)", manifest.len());
        Ok(())
    }

    fn commit(&self, rootfs: &Path, layers_dir: &Path, description: &str) -> Result<LayerMeta> {
        let base_path = layers_dir.join("base.sha256");
        if !base_path.exists() {
            return Err(anyhow!("[tar] No base manifest. Run init-layer first."));
        }
        let base = load_manifest(&base_path)?;
        let current = generate_manifest(rootfs)?;
        let diff = diff_manifests(&base, &current);

        let num = next_layer_number(layers_dir)?;
        let layer_dir = layers_dir.join(format!("layer-{:03}", num));
        std::fs::create_dir_all(&layer_dir)?;

        let mut bytes_written: u64 = 0;

        // Collect changed/added files
        let mut files_to_tar: Vec<String> = vec![];
        for path in diff.changed.iter().chain(diff.added.iter()) {
            let rel = path.trim_start_matches('/');
            files_to_tar.push(rel.to_string());
        }

        if !files_to_tar.is_empty() {
            // Create tar of changed files
            let tar_path = layer_dir.join("files.tar.gz");
            let file_list_path = layer_dir.join("files.txt");
            std::fs::write(&file_list_path, files_to_tar.join("\n") + "\n")?;

            let output = std::process::Command::new("tar")
                .args(["-czf", tar_path.to_str().unwrap(),
                       "-C", rootfs.to_str().unwrap(),
                       "-T", file_list_path.to_str().unwrap()])
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("[tar] tar creation failed: {}", stderr));
            }

            bytes_written = std::fs::metadata(&tar_path)?.len();
            std::fs::remove_file(&file_list_path)?;
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

        println!("[tar] Layer {:03}: {} changed, {} added, {} deleted, {} compressed bytes",
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
        let tar_path = layer_dir.join("files.tar.gz");

        if tar_path.exists() {
            let output = std::process::Command::new("tar")
                .args(["-xzf", tar_path.to_str().unwrap(),
                       "-C", rootfs.to_str().unwrap()])
                .output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!("[tar] extract failed: {}", stderr));
            }
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
            println!("[tar] Replaying layer {:03}", meta.layer_number);
            self.apply(rootfs, layers_dir, meta.layer_number)?;
        }
        let manifest = generate_manifest(rootfs)?;
        save_manifest(&manifest, &layers_dir.join("base.sha256"))?;
        Ok(())
    }
}

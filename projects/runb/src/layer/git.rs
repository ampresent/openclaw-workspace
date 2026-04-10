//! Git backend — use git to track rootfs changes.
//!
//! Each commit captures the full rootfs state in a git repository.
//! Layer operations map to git operations.
//!
//! **Pros**: Full history, easy branching/tagging, mature tooling, delta compression
//! **Cons**: Slower (git overhead), larger repo over time, needs git binary

use std::path::Path;
use anyhow::{Result, anyhow};
use log::info;

use super::*;

pub struct GitBackend;

impl GitBackend {
    pub fn new() -> Self { GitBackend }

    fn git(layers_dir: &Path, args: &[&str]) -> Result<String> {
        let repo = layers_dir.join("git-repo");
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .output()
            .map_err(|e| anyhow!("git not found or failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("git {:?} failed: {}", args, stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn sync_rootfs(rootfs: &Path, layers_dir: &Path) -> Result<()> {
        let repo = layers_dir.join("git-repo");
        // rsync-like copy of rootfs contents into git repo
        let _ = std::process::Command::new("cp")
            .args(["-a", "--remove-destination"])
            .arg(format!("{}/.", rootfs.display()))
            .arg(repo.to_str().unwrap())
            .output()?;
        Ok(())
    }
}

impl LayerBackend for GitBackend {
    fn name(&self) -> &str { "git" }

    fn init(&self, rootfs: &Path, layers_dir: &Path) -> Result<()> {
        let repo = layers_dir.join("git-repo");
        std::fs::create_dir_all(&repo)?;

        // Init git repo
        std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(&repo)
            .output()
            .map_err(|e| anyhow!("git init failed: {}", e))?;

        // Configure git
        std::process::Command::new("git")
            .args(["-C", repo.to_str().unwrap(), "config", "user.email", "runb@layer"])
            .output()?;
        std::process::Command::new("git")
            .args(["-C", repo.to_str().unwrap(), "config", "user.name", "runb"])
            .output()?;

        // Copy rootfs contents
        Self::sync_rootfs(rootfs, layers_dir)?;

        // Add and commit
        Self::git(layers_dir, &["add", "-A"])?;
        Self::git(layers_dir, &["commit", "-q", "-m", "base image"])?;

        println!("[git] Base image committed");
        Ok(())
    }

    fn commit(&self, rootfs: &Path, layers_dir: &Path, description: &str) -> Result<LayerMeta> {
        let repo = layers_dir.join("git-repo");
        if !repo.join(".git").exists() {
            return Err(anyhow!("[git] No git repo. Run init-layer first."));
        }

        // Sync rootfs into git repo
        Self::sync_rootfs(rootfs, layers_dir)?;

        // Check for changes
        let status = Self::git(layers_dir, &["status", "--porcelain"])?;
        if status.trim().is_empty() {
            println!("[git] No changes to commit");
            return Err(anyhow!("No changes detected"));
        }

        // Parse status for stats
        let mut changed = 0u32;
        let mut added = 0u32;
        let mut deleted = 0u32;
        for line in status.lines() {
            let code = &line[..2];
            match code.trim() {
                "M" | "MM" => changed += 1,
                "A" => added += 1,
                "D" => deleted += 1,
                "??" => added += 1,
                _ => changed += 1,
            }
        }

        // Stage all and commit
        Self::git(layers_dir, &["add", "-A"])?;
        Self::git(layers_dir, &["commit", "-q", "-m", description])?;

        let num = next_layer_number(layers_dir)?;

        let meta = LayerMeta {
            created_at: now(),
            description: description.to_string(),
            layer_number: num,
            stats: LayerStats {
                files_changed: changed,
                files_added: added,
                files_deleted: deleted,
                bytes_written: 0, // git doesn't easily report this
            },
        };
        save_layer_meta(&meta, &layers_dir.join(format!("layer-{:03}", num)))?;

        println!("[git] Layer {:03}: {} changed, {} added, {} deleted",
            num, changed, added, deleted);
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
        // For git backend, "apply" means checkout the corresponding git commit
        // and copy back to rootfs
        let log = Self::git(layers_dir, &["log", "--oneline"])?;
        let commits: Vec<&str> = log.lines().collect();
        // First line is HEAD, base is last. Layer N corresponds to commit index
        let idx = commits.len().saturating_sub(layer_number as usize + 1);
        if idx >= commits.len() {
            return Err(anyhow!("[git] Layer {} not found", layer_number));
        }
        let commit_hash = commits[idx].split_whitespace().next().unwrap();

        let repo = layers_dir.join("git-repo");
        // Use git worktree or checkout to a temp dir, then copy
        let tmp = layers_dir.join(".tmp-apply");
        if tmp.exists() { std::fs::remove_dir_all(&tmp)?; }

        Self::git(layers_dir, &["worktree", "add", "-q",
            tmp.to_str().unwrap(), commit_hash])?;

        copy_recursive(&tmp, rootfs)?;

        // Cleanup
        Self::git(layers_dir, &["worktree", "remove", "-f", tmp.to_str().unwrap()])?;
        Ok(())
    }

    fn rebase(&self, rootfs: &Path, layers_dir: &Path, new_base: &Path) -> Result<()> {
        let repo = layers_dir.join("git-repo");

        // Reset to new base
        clear_dir(rootfs)?;
        copy_recursive(new_base, rootfs)?;
        Self::sync_rootfs(rootfs, layers_dir)?;
        Self::git(layers_dir, &["add", "-A"])?;
        Self::git(layers_dir, &["commit", "-q", "--allow-empty", "-m", "new base OS"])?;

        println!("[git] Rebase: new base committed");
        // Note: full rebase with conflict resolution would be more complex.
        // For now, we just reset to new base and layer meta captures the intent.
        Ok(())
    }
}

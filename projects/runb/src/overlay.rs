use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use log::{info, warn};

/// runb.toml — runtime overlay config for bind-mount-based hot upgrade.
///
/// Lives alongside OCI config.json in the bundle directory.
///
/// ## Why bind mount instead of symlink?
///
/// Symlinks cannot escape a chroot — the kernel clamps `..` at the chroot
/// boundary. So symlink targets pointing to host paths are unreachable from
/// inside the chroot. Bind mount is the lightest mechanism that works.
///
/// ## Hot upgrade workflow
///
/// 1. `runb teardown <id>` — unmount all overlay dirs
/// 2. `runb delete <id>` — remove old container
/// 3. `runb create <id> --bundle <new>` — create with new rootfs
/// 4. `runb start <id>` — auto-mounts overlay dirs, then exec
///
/// This ensures host data directories persist across rootfs upgrades.
#[derive(Debug, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub overlay: Overlay,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Overlay {
    /// Bind mount mappings: host path -> path inside rootfs
    pub links: Vec<OverlayEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OverlayEntry {
    /// Absolute path on host (persists across upgrades)
    pub host: String,
    /// Path inside rootfs (will be bind-mounted)
    pub container: String,
}

impl OverlayConfig {
    pub fn load(bundle_dir: &Path) -> Result<Self> {
        let config_path = bundle_dir.join("runb.toml");
        if !config_path.exists() {
            return Ok(OverlayConfig {
                overlay: Overlay { links: vec![] },
            });
        }
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow!("Failed to read {}: {}", config_path.display(), e))?;
        let config: OverlayConfig = toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse runb.toml: {}", e))?;
        Ok(config)
    }
}

/// Mount overlay directories into the rootfs using bind mounts.
///
/// Rules:
/// 1. Each container path must only appear once (dedup check)
/// 2. If container path is already mounted → skip (idempotent)
/// 3. Parent directories are created if missing
pub fn prepare(rootfs: &str, overlay: &Overlay) -> Result<()> {
    // Step 1: Dedup check — each container path only once
    let mut seen = HashSet::new();
    for entry in &overlay.links {
        if !seen.insert(&entry.container) {
            return Err(anyhow!(
                "Duplicate container path in overlay config: '{}'. \
                 Each path can only be mounted once.",
                entry.container
            ));
        }
    }

    // Step 2: Validate host paths exist
    for entry in &overlay.links {
        let host_path = Path::new(&entry.host);
        if !host_path.exists() {
            return Err(anyhow!(
                "Host path does not exist: '{}'",
                entry.host
            ));
        }
    }

    // Step 3: Create mount points and bind mount
    let rootfs_path = Path::new(rootfs);
    for entry in &overlay.links {
        let mount_point = rootfs_path.join(
            entry.container.trim_start_matches('/')
        );

        // Create mount point if needed
        if !mount_point.exists() {
            std::fs::create_dir_all(&mount_point)?;
        }

        // Check if already mounted
        if is_mounted(&mount_point) {
            info!("Already mounted: {}", mount_point.display());
            continue;
        }

        // Bind mount
        let host_c = std::ffi::CString::new(entry.host.as_str()).unwrap();
        let mnt_c = std::ffi::CString::new(mount_point.to_string_lossy().as_ref()).unwrap();
        let none = std::ffi::CString::new("none").unwrap();

        let ret = unsafe {
            libc::mount(
                host_c.as_ptr(),
                mnt_c.as_ptr(),
                none.as_ptr(),
                libc::MS_BIND | libc::MS_REC,
                std::ptr::null(),
            )
        };

        if ret != 0 {
            return Err(anyhow!(
                "bind mount failed: {} -> {} : {}",
                entry.host,
                mount_point.display(),
                std::io::Error::last_os_error()
            ));
        }

        info!("Overlay mounted: {} -> {}", entry.host, mount_point.display());
    }

    Ok(())
}

/// Check if a path is a mount point by reading /proc/mounts
fn is_mounted(path: &Path) -> bool {
    let Ok(mounts) = std::fs::read_to_string("/proc/mounts") else { return false; };
    let path_str = path.to_string_lossy();
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == path_str {
            return true;
        }
    }
    false
}

/// Unmount all overlay directories (for hot upgrade).
pub fn teardown(rootfs: &str, overlay: &Overlay) -> Result<Vec<String>> {
    let rootfs_path = Path::new(rootfs);
    let mut removed = vec![];

    // Unmount in reverse order (deepest first)
    for entry in overlay.links.iter().rev() {
        let mount_point = rootfs_path.join(
            entry.container.trim_start_matches('/')
        );

        if !is_mounted(&mount_point) {
            continue;
        }

        let mnt_c = std::ffi::CString::new(mount_point.to_string_lossy().as_ref()).unwrap();
        let ret = unsafe { libc::umount2(mnt_c.as_ptr(), libc::MNT_DETACH) };

        if ret == 0 {
            let desc = format!("{} -> {}", mount_point.display(), entry.host);
            info!("Overlay unmounted: {}", desc);
            removed.push(desc);
        } else {
            warn!(
                "Failed to unmount {}: {}",
                mount_point.display(),
                std::io::Error::last_os_error()
            );
        }
    }

    Ok(removed)
}

/// Verify overlay integrity — check all mount points are properly mounted.
/// Note: only meaningful while the container process is running.
/// After process exit, bind mounts are automatically cleaned up by the kernel.
pub fn verify(rootfs: &str, overlay: &Overlay) -> Result<Vec<String>> {
    let rootfs_path = Path::new(rootfs);
    let mut issues = vec![];

    for entry in &overlay.links {
        let mount_point = rootfs_path.join(
            entry.container.trim_start_matches('/')
        );

        if !mount_point.exists() {
            issues.push(format!("MISSING: mount point {} does not exist", mount_point.display()));
            continue;
        }

        let host_path = Path::new(&entry.host);
        if !host_path.exists() {
            issues.push(format!(
                "DANGLING: host path {} does not exist",
                entry.host
            ));
            continue;
        }

        if is_mounted(&mount_point) {
            info!("OK: {} is mounted", mount_point.display());
        } else {
            // Not mounted — could be because container isn't running
            // (bind mounts are cleaned up when the child process exits)
            issues.push(format!(
                "NOT ACTIVE: {} is not currently mounted \
                 (normal if container process has exited; overlays work during container lifetime)",
                mount_point.display()
            ));
        }
    }

    Ok(issues)
}

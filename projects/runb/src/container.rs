use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow, Context};
use log::{info, debug};
use serde::{Serialize, Deserialize};

use crate::error::ContainerError;
use crate::spec;

// Runtime state root
const RUNB_ROOT: &str = "/run/runb";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerState {
    Created,
    Running,
    Stopped,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerMetadata {
    pub id: String,
    pub bundle: String,
    pub rootfs: String,
    pub pid: Option<i32>,
    pub state: ContainerState,
    pub created_at: u64,
}

pub fn runb_root() -> PathBuf {
    PathBuf::from(RUNB_ROOT)
}

pub fn container_dir(id: &str) -> PathBuf {
    runb_root().join(id)
}

fn state_file(id: &str) -> PathBuf {
    container_dir(id).join("state.json")
}

fn pid_file(id: &str) -> PathBuf {
    container_dir(id).join("pid")
}

/// Check if a container exists
pub fn exists(id: &str) -> bool {
    state_file(id).exists()
}

/// Load container metadata
pub fn load_metadata(id: &str) -> Result<ContainerMetadata> {
    let path = state_file(id);
    let content = fs::read_to_string(&path)
        .map_err(|_| ContainerError::NotFound(id.to_string()))?;
    let meta: ContainerMetadata = serde_json::from_str(&content)?;
    Ok(meta)
}

/// Save container metadata
fn save_metadata(meta: &ContainerMetadata) -> Result<()> {
    let path = state_file(&meta.id);
    let content = serde_json::to_string_pretty(meta)?;
    fs::write(&path, content)?;
    Ok(())
}

/// Create a container: set up state dir, validate bundle, record metadata.
/// The process is NOT started yet — that happens on `start`.
pub fn create(id: &str, bundle: &PathBuf) -> Result<()> {
    if exists(id) {
        return Err(ContainerError::AlreadyExists(id.to_string()).into());
    }

    // Validate bundle exists
    if !bundle.join("config.json").exists() {
        return Err(anyhow!("Bundle missing config.json: {}", bundle.display()));
    }

    let spec = spec::load_spec(bundle)?;
    let rootfs_path = if spec.root.path.starts_with('/') {
        spec.root.path.clone()
    } else {
        bundle.join(&spec.root.path).to_string_lossy().to_string()
    };

    // Validate rootfs exists
    if !Path::new(&rootfs_path).exists() {
        return Err(anyhow!("Rootfs does not exist: {}", rootfs_path));
    }

    // Create state directory
    let dir = container_dir(id);
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create container dir: {}", dir.display()))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let meta = ContainerMetadata {
        id: id.to_string(),
        bundle: bundle.canonicalize()?.to_string_lossy().to_string(),
        rootfs: rootfs_path,
        pid: None,
        state: ContainerState::Created,
        created_at: now,
    };

    save_metadata(&meta)?;
    info!("Container created: {}", id);
    Ok(())
}

/// Start a container: chroot into rootfs and exec the process.
/// In chroot-only mode (no namespaces), the child process runs as
/// a regular process inside the chroot jail.
pub fn start(id: &str) -> Result<()> {
    let mut meta = load_metadata(id)?;

    if meta.state != ContainerState::Created {
        return Err(ContainerError::InvalidState(format!(
            "Cannot start container '{}' in state {:?}",
            id, meta.state
        )).into());
    }

    let bundle = PathBuf::from(&meta.bundle);
    let spec = spec::load_spec(&bundle)?;

    info!("Starting container: {} (rootfs: {})", id, meta.rootfs);
    debug!("Process args: {:?}", spec.process.args);
    debug!("Process env: {:?}", spec.process.env);

    // Fork: parent records PID, child does chroot + exec
    unsafe {
        let pid = libc::fork();
        if pid < 0 {
            return Err(anyhow!("fork failed: {}", std::io::Error::last_os_error()));
        }

        if pid == 0 {
            // === CHILD PROCESS ===
            setup_chroot_and_exec(&meta.rootfs, &spec.process.cwd, &spec.process.args, &spec.process.env);
            // If we reach here, exec failed
            std::process::exit(127);
        } else {
            // === PARENT PROCESS ===
            meta.pid = Some(pid);
            meta.state = ContainerState::Running;
            save_metadata(&meta)?;

            // Write PID file for easy access
            fs::write(pid_file(id), pid.to_string())?;

            info!("Container started: {} (pid {})", id, pid);
        }
    }

    Ok(())
}

/// Child: chroot into rootfs, set cwd, env, and exec.
fn setup_chroot_and_exec(rootfs: &str, cwd: &str, args: &[String], env: &[String]) {
    // 1. chroot
    let rootfs_c = std::ffi::CString::new(rootfs).unwrap();
    let ret = unsafe { libc::chroot(rootfs_c.as_ptr()) };
    if ret != 0 {
        eprintln!("chroot failed: {}", std::io::Error::last_os_error());
        std::process::exit(126);
    }

    // 3. chdir
    let cwd_c = std::ffi::CString::new(cwd).unwrap();
    unsafe { libc::chdir(cwd_c.as_ptr()) };

    // 4. Clean environment and set new vars
    // Clear inherited environment first — use clearenv via libc
    unsafe {
        libc::clearenv();
    }
    for env_var in env {
        let parts: Vec<&str> = env_var.splitn(2, '=').collect();
        if parts.len() == 2 {
            let key = std::ffi::CString::new(parts[0]).unwrap();
            let val = std::ffi::CString::new(parts[1]).unwrap();
            unsafe { libc::setenv(key.as_ptr(), val.as_ptr(), 1) };
        }
    }

    // 5. exec
    let c_args: Vec<std::ffi::CString> = args.iter()
        .map(|a| std::ffi::CString::new(a.as_str()).unwrap())
        .collect();
    let c_ptrs: Vec<*const libc::c_char> = c_args.iter()
        .map(|a| a.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect();

    unsafe { libc::execvp(c_ptrs[0], c_ptrs.as_ptr()) };

    // If exec fails
    eprintln!("execvp failed: {}", std::io::Error::last_os_error());
    std::process::exit(127);
}

/// Stop a container by sending SIGTERM to its PID
pub fn stop(id: &str, signal: Option<i32>) -> Result<()> {
    let mut meta = load_metadata(id)?;

    if meta.state != ContainerState::Running {
        return Err(ContainerError::InvalidState(format!(
            "Cannot stop container '{}' in state {:?}",
            id, meta.state
        )).into());
    }

    let pid = meta.pid.ok_or_else(|| anyhow!("No PID recorded"))?;
    let sig = signal.unwrap_or(libc::SIGTERM);

    unsafe {
        let ret = libc::kill(pid, sig);
        if ret != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return Err(anyhow!("kill failed: {}", std::io::Error::last_os_error()));
        }
        // Reap the child
        libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG);
    }

    meta.state = ContainerState::Stopped;
    meta.pid = None;
    save_metadata(&meta)?;

    info!("Container stopped: {} (sent signal {} to pid {})", id, sig, pid);
    Ok(())
}

/// Delete a container: clean up state directory
pub fn delete(id: &str) -> Result<()> {
    // Use state() to auto-detect if process has exited
    let meta = state(id)?;

    if meta.state == ContainerState::Running {
        return Err(ContainerError::InvalidState(format!(
            "Cannot delete running container '{}'",
            id
        )).into());
    }

    let dir = container_dir(id);
    fs::remove_dir_all(&dir)
        .with_context(|| format!("Failed to remove container dir: {}", dir.display()))?;

    info!("Container deleted: {}", id);
    Ok(())
}

/// Check if a process is still alive
fn is_process_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Query container state (OCI `state` command)
/// Auto-detects if process has exited and updates state.
pub fn state(id: &str) -> Result<ContainerMetadata> {
    let mut meta = load_metadata(id)?;

    // If running, check if process is still alive
    if meta.state == ContainerState::Running {
        if let Some(pid) = meta.pid {
            if !is_process_alive(pid) {
                // Process has exited, reap it and update state
                unsafe {
                    libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG);
                }
                meta.state = ContainerState::Stopped;
                meta.pid = None;
                save_metadata(&meta)?;
            }
        }
    }

    Ok(meta)
}

/// List all containers
pub fn list() -> Result<Vec<ContainerMetadata>> {
    let root = runb_root();
    if !root.exists() {
        return Ok(vec![]);
    }

    let mut containers = vec![];
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let id = entry.file_name().to_string_lossy().to_string();
            // Use state() to auto-detect if process has exited
            if let Ok(meta) = state(&id) {
                containers.push(meta);
            }
        }
    }
    Ok(containers)
}

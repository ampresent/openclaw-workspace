use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// List patch files in a package's patch directory, sorted.
pub fn list_patches(patches_dir: &Path) -> Result<Vec<PathBuf>> {
    if !patches_dir.exists() {
        return Ok(vec![]);
    }
    let mut patches: Vec<PathBuf> = std::fs::read_dir(patches_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |e| e == "patch"))
        .collect();
    patches.sort();
    Ok(patches)
}

/// Discover all initialized package names under src/.
pub fn discover_packages(root: &Path) -> Result<Vec<String>> {
    let src_dir = root.join("src");
    if !src_dir.exists() {
        return Ok(vec![]);
    }
    let mut packages: Vec<String> = std::fs::read_dir(&src_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();
    packages.sort();
    Ok(packages)
}

/// Run a git command in a directory, return stdout as string.
pub fn git_output(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("git {:?} failed in {}", args, dir.display()))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Run a git command, check success.
pub fn git_ok(dir: &Path, args: &[&str]) -> Result<bool> {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()?;
    Ok(status.success())
}

/// Copy directory contents, skipping .git.
pub fn copy_tree(src: &Path, dest: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let src_path = entry.path();
        let dest_path = dest.join(&name);
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Apply patches in order, return (applied_count, failed_patches).
pub fn apply_patches(src: &Path, patches: &[PathBuf]) -> Result<(usize, Vec<String>)> {
    let mut applied = 0;
    let mut failed = Vec::new();

    for patch in patches {
        let name = patch.file_name().unwrap().to_string_lossy().to_string();
        let result = Command::new("git")
            .args(["apply", "--whitespace=nowarn"])
            .arg(patch)
            .current_dir(src)
            .output()?;

        if result.status.success() {
            applied += 1;
        } else {
            failed.push(name);
        }
    }

    Ok((applied, failed))
}

/// Count files recursively.
pub fn count_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                count += 1;
            } else if entry.path().is_dir() && entry.file_name() != ".git" {
                count += count_files(&entry.path());
            }
        }
    }
    count
}

/// Check if a package has uncommitted changes.
pub fn has_changes(src: &Path) -> bool {
    if !src.join(".git").exists() {
        return true;
    }
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(src)
        .output();
    match out {
        Ok(o) => !o.stdout.is_empty(),
        Err(_) => true,
    }
}

/// Get next patch number in the stack.
pub fn next_patch_number(root: &Path, package: &str) -> Result<u32> {
    let patches_dir = root.join("patches").join(package);
    let patches = list_patches(&patches_dir)?;
    if patches.is_empty() {
        return Ok(1);
    }
    let last = patches.last().unwrap();
    let name = last.file_stem().unwrap().to_string_lossy();
    let num_str = name.split('-').next().unwrap_or("0");
    Ok(num_str.parse::<u32>().unwrap_or(0) + 1)
}

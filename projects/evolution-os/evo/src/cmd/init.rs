use anyhow::{bail, Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Args)]
pub struct InitArgs {
    /// Package name to initialize
    package: String,

    /// Rocky Linux version to source from
    #[arg(long, default_value = "9")]
    rocky_version: String,

    /// Skip download, use existing src.rpm at this path
    #[arg(long)]
    srpm: Option<String>,

    /// Architecture for download (default: src)
    #[arg(long, default_value = "src")]
    arch: String,
}

pub fn run(args: InitArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;
    let pkg_dir = root.join("src").join(&args.package);
    let patches_dir = root.join("patches").join(&args.package);
    let specs_dir = root.join("specs");

    // Check if already initialized
    if pkg_dir.exists() {
        bail!(
            "package '{}' already initialized at {}. Use `evo rebase {}` to update.",
            args.package,
            pkg_dir.display(),
            args.package
        );
    }

    // Create directories
    std::fs::create_dir_all(&pkg_dir)
        .with_context(|| format!("failed to create {}", pkg_dir.display()))?;
    std::fs::create_dir_all(&patches_dir)
        .with_context(|| format!("failed to create {}", patches_dir.display()))?;
    std::fs::create_dir_all(&specs_dir)?;

    // Step 1: Get the src.rpm
    let srpm_path = if let Some(ref srpm) = args.srpm {
        PathBuf::from(srpm)
    } else {
        println!("{} downloading src.rpm for {}...", "→".dimmed(), args.package);
        download_srpm(&args.package, &args.rocky_version)?
    };

    if !srpm_path.exists() {
        bail!("src.rpm not found at {}", srpm_path.display());
    }

    // Step 2: Extract src.rpm to temp rpmbuild dir
    let tmp_build = root.join(".tmp-rpmbuild");
    if tmp_build.exists() {
        std::fs::remove_dir_all(&tmp_build)?;
    }

    println!("{} extracting src.rpm...", "→".dimmed());
    extract_srpm(&srpm_path, &tmp_build)?;

    // Step 3: Copy sources to src/<package>/
    let sources_dir = tmp_build.join("SOURCES");
    let specs_tmp = tmp_build.join("SPECS");

    if sources_dir.exists() {
        copy_dir_contents(&sources_dir, &pkg_dir)?;
    }

    // Step 4: Copy spec file
    if specs_tmp.exists() {
        for entry in std::fs::read_dir(&specs_tmp)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "spec") {
                let dest = specs_dir.join(format!("{}.spec", args.package));
                std::fs::copy(&path, &dest)?;
                println!("{} spec: {}", "✓".green(), dest.display());
            }
        }
    }

    // Step 5: Initialize git in src/<package> if not already
    if !pkg_dir.join(".git").exists() {
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(&pkg_dir)
            .output()
            .context("failed to init git in package dir")?;

        Command::new("git")
            .args(["add", "-A"])
            .current_dir(&pkg_dir)
            .output()?;

        Command::new("git")
            .args(["commit", "-q", "-m", "initial: upstream source from src.rpm"])
            .current_dir(&pkg_dir)
            .output()?;
    }

    // Cleanup
    std::fs::remove_dir_all(&tmp_build).ok();

    // Summary
    let file_count = count_files(&pkg_dir);
    println!();
    println!("{} package {} initialized", "✓".green().bold(), args.package);
    println!("  source:  {}/ ({} files)", pkg_dir.display(), file_count);
    println!("  patches: {}/", patches_dir.display());
    println!("  spec:    {}/{}.spec", specs_dir.display(), args.package);

    Ok(())
}

/// Download src.rpm using dnf
fn download_srpm(package: &str, version: &str) -> Result<PathBuf> {
    let output = Command::new("dnf")
        .args([
            "download",
            "--source",
            "--releasever",
            version,
            package,
        ])
        .output()
        .context("failed to run dnf download. Is dnf installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("dnf download failed:\n{}", stderr);
    }

    // dnf downloads to current directory, find the .src.rpm
    let cwd = std::env::current_dir()?;
    for entry in std::fs::read_dir(&cwd)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".src.rpm") {
            return Ok(entry.path());
        }
    }

    bail!("src.rpm not found after download for {}", package)
}

/// Extract src.rpm using rpm2cpio + cpio
fn extract_srpm(srpm: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;

    // rpm2cpio file.src.rpm | cpio -idm
    let rpm2cpio = Command::new("rpm2cpio")
        .arg(srpm)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .context("failed to run rpm2cpio. Is rpm installed?")?;

    let cpio = Command::new("cpio")
        .args(["-idm", "--quiet"])
        .current_dir(dest)
        .stdin(rpm2cpio.stdout.unwrap())
        .output()
        .context("failed to run cpio")?;

    if !cpio.status.success() {
        bail!("cpio extraction failed");
    }

    Ok(())
}

/// Copy contents of src to dest (non-recursive files only, plus dirs)
fn copy_dir_contents(src: &Path, dest: &Path) -> Result<()> {
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

fn count_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                count += 1;
            } else if entry.path().is_dir() {
                count += count_files(&entry.path());
            }
        }
    }
    count
}

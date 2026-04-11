use anyhow::{bail, Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Args)]
pub struct RebaseArgs {
    /// Packages to rebase (empty = all initialized packages)
    packages: Vec<String>,

    /// Target Rocky Linux version
    #[arg(long)]
    version: Option<String>,

    /// Show what would change, don't apply
    #[arg(long)]
    dry_run: bool,

    /// Keep the downloaded src.rpm for inspection
    #[arg(long)]
    keep_srpm: bool,
}

pub fn run(args: RebaseArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    let packages = if args.packages.is_empty() {
        discover_packages(&root)?
    } else {
        args.packages.clone()
    };

    if packages.is_empty() {
        println!("{} no packages to rebase", "→".dimmed());
        return Ok(());
    }

    let mut results = RebaseResults::default();

    for pkg in &packages {
        println!();
        match rebase_package(&root, pkg, &args) {
            Ok(RebaseOutcome::UpToDate) => {
                println!("{} {}: already up to date", "✓".green(), pkg);
                results.up_to_date += 1;
            }
            Ok(RebaseOutcome::Rebased { patches_applied, conflicts }) => {
                if conflicts.is_empty() {
                    println!(
                        "{} {}: rebased ({} patches reapplied)",
                        "✓".green(),
                        pkg,
                        patches_applied
                    );
                    results.success += 1;
                } else {
                    println!(
                        "{} {}: rebased with {} conflict(s)",
                        "⚠".yellow().bold(),
                        pkg,
                        conflicts.len()
                    );
                    for c in &conflicts {
                        println!("    conflict: {}", c);
                    }
                    results.conflicts += 1;
                }
            }
            Err(e) => {
                println!("{} {}: {}", "✗".red().bold(), pkg, e);
                results.failed += 1;
            }
        }
    }

    // Summary
    println!();
    println!("Rebase summary:");
    println!("  up to date: {}", results.up_to_date);
    println!("  rebased:    {}", results.success);
    println!("  conflicts:  {}", results.conflicts);
    println!("  failed:     {}", results.failed);

    if results.conflicts > 0 {
        println!();
        println!(
            "{} {} package(s) have conflicts. Resolve manually or use AI assist.",
            "⚠".yellow(),
            results.conflicts
        );
    }

    Ok(())
}

#[derive(Default)]
struct RebaseResults {
    up_to_date: usize,
    success: usize,
    conflicts: usize,
    failed: usize,
}

enum RebaseOutcome {
    UpToDate,
    Rebased {
        patches_applied: usize,
        conflicts: Vec<String>,
    },
}

/// Discover all initialized packages
fn discover_packages(root: &Path) -> Result<Vec<String>> {
    let src_dir = root.join("src");
    if !src_dir.exists() {
        return Ok(vec![]);
    }

    let mut packages = Vec::new();
    for entry in std::fs::read_dir(&src_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden dirs
            if !name.starts_with('.') {
                packages.push(name);
            }
        }
    }
    packages.sort();
    Ok(packages)
}

fn rebase_package(root: &Path, package: &str, args: &RebaseArgs) -> Result<RebaseOutcome> {
    let src = root.join("src").join(package);
    let patches_dir = root.join("patches").join(package);

    if !src.exists() {
        bail!("package not initialized. Run `evo init {}` first.", package);
    }

    // Step 1: Gather current patch stack
    let patches = list_patches(&patches_dir)?;
    if patches.is_empty() {
        println!("  {} no patches, checking upstream...", "→".dimmed());
        // Still worth checking if base source changed
    }

    // Step 2: Download new src.rpm
    println!("  {} fetching upstream src.rpm...", "→".dimmed());
    let srpm_path = download_srpm(package, args.version.as_deref())?;

    // Step 3: Extract new source to temp dir
    let tmp_new = root.join(format!(".tmp-rebase-{}", package));
    if tmp_new.exists() {
        std::fs::remove_dir_all(&tmp_new)?;
    }
    std::fs::create_dir_all(&tmp_new)?;

    extract_srpm(&srpm_path, &tmp_new)?;

    let new_sources = tmp_new.join("SOURCES");
    if !new_sources.exists() {
        std::fs::remove_dir_all(&tmp_new)?;
        bail!("no SOURCES dir in downloaded src.rpm");
    }

    // Step 4: Compare old vs new base source
    // We use git to diff: commit current state, overlay new, diff
    let old_tree = capture_tree_hash(&src)?;

    // Check if source actually changed
    let new_tree = compute_overlay_hash(&src, &new_sources)?;

    if old_tree == new_tree {
        println!("  {} base source unchanged", "✓".green());
        std::fs::remove_dir_all(&tmp_new).ok();
        if args.dry_run {
            println!("  (dry run - no changes)");
        }
        return Ok(RebaseOutcome::UpToDate);
    }

    if args.dry_run {
        println!(
            "  {} base source changed (would rebase {} patches)",
            "⚠".yellow(),
            patches.len()
        );
        std::fs::remove_dir_all(&tmp_new).ok();
        return Ok(RebaseOutcome::Rebased {
            patches_applied: 0,
            conflicts: vec![],
        });
    }

    // Step 5: Save old patches, reset to new base
    let tmp_patches = root.join(format!(".tmp-patches-{}", package));
    if tmp_patches.exists() {
        std::fs::remove_dir_all(&tmp_patches)?;
    }
    std::fs::create_dir_all(&tmp_patches)?;

    for patch in &patches {
        let name = patch.file_name().unwrap();
        std::fs::copy(patch, tmp_patches.join(name))?;
    }

    // Reset source to new base
    println!("  {} applying new base source...", "→".dimmed());
    reset_to_new_base(&src, &new_sources)?;

    // Update spec file if changed
    let new_specs = tmp_new.join("SPECS");
    if new_specs.exists() {
        update_spec(root, package, &new_specs)?;
    }

    // Step 6: Re-apply patches
    let mut conflicts = Vec::new();
    let mut applied = 0;

    if !patches.is_empty() {
        println!(
            "  {} reapplying {} patches...",
            "→".dimmed(),
            patches.len()
        );

        for patch in &patches {
            let name = patch.file_name().unwrap().to_string_lossy().to_string();
            let patch_path = tmp_patches.join(&name);

            print!("    {} {}...", "→".dimmed(), name);

            let result = Command::new("patch")
                .args(["-p1", "--forward", "--no-backup-if-mismatch"])
                .arg("-i")
                .arg(&patch_path)
                .current_dir(&src)
                .output()?;

            if result.status.success() {
                println!(" {}", "ok".green());
                applied += 1;
            } else {
                // Check if it partially applied
                let reject_check = Command::new("patch")
                    .args(["-p1", "--dry-run", "-i"])
                    .arg(&patch_path)
                    .current_dir(&src)
                    .output()?;

                if reject_check.status.success() {
                    // Conflicts but patch can still apply with fuzz
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    if stderr.contains("saving rejects") {
                        println!(" {}", "CONFLICT".red());
                        conflicts.push(name.clone());

                        // Show conflict hints
                        let rej_file = src.join(format!("{}.rej", "unknown"));
                        if rej_file.exists() {
                            println!("      rejects: {}", rej_file.display());
                        }
                    }
                } else {
                    println!(" {}", "FAILED".red());
                    conflicts.push(name);
                }
            }
        }
    }

    // Step 7: Commit the rebased state
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(&src)
        .output()?;

    Command::new("git")
        .args([
            "commit",
            "-q",
            "-m",
            &format!("rebase: upstream update ({} patches, {} conflicts)", applied, conflicts.len()),
        ])
        .current_dir(&src)
        .output()?;

    // Cleanup
    std::fs::remove_dir_all(&tmp_new).ok();
    std::fs::remove_dir_all(&tmp_patches).ok();
    if !args.keep_srpm {
        std::fs::remove_file(&srpm_path).ok();
    }

    // Copy updated spec
    // (already done in step 5)

    Ok(RebaseOutcome::Rebased {
        patches_applied: applied,
        conflicts,
    })
}

// ── Helpers ─────────────────────────────────────────────────

fn list_patches(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut patches: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |e| e == "patch"))
        .collect();
    patches.sort();
    Ok(patches)
}

fn download_srpm(package: &str, version: Option<&str>) -> Result<PathBuf> {
    let mut cmd = Command::new("dnf");
    cmd.args(["download", "--source", package]);
    if let Some(v) = version {
        cmd.args(["--releasever", v]);
    }

    let output = cmd.output().context("failed to run dnf download")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("dnf download failed for {}:\n{}", package, stderr);
    }

    let cwd = std::env::current_dir()?;
    for entry in std::fs::read_dir(&cwd)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().ends_with(".src.rpm") {
            return Ok(entry.path());
        }
    }

    bail!("no .src.rpm found after download for {}", package)
}

fn extract_srpm(srpm: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;

    let rpm2cpio = Command::new("rpm2cpio")
        .arg(srpm)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .context("failed to run rpm2cpio")?;

    Command::new("cpio")
        .args(["-idm", "--quiet"])
        .current_dir(dest)
        .stdin(rpm2cpio.stdout.unwrap())
        .output()
        .context("cpio extraction failed")?;

    Ok(())
}

/// Get a git tree hash of the current source state
fn capture_tree_hash(dir: &Path) -> Result<String> {
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()?;

    let output = Command::new("git")
        .args(["write-tree"])
        .current_dir(dir)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Compute what the tree hash would be after overlaying new sources
fn compute_overlay_hash(src: &Path, new_sources: &Path) -> Result<String> {
    // Copy new sources to temp, add, write-tree, clean up
    let tmp = src.join(".evo-overlay-tmp");
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(&tmp)?;

    // Copy current state
    copy_dir_contents(src, &tmp)?;

    // Overlay new sources
    copy_dir_contents(new_sources, &tmp)?;

    // Init git and get tree hash
    Command::new("git").args(["init", "-q"]).current_dir(&tmp).output()?;
    Command::new("git").args(["add", "-A"]).current_dir(&tmp).output()?;

    let output = Command::new("git")
        .args(["write-tree"])
        .current_dir(&tmp)
        .output()?;

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    std::fs::remove_dir_all(&tmp).ok();
    Ok(hash)
}

/// Reset source dir to new base (git rm everything, copy new, commit)
fn reset_to_new_base(src: &Path, new_sources: &Path) -> Result<()> {
    // Remove tracked files
    Command::new("git")
        .args(["rm", "-rf", "--quiet", "."])
        .current_dir(src)
        .output()?;

    // Remove untracked
    Command::new("git")
        .args(["clean", "-fdq"])
        .current_dir(src)
        .output()?;

    // Copy new sources
    copy_dir_contents(new_sources, src)?;

    // Stage and commit
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(src)
        .output()?;

    Command::new("git")
        .args(["commit", "-q", "--allow-empty", "-m", "rebase: new upstream base"])
        .current_dir(src)
        .output()?;

    Ok(())
}

fn update_spec(root: &Path, package: &str, new_specs: &Path) -> Result<()> {
    let specs_dir = root.join("specs");
    std::fs::create_dir_all(&specs_dir)?;

    for entry in std::fs::read_dir(new_specs)? {
        let entry = entry?;
        if entry.path().extension().map_or(false, |e| e == "spec") {
            let dest = specs_dir.join(format!("{}.spec", package));
            std::fs::copy(entry.path(), &dest)?;
            println!("  {} spec updated", "✓".green());
        }
    }
    Ok(())
}

fn copy_dir_contents(src: &Path, dest: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let name = entry.file_name();

        // Skip .git
        if name == ".git" {
            continue;
        }

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

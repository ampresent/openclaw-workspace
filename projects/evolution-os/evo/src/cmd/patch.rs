use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Subcommand)]
pub enum PatchCmd {
    /// Create a new patch from current source changes
    Create(PatchCreateArgs),
    /// List patches in a package's patch stack
    List(PatchListArgs),
    /// Drop the last N patches from the stack
    Drop(PatchDropArgs),
    /// Show diff of a specific patch
    Show(PatchShowArgs),
    /// Apply all patches in stack to a clean source tree
    Apply(PatchApplyArgs),
}

#[derive(Args)]
pub struct PatchCreateArgs {
    /// Package name
    package: String,

    /// Patch description
    #[arg(long, short)]
    message: Option<String>,
}

#[derive(Args)]
pub struct PatchListArgs {
    /// Package name
    package: String,
}

#[derive(Args)]
pub struct PatchDropArgs {
    /// Package name
    package: String,

    /// Number of patches to drop from top of stack
    #[arg(default_value = "1")]
    count: usize,
}

#[derive(Args)]
pub struct PatchShowArgs {
    /// Package name
    package: String,

    /// Patch index (1-based) or filename
    patch_ref: String,
}

#[derive(Args)]
pub struct PatchApplyArgs {
    /// Package name
    package: String,
}

pub fn run(cmd: PatchCmd, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    match cmd {
        PatchCmd::Create(args) => create_patch(&root, &args),
        PatchCmd::List(args) => list_patches(&root, &args),
        PatchCmd::Drop(args) => drop_patches(&root, &args),
        PatchCmd::Show(args) => show_patch(&root, &args),
        PatchCmd::Apply(args) => apply_patches(&root, &args),
    }
}

// ── Patch Stack Layout ──────────────────────────────────────
//
// patches/<package>/
//   0001-feature-one.patch
//   0002-fix-bug.patch
//   0003-another.patch
//
// Each patch is a standard unified diff.
// Stack order = filename sort order.

fn patches_dir(root: &Path, package: &str) -> PathBuf {
    root.join("patches").join(package)
}

fn src_dir(root: &Path, package: &str) -> PathBuf {
    root.join("src").join(package)
}

/// List all patches in a package stack, sorted by number
fn list_patch_files(root: &Path, package: &str) -> Result<Vec<PathBuf>> {
    let dir = patches_dir(root, package);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut patches: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |e| e == "patch"))
        .collect();

    patches.sort();
    Ok(patches)
}

/// Get next patch number in the stack
fn next_patch_number(root: &Path, package: &str) -> Result<u32> {
    let patches = list_patch_files(root, package)?;
    if patches.is_empty() {
        return Ok(1);
    }

    let last = patches.last().unwrap();
    let name = last.file_stem().unwrap().to_string_lossy();
    let num_str = name.split('-').next().unwrap_or("0");
    Ok(num_str.parse::<u32>().unwrap_or(0) + 1)
}

// ── Commands ────────────────────────────────────────────────

fn create_patch(root: &Path, args: &PatchCreateArgs) -> Result<()> {
    let src = src_dir(root, &args.package);
    let pdir = patches_dir(root, &args.package);

    if !src.exists() {
        bail!("package '{}' not initialized. Run `evo init {}` first.", args.package, args.package);
    }

    std::fs::create_dir_all(&pdir)?;

    // Use git diff in the source dir
    // First, check if there are changes
    let status = Command::new("git")
        .args(["diff", "--quiet", "HEAD"])
        .current_dir(&src)
        .status()?;

    if status.success() {
        // Also check untracked files
        let untracked = Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(&src)
            .output()?;

        if untracked.stdout.is_empty() {
            println!("{} no changes in {}", "→".dimmed(), args.package);
            return Ok(());
        }

        // Stage untracked files for the diff
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(&src)
            .output()?;
    }

    // Generate diff
    let diff = Command::new("git")
        .args(["diff", "HEAD"])
        .current_dir(&src)
        .output()?;

    if diff.stdout.is_empty() {
        println!("{} no changes to capture", "→".dimmed());
        return Ok(());
    }

    // Build patch filename
    let num = next_patch_number(root, &args.package)?;
    let desc = args.message
        .as_deref()
        .unwrap_or("custom")
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();

    let filename = format!("{:04}-{}.patch", num, desc);
    let patch_path = pdir.join(&filename);

    std::fs::write(&patch_path, &diff.stdout)?;

    // Commit the source changes so git diff HEAD is clean again
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(&src)
        .output()?;

    Command::new("git")
        .args(["commit", "-q", "-m", &format!("patch: {}", filename)])
        .current_dir(&src)
        .output()?;

    println!("{} created {}", "✓".green(), patch_path.display());

    // Show patch stats
    let line_count = diff.stdout.iter().filter(|&&b| b == b'\n').count();
    println!("  {} lines", line_count);

    Ok(())
}

fn list_patches(root: &Path, args: &PatchListArgs) -> Result<()> {
    let patches = list_patch_files(root, &args.package)?;

    if patches.is_empty() {
        println!("no patches for {}", args.package);
        return Ok(());
    }

    println!("Patch stack for {} ({} patches):", args.package, patches.len());
    for (i, p) in patches.iter().enumerate() {
        let name = p.file_name().unwrap().to_string_lossy();
        let size = std::fs::metadata(p)
            .map(|m| m.len())
            .unwrap_or(0);
        println!("  {}. {} ({} bytes)", i + 1, name, size);
    }

    Ok(())
}

fn drop_patches(root: &Path, args: &PatchDropArgs) -> Result<()> {
    let patches = list_patch_files(root, &args.package)?;

    if patches.is_empty() {
        bail!("patch stack for '{}' is empty", args.package);
    }

    if args.count >= patches.len() {
        bail!(
            "cannot drop {} patches, only {} in stack",
            args.count,
            patches.len()
        );
    }

    // Drop last N patches
    let to_drop = &patches[patches.len() - args.count..];
    for p in to_drop {
        let name = p.file_name().unwrap().to_string_lossy();
        std::fs::remove_file(p)?;
        println!("{} dropped {}", "✓".green(), name);
    }

    // Reset source git to the last remaining patch's commit
    // TODO: better approach - re-apply remaining patches to clean tree

    Ok(())
}

fn show_patch(root: &Path, args: &PatchShowArgs) -> Result<()> {
    let patches = list_patch_files(root, &args.package)?;

    // Try to find by index or name
    let patch = if let Ok(idx) = args.patch_ref.parse::<usize>() {
        patches.get(idx - 1).cloned()
    } else {
        patches
            .iter()
            .find(|p| p.file_name().unwrap().to_string_lossy().contains(&args.patch_ref))
            .cloned()
    };

    match patch {
        Some(path) => {
            let content = std::fs::read_to_string(&path)?;
            println!("{}:", path.file_name().unwrap().to_string_lossy());
            println!("{}", content);
        }
        None => bail!("patch '{}' not found in {}", args.patch_ref, args.package),
    }

    Ok(())
}

fn apply_patches(root: &Path, args: &PatchApplyArgs) -> Result<()> {
    let patches = list_patch_files(root, &args.package)?;
    let src = src_dir(root, &args.package);

    if !src.exists() {
        bail!("package '{}' not initialized", args.package);
    }

    if patches.is_empty() {
        println!("no patches to apply for {}", args.package);
        return Ok(());
    }

    println!("applying {} patches to {}...", patches.len(), args.package);

    for patch in &patches {
        let name = patch.file_name().unwrap().to_string_lossy();
        print!("  {} {}...", "→".dimmed(), name);

        // git apply supports new files, renames, and binary diffs
        let result = Command::new("git")
            .args(["apply", "--whitespace=nowarn"])
            .arg(patch)
            .current_dir(&src)
            .output()?;

        if result.status.success() {
            println!(" {}", "ok".green());
        } else {
            println!(" {}", "FAILED".red());
            let stderr = String::from_utf8_lossy(&result.stderr);
            eprintln!("{}", stderr);
            bail!("patch failed: {}", name);
        }
    }

    println!("{} all patches applied", "✓".green());
    Ok(())
}

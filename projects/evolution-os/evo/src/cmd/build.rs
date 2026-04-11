use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

#[derive(Args)]
pub struct BuildArgs {
    /// Packages to build (empty = all with changes)
    packages: Vec<String>,

    /// Force rebuild even if no changes
    #[arg(long)]
    force: bool,

    /// Number of parallel make jobs
    #[arg(long, short = 'j')]
    jobs: Option<usize>,

    /// Apply patches before building
    #[arg(long, default_value = "true")]
    apply_patches: bool,
}

pub fn run(args: BuildArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    // Check frozen state
    if root.join(".evo").join("frozen").exists() {
        bail!("system is frozen — run `evo freeze --unfreeze` first");
    }

    let packages = if args.packages.is_empty() {
        super::util::discover_packages(&root)?
            .into_iter()
            .filter(|p| {
                if args.force {
                    return true;
                }
                let patches_dir = root.join("patches").join(p);
                let patches = super::util::list_patches(&patches_dir).unwrap_or_default();
                let src = root.join("src").join(p);
                super::util::has_changes(&src) || !patches.is_empty()
            })
            .collect()
    } else {
        args.packages.clone()
    };

    if packages.is_empty() {
        println!("{} no packages to build", "→".dimmed());
        return Ok(());
    }

    let mut built = 0;
    let mut failed = 0;

    for pkg in &packages {
        println!();
        match build_package(&root, pkg, &args) {
            Ok(stats) => {
                println!(
                    "{} {} built ({} files → {}ms)",
                    "✓".green().bold(),
                    pkg,
                    stats.source_files,
                    stats.duration_ms
                );
                built += 1;
            }
            Err(e) => {
                println!("{} {} failed: {}", "✗".red().bold(), pkg, e);
                failed += 1;
            }
        }
    }

    println!();
    println!("Build summary: {} built, {} failed", built, failed);

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

struct BuildStats {
    source_files: usize,
    duration_ms: u128,
}

fn build_package(root: &Path, package: &str, args: &BuildArgs) -> Result<BuildStats> {
    use std::time::Instant;
    let start = Instant::now();

    let src = root.join("src").join(package);
    let patches_dir = root.join("patches").join(package);
    let spec = root.join("specs").join(format!("{}.spec", package));
    let builds_dir = root.join("builds").join(package);

    if !src.exists() {
        bail!("package '{}' not initialized", package);
    }

    // Step 1: Apply patches if requested
    if args.apply_patches {
        let patches = super::util::list_patches(&patches_dir)?;
        if !patches.is_empty() {
            println!(
                "  {} applying {} patches...",
                "→".dimmed(),
                patches.len()
            );
            let (applied, failed) = super::util::apply_patches(&src, &patches)?;
            if !failed.is_empty() {
                bail!("patches failed: {}", failed.join(", "));
            }
            println!("  {} {} patches applied", "✓".green(), applied);
        }
    }

    // Step 2: Prepare build directory
    if builds_dir.exists() {
        std::fs::remove_dir_all(&builds_dir)?;
    }
    std::fs::create_dir_all(&builds_dir)?;

    // Step 3: rpmbuild
    if spec.exists() {
        println!("  {} building with rpmbuild...", "→".dimmed());
        run_rpmbuild(&src, &spec, &builds_dir, args.jobs)?;
    } else {
        // Fallback: make-based build
        println!("  {} no spec file, trying make...", "→".dimmed());
        run_make(&src, &builds_dir, args.jobs)?;
    }

    let duration = start.elapsed().as_millis();
    let files = super::util::count_files(&builds_dir);

    // Commit build state
    super::util::git_ok(&src, &["add", "-A"])?;
    super::util::git_ok(
        &src,
        &["commit", "-q", "--allow-empty", "-m", &format!("build: {}", package)],
    )?;

    Ok(BuildStats {
        source_files: files,
        duration_ms: duration,
    })
}

fn run_rpmbuild(
    src: &Path,
    spec: &Path,
    builds_dir: &Path,
    jobs: Option<usize>,
) -> Result<()> {
    let topdir = builds_dir.display().to_string();

    // Create rpmbuild directory structure
    for dir in &["BUILD", "RPMS", "SOURCES", "SPECS", "SRPMS"] {
        std::fs::create_dir_all(builds_dir.join(dir))?;
    }

    // Copy source files to SOURCES
    super::util::copy_tree(src, &builds_dir.join("SOURCES"))?;

    // Copy spec to SPECS
    std::fs::copy(spec, builds_dir.join("SPECS").join(spec.file_name().unwrap()))?;

    let nproc = jobs.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });

    let output = Command::new("rpmbuild")
        .args([
            "-bb",
            "--define",
            &format!("_topdir {}", topdir),
            "--define",
            &format!("_smp_build_ncpus {}", nproc),
            spec.to_str().unwrap(),
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            println!("  {} rpmbuild succeeded", "✓".green());
            // List built RPMs
            let rpms_dir = builds_dir.join("RPMS");
            if let Ok(entries) = std::fs::read_dir(&rpms_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "rpm") {
                        println!("    → {}", path.display());
                    }
                }
            }
            Ok(())
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let stdout = String::from_utf8_lossy(&o.stdout);
            bail!(
                "rpmbuild failed:\n{}\n{}",
                stdout.lines().take(20).collect::<Vec<_>>().join("\n"),
                stderr.lines().take(10).collect::<Vec<_>>().join("\n")
            );
        }
        Err(e) => {
            bail!("rpmbuild not found: {}. Install with: dnf install rpm-build", e);
        }
    }
}

fn run_make(src: &Path, builds_dir: &Path, jobs: Option<usize>) -> Result<()> {
    if !src.join("Makefile").exists() && !src.join("configure").exists() {
        bail!("no Makefile, configure, or spec file found");
    }

    let nproc = jobs.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });

    // Run configure if present
    if src.join("configure").exists() {
        let output = Command::new("./configure")
            .arg(&format!("--prefix={}", builds_dir.display()))
            .current_dir(src)
            .output()?;
        if !output.status.success() {
            bail!("configure failed");
        }
    }

    let output = Command::new("make")
        .args(["-j", &nproc.to_string()])
        .current_dir(src)
        .output()?;

    if !output.status.success() {
        bail!("make failed");
    }

    // make install to builds_dir
    Command::new("make")
        .args(["install", &format!("DESTDIR={}", builds_dir.display())])
        .current_dir(src)
        .output()?;

    println!("  {} make succeeded", "✓".green());
    Ok(())
}

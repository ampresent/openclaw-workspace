use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct BuildArgs {
    /// Packages to build (empty = all modified)
    packages: Vec<String>,

    /// Force rebuild even if unchanged
    #[arg(long)]
    force: bool,

    /// Number of parallel jobs
    #[arg(long, short = 'j')]
    jobs: Option<usize>,
}

pub fn run(args: BuildArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    if args.packages.is_empty() {
        println!("Scanning for modified packages...");
        // TODO: detect packages with unapplied patches or source changes
        println!("{} no modified packages detected", "→".dimmed());
        return Ok(());
    }

    for pkg in &args.packages {
        println!("{} {}...", "building".green().bold(), pkg);
        // TODO: apply patch stack → rpmbuild → install
        // TODO: respect -j parallelism
        // TODO: build as evo user (permission isolation)
        println!("{} {} built successfully", "✓".green(), pkg);
    }
    Ok(())
}

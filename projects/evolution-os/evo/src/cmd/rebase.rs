use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct RebaseArgs {
    /// Packages to rebase (empty = all)
    packages: Vec<String>,

    /// Target Rocky Linux version
    #[arg(long)]
    version: Option<String>,

    /// Show conflicts only, don't apply
    #[arg(long)]
    dry_run: bool,
}

pub fn run(args: RebaseArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    println!("{} against upstream...", "rebasing".green().bold());

    if args.packages.is_empty() {
        println!("  scanning all packages...");
        // TODO: enumerate root/src/* and root/patches/*
    }

    for pkg in &args.packages {
        println!("  {} {}...", "→".dimmed(), pkg);
        // TODO: fetch latest src.rpm, diff against current base
        // TODO: rebase patch stack on new base
        // TODO: detect conflicts, offer AI-assisted resolution
    }

    if args.dry_run {
        println!("{} dry run complete", "✓".green());
    } else {
        println!("{} rebase complete", "✓".green());
    }
    Ok(())
}

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;

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

    /// Patch index (1-based) or name
    patch_ref: String,
}

pub fn run(cmd: PatchCmd, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    match cmd {
        PatchCmd::Create(args) => {
            println!(
                "{} patch for {}...",
                "creating".green().bold(),
                args.package
            );
            // TODO: diff root/src/<package> → root/patches/<package>/NNNN-desc.patch
            // TODO: git add + commit
            println!("{} patch created", "✓".green());
        }
        PatchCmd::List(args) => {
            let patches_dir = root.join("patches").join(&args.package);
            if !patches_dir.exists() {
                println!("no patches for {}", args.package);
                return Ok(());
            }
            println!("Patch stack for {}:", args.package);
            // TODO: list *.patch in order
            println!("  (empty)");
        }
        PatchCmd::Drop(args) => {
            println!(
                "dropping last {} patch(es) from {}...",
                args.count, args.package
            );
            // TODO: remove last N patches, re-apply remaining
        }
        PatchCmd::Show(args) => {
            println!(
                "patch {} in {}:",
                args.patch_ref, args.package
            );
            // TODO: show patch content
        }
    }
    Ok(())
}

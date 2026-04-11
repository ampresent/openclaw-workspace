use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct TagArgs {
    /// Create a new stable tag
    #[arg(long)]
    create: Option<String>,

    /// Tag description
    #[arg(long, short)]
    message: Option<String>,

    /// List existing tags
    #[arg(long)]
    list: bool,
}

pub fn run(args: TagArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    if args.list {
        println!("Stable tags:");
        // TODO: read from root/.evo/tags.json
        println!("  (none)");
        return Ok(());
    }

    if let Some(name) = args.create {
        println!(
            "{} tag {}...",
            "creating".green().bold(),
            name
        );
        // TODO: snapshot all package states
        // TODO: record git commit hashes + patch stacks
        // TODO: save to root/.evo/tags.json
        println!("{} tag {} created", "✓".green(), name);
    } else {
        println!("use --create <name> or --list");
    }
    Ok(())
}

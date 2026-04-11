use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct StatusArgs {
    /// Show only packages with pending changes
    #[arg(long)]
    pending: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

pub fn run(args: StatusArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;

    if args.json {
        let status = crate::config::load_status(&root)?;
        println!("{}", serde_json::to_string_pretty(&status)?);
        return Ok(());
    }

    println!("╔══════════════════════════════════════════╗");
    println!("║         Evolution OS — evo status        ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  Source root: {:<28} ║", root.display());
    println!("║  Packages:    {:<28} ║", "—");
    println!("║  Pending:     {:<28} ║", "—");
    println!("║  Building:    {:<28} ║", "none");
    println!("║  Frozen:      {:<28} ║", "no");
    println!("╚══════════════════════════════════════════╝");

    // TODO: scan root/src/, root/patches/, show real status
    // TODO: TUI mode with ratatui
    Ok(())
}

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::Path;

#[derive(Args)]
pub struct FreezeArgs {
    /// Unfreeze (re-enable evolution)
    #[arg(long)]
    unfreeze: bool,

    /// Show current freeze status
    #[arg(long)]
    status: bool,
}

pub fn run(args: FreezeArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;
    let evo_dir = root.join(".evo");
    let lock_path = evo_dir.join("frozen");

    if args.status {
        if lock_path.exists() {
            let since = std::fs::read_to_string(&lock_path).unwrap_or_default();
            println!("System is {} (since {})", "FROZEN".red().bold(), since.trim());
            println!("Run `evo freeze --unfreeze` to resume evolution.");
        } else {
            println!("System is {} — evolution active", "LIVE".green().bold());
        }
        return Ok(());
    }

    if args.unfreeze {
        if lock_path.exists() {
            let since = std::fs::read_to_string(&lock_path).unwrap_or_default();
            std::fs::remove_file(&lock_path)?;
            println!(
                "{} evolution resumed (was frozen since {})",
                "✓".green().bold(),
                since.trim()
            );
        } else {
            println!("system is not frozen");
        }
        return Ok(());
    }

    // Freeze
    std::fs::create_dir_all(&evo_dir)?;
    let timestamp = chrono::Local::now().to_rfc3339();
    std::fs::write(&lock_path, &timestamp)?;

    println!(
        "{} system frozen — AI hooks disabled, no auto-builds",
        "✓".green().bold()
    );
    println!("  timestamp: {}", timestamp);
    println!("  unlock:    evo freeze --unfreeze");

    Ok(())
}

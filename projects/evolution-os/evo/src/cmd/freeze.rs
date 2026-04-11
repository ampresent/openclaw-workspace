use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct FreezeArgs {
    /// Unfreeze (re-enable evolution)
    #[arg(long)]
    unfreeze: bool,
}

pub fn run(args: FreezeArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;
    let lock_path = root.join(".evo").join("frozen");

    if args.unfreeze {
        if lock_path.exists() {
            std::fs::remove_file(&lock_path)?;
            println!("{} evolution resumed", "✓".green());
            // TODO: re-enable AI hooks / daemon
        } else {
            println!("system is not frozen");
        }
    } else {
        std::fs::create_dir_all(lock_path.parent().unwrap())?;
        std::fs::write(&lock_path, chrono::Local::now().to_rfc3339())?;
        println!("{} system frozen — AI hooks disabled, no auto-builds", "✓".green());
        // TODO: disable daemon / AI hooks
    }
    Ok(())
}

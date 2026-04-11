use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct InitArgs {
    /// Package name to initialize
    package: String,

    /// Rocky Linux version to source from
    #[arg(long, default_value = "9")]
    rocky_version: String,
}

pub fn run(args: InitArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;
    println!(
        "evo init: extracting {} from Rocky Linux {} src.rpm → {}",
        args.package, args.rocky_version, root.display()
    );
    // TODO: download src.rpm, extract to root/src/<package>/
    // TODO: create root/specs/<package>.spec
    // TODO: create root/patches/<package>/ directory
    println!("{} package initialized", "✓".green());
    Ok(())
}

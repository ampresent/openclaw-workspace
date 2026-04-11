mod cmd;
mod config;
mod error;

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "evo",
    version,
    about = "Evolution OS CLI - your system is a living source tree"
)]
struct Cli {
    /// Path to Evolution OS source root
    #[arg(long, global = true, env = "EVO_ROOT")]
    root: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a package from Rocky Linux src.rpm
    Init(cmd::init::InitArgs),
    /// Show system evolution status (TUI dashboard)
    Status(cmd::status::StatusArgs),
    /// Build packages
    Build(cmd::build::BuildArgs),
    /// Manage patch stacks
    Patch {
        #[command(subcommand)]
        cmd: cmd::patch::PatchCmd,
    },
    /// Rebase against upstream Rocky Linux
    Rebase(cmd::rebase::RebaseArgs),
    /// Create or manage stable tags
    Tag(cmd::tag::TagArgs),
    /// Freeze evolution (disable AI hooks and builds)
    Freeze(cmd::freeze::FreezeArgs),
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => cmd::init::run(args, cli.root.as_deref()),
        Commands::Status(args) => cmd::status::run(args, cli.root.as_deref()),
        Commands::Build(args) => cmd::build::run(args, cli.root.as_deref()),
        Commands::Patch { cmd } => cmd::patch::run(cmd, cli.root.as_deref()),
        Commands::Rebase(args) => cmd::rebase::run(args, cli.root.as_deref()),
        Commands::Tag(args) => cmd::tag::run(args, cli.root.as_deref()),
        Commands::Freeze(args) => cmd::freeze::run(args, cli.root.as_deref()),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use serde::Serialize;
use std::path::Path;

#[derive(Args)]
pub struct StatusArgs {
    /// Show only packages with pending patches
    #[arg(long)]
    pending: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Serialize)]
struct StatusReport {
    root: String,
    frozen: bool,
    packages: Vec<PkgStatus>,
}

#[derive(Serialize)]
struct PkgStatus {
    name: String,
    patches: usize,
    has_changes: bool,
    files: usize,
}

pub fn run(args: StatusArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;
    let packages = super::util::discover_packages(&root)?;
    let frozen = root.join(".evo").join("frozen").exists();

    let mut statuses = Vec::new();
    for pkg in &packages {
        let src = root.join("src").join(pkg);
        let patches_dir = root.join("patches").join(pkg);
        let patches = super::util::list_patches(&patches_dir)?;
        let has_changes = super::util::has_changes(&src);
        let files = super::util::count_files(&src);

        if args.pending && !has_changes && patches.is_empty() {
            continue;
        }

        statuses.push(PkgStatus {
            name: pkg.clone(),
            patches: patches.len(),
            has_changes,
            files,
        });
    }

    if args.json {
        let report = StatusReport {
            root: root.display().to_string(),
            frozen,
            packages: statuses,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    // TUI-style display
    println!(
        "╔══════════════════════════════════════════════════════════╗"
    );
    println!(
        "║              Evolution OS — evo status                  ║"
    );
    println!(
        "╠══════════════════════════════════════════════════════════╣"
    );
    println!(
        "║  Source root: {:<43} ║",
        truncate(&root.display().to_string(), 43)
    );
    println!(
        "║  Packages:    {:<43} ║",
        statuses.len()
    );
    println!(
        "║  Frozen:      {:<43} ║",
        if frozen {
            "yes ⛔".to_string()
        } else {
            "no".to_string()
        }
    );
    println!(
        "╠══════════════════════════════════════════════════════════╣"
    );

    if statuses.is_empty() {
        println!(
            "║  No packages initialized.                               ║"
        );
        println!(
            "║  Run: evo init <package>                                ║"
        );
    } else {
        println!(
            "║  {:<20} {:>8} {:>8} {:>8}  {} ║",
            "Package", "Patches", "Files", "Dirty", ""
        );
        println!(
            "║  {:<20} {:>8} {:>8} {:>8}  {} ║",
            "────────────────────", "────────", "────────", "────────", ""
        );
        for s in &statuses {
            let dirty = if s.has_changes {
                "yes".yellow().to_string()
            } else {
                "no".dimmed().to_string()
            };
            println!(
                "║  {:<20} {:>8} {:>8} {:>14}    ║",
                truncate(&s.name, 20),
                s.patches,
                s.files,
                dirty
            );
        }
    }

    println!(
        "╚══════════════════════════════════════════════════════════╝"
    );
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::Path;

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

    /// Delete a tag
    #[arg(long)]
    delete: Option<String>,

    /// Show details of a specific tag
    #[arg(long)]
    show: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Tag {
    name: String,
    description: String,
    created_at: String,
    packages: Vec<PackageSnapshot>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PackageSnapshot {
    name: String,
    git_hash: String,
    patches: Vec<String>,
    patch_count: usize,
}

pub fn run(args: TagArgs, root: Option<&str>) -> Result<()> {
    let root = crate::config::resolve_root(root)?;
    let tags_dir = root.join(".evo").join("tags");

    if args.list {
        return list_tags(&tags_dir);
    }

    if let Some(ref name) = args.show {
        return show_tag(&tags_dir, name);
    }

    if let Some(ref name) = args.delete {
        return delete_tag(&tags_dir, name);
    }

    if let Some(ref name) = args.create {
        return create_tag(&root, &tags_dir, name, args.message.as_deref());
    }

    println!("use --create <name>, --list, --show <name>, or --delete <name>");
    Ok(())
}

fn create_tag(root: &Path, tags_dir: &Path, name: &str, message: Option<&str>) -> Result<()> {
    let packages = super::util::discover_packages(root)?;

    let mut snapshots = Vec::new();
    for pkg in &packages {
        let src = root.join("src").join(pkg);
        let patches_dir = root.join("patches").join(pkg);

        let git_hash = if src.join(".git").exists() {
            super::util::git_output(&src, &["rev-parse", "HEAD"]).unwrap_or_default()
        } else {
            "no-git".to_string()
        };

        let patches = super::util::list_patches(&patches_dir)?;
        let patch_names: Vec<String> = patches
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        snapshots.push(PackageSnapshot {
            name: pkg.clone(),
            git_hash,
            patch_count: patch_names.len(),
            patches: patch_names,
        });
    }

    let tag = Tag {
        name: name.to_string(),
        description: message.unwrap_or("").to_string(),
        created_at: chrono::Local::now().to_rfc3339(),
        packages: snapshots,
    };

    std::fs::create_dir_all(tags_dir)?;
    let tag_file = tags_dir.join(format!("{}.json", name));
    if tag_file.exists() {
        bail!("tag '{}' already exists. Delete it first with --delete {}", name, name);
    }

    let json = serde_json::to_string_pretty(&tag)?;
    std::fs::write(&tag_file, &json)?;

    println!("{} tag {} created", "✓".green().bold(), name);
    println!("  packages: {}", tag.packages.len());
    for p in &tag.packages {
        println!(
            "    {} ({} patches, HEAD: {}…)",
            p.name,
            p.patch_count,
            &p.git_hash[..8.min(p.git_hash.len())]
        );
    }
    println!("  file: {}", tag_file.display());

    Ok(())
}

fn list_tags(tags_dir: &Path) -> Result<()> {
    if !tags_dir.exists() {
        println!("no tags");
        return Ok(());
    }

    let mut tags: Vec<Tag> = Vec::new();
    for entry in std::fs::read_dir(tags_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(tag) = serde_json::from_str::<Tag>(&content) {
                    tags.push(tag);
                }
            }
        }
    }

    if tags.is_empty() {
        println!("no tags");
        return Ok(());
    }

    tags.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    println!("Tags ({}):", tags.len());
    for tag in &tags {
        println!(
            "  {} — {} packages, {}",
            tag.name.bold(),
            tag.packages.len(),
            tag.created_at
        );
        if !tag.description.is_empty() {
            println!("    {}", tag.description.dimmed());
        }
    }

    Ok(())
}

fn show_tag(tags_dir: &Path, name: &str) -> Result<()> {
    let tag_file = tags_dir.join(format!("{}.json", name));
    if !tag_file.exists() {
        bail!("tag '{}' not found", name);
    }

    let content = std::fs::read_to_string(&tag_file)?;
    let tag: Tag = serde_json::from_str(&content)?;

    println!("Tag: {}", tag.name.bold());
    if !tag.description.is_empty() {
        println!("Description: {}", tag.description);
    }
    println!("Created: {}", tag.created_at);
    println!("Packages ({}):", tag.packages.len());
    for p in &tag.packages {
        println!(
            "  {} — HEAD: {}…, {} patches",
            p.name,
            &p.git_hash[..8.min(p.git_hash.len())],
            p.patch_count
        );
        for patch in &p.patches {
            println!("    {}", patch.dimmed());
        }
    }

    Ok(())
}

fn delete_tag(tags_dir: &Path, name: &str) -> Result<()> {
    let tag_file = tags_dir.join(format!("{}.json", name));
    if !tag_file.exists() {
        bail!("tag '{}' not found", name);
    }

    std::fs::remove_file(&tag_file)?;
    println!("{} tag {} deleted", "✓".green(), name);
    Ok(())
}

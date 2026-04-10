mod spec;
mod container;
mod overlay;
mod layer;
mod error;

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand, ValueEnum};
use anyhow::{Result, anyhow};

use layer::LayerBackend;
use layer::diff::DiffBackend;
use layer::git::GitBackend;
use layer::tar::TarBackend;
use layer::hardlink::HardlinkBackend;

#[derive(Clone, ValueEnum, Debug)]
enum Backend {
    /// File-level diff with SHA256 manifest
    Diff,
    /// Git-based version control
    Git,
    /// Tar archive per layer (Docker-like)
    Tar,
    /// Hardlink snapshots (space-efficient)
    Hardlink,
}

impl Backend {
    fn into_backend(&self) -> Arc<dyn LayerBackend> {
        match self {
            Backend::Diff => Arc::new(DiffBackend::new()),
            Backend::Git => Arc::new(GitBackend::new()),
            Backend::Tar => Arc::new(TarBackend::new()),
            Backend::Hardlink => Arc::new(HardlinkBackend::new()),
        }
    }
}

#[derive(Parser)]
#[command(name = "runb", version, about = "A lightweight chroot-only OCI container runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a container from an OCI bundle
    Create {
        #[arg(required = true)]
        id: String,
        #[arg(short, long, default_value = ".")]
        bundle: PathBuf,
    },

    /// Start a created container
    Start {
        #[arg(required = true)]
        id: String,
    },

    /// Stop a running container
    Stop {
        #[arg(required = true)]
        id: String,
        #[arg(short, long)]
        signal: Option<i32>,
    },

    /// Delete a stopped container
    Delete {
        #[arg(required = true)]
        id: String,
    },

    /// Query container state
    State {
        #[arg(required = true)]
        id: String,
    },

    /// List all containers
    List,

    /// Prepare overlay bind mounts
    Prepare {
        #[arg(required = true)]
        id: String,
    },

    /// Tear down overlay bind mounts
    Teardown {
        #[arg(required = true)]
        id: String,
    },

    /// Verify overlay integrity
    Verify {
        #[arg(required = true)]
        id: String,
    },

    /// Hot upgrade: teardown → delete → create → prepare → start
    Upgrade {
        #[arg(required = true)]
        id: String,
        #[arg(short, long, default_value = ".")]
        bundle: PathBuf,
    },

    /// Initialize layer tracking (snapshot base image)
    InitLayer {
        #[arg(required = true)]
        id: String,
        /// Version management backend
        #[arg(short, long, value_enum, default_value_t = Backend::Diff)]
        backend: Backend,
    },

    /// Commit current rootfs state as a new layer
    Commit {
        #[arg(required = true)]
        id: String,
        #[arg(short, long, default_value = "")]
        message: String,
    },

    /// List all committed layers
    Layers {
        #[arg(required = true)]
        id: String,
    },

    /// Rebase: replace base OS and re-apply user layers
    Rebase {
        #[arg(required = true)]
        id: String,
        #[arg(required = true)]
        new_rootfs: String,
    },

    /// Benchmark: compare all backends on the same workload
    Bench {
        #[arg(required = true)]
        rootfs: String,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { id, bundle } => {
            container::create(&id, &bundle)?;
            println!("Created container: {}", id);
        }
        Commands::Start { id } => {
            let meta = container::load_metadata(&id)?;
            let bundle = PathBuf::from(&meta.bundle);
            let config = overlay::OverlayConfig::load(&bundle)?;
            if !config.overlay.links.is_empty() {
                overlay::prepare(&meta.rootfs, &config.overlay)?;
            }
            container::start(&id)?;
            println!("Started container: {}", id);
        }
        Commands::Stop { id, signal } => {
            container::stop(&id, signal)?;
            println!("Stopped container: {}", id);
        }
        Commands::Delete { id } => {
            container::delete(&id)?;
            println!("Deleted container: {}", id);
        }
        Commands::State { id } => {
            let state = container::state(&id)?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
        Commands::List => {
            let containers = container::list()?;
            if containers.is_empty() {
                println!("No containers");
            } else {
                for c in &containers {
                    println!("{}\t{:?}\tpid:{:?}\t{}", c.id, c.state, c.pid, c.bundle);
                }
            }
        }
        Commands::Prepare { id } => {
            let meta = container::load_metadata(&id)?;
            let bundle = PathBuf::from(&meta.bundle);
            let config = overlay::OverlayConfig::load(&bundle)?;
            if config.overlay.links.is_empty() {
                println!("No overlay configured");
            } else {
                overlay::prepare(&meta.rootfs, &config.overlay)?;
                println!("Overlay prepared: {} dirs", config.overlay.links.len());
            }
        }
        Commands::Teardown { id } => {
            let meta = container::load_metadata(&id)?;
            let bundle = PathBuf::from(&meta.bundle);
            let config = overlay::OverlayConfig::load(&bundle)?;
            let removed = overlay::teardown(&meta.rootfs, &config.overlay)?;
            for r in &removed { println!("Removed: {}", r); }
            if removed.is_empty() { println!("Nothing to teardown"); }
        }
        Commands::Verify { id } => {
            let meta = container::load_metadata(&id)?;
            let bundle = PathBuf::from(&meta.bundle);
            let config = overlay::OverlayConfig::load(&bundle)?;
            let issues = overlay::verify(&meta.rootfs, &config.overlay)?;
            if issues.is_empty() {
                println!("All overlays OK");
            } else {
                for i in &issues { println!("ISSUE: {}", i); }
            }
        }
        Commands::Upgrade { id, bundle } => {
            let meta = container::state(&id)?;
            let is_running = meta.state == container::ContainerState::Running;
            let old_rootfs = meta.rootfs.clone();
            let old_bundle = PathBuf::from(&meta.bundle);
            let old_config = overlay::OverlayConfig::load(&old_bundle)?;
            if is_running { container::stop(&id, None)?; }
            if !old_config.overlay.links.is_empty() {
                overlay::teardown(&old_rootfs, &old_config.overlay)?;
            }
            container::delete(&id)?;
            container::create(&id, &bundle)?;
            let new_meta = container::load_metadata(&id)?;
            let new_config = overlay::OverlayConfig::load(&bundle)?;
            if !new_config.overlay.links.is_empty() {
                overlay::prepare(&new_meta.rootfs, &new_config.overlay)?;
            }
            container::start(&id)?;
            println!("Hot upgrade complete: {}", id);
        }
        Commands::InitLayer { id, backend } => {
            let meta = container::load_metadata(&id)?;
            let bundle = PathBuf::from(&meta.bundle);
            let layers_dir = bundle.join("layers");
            let engine = backend.into_backend();
            println!("Using backend: {}", engine.name());
            engine.init(std::path::Path::new(&meta.rootfs), &layers_dir)?;
        }
        Commands::Commit { id, message } => {
            let meta = container::state(&id)?;
            let bundle = PathBuf::from(&meta.bundle);
            let layers_dir = bundle.join("layers");
            // Auto-detect backend from existing layers
            let engine = detect_backend(&layers_dir)?;
            let desc = if message.is_empty() {
                format!("commit at {}", chrono_now())
            } else {
                message
            };
            engine.commit(std::path::Path::new(&meta.rootfs), &layers_dir, &desc)?;
        }
        Commands::Layers { id } => {
            let meta = container::load_metadata(&id)?;
            let bundle = PathBuf::from(&meta.bundle);
            let layers_dir = bundle.join("layers");
            let engine = detect_backend(&layers_dir)?;
            let layers = engine.list(&layers_dir)?;
            if layers.is_empty() {
                println!("No layers (backend: {})", engine.name());
            } else {
                println!("Layers (backend: {}):", engine.name());
                for l in &layers {
                    let s = &l.stats;
                    println!("  layer-{:03}  +{} -{} ~{}  {} bytes  {}",
                        l.layer_number, s.files_added, s.files_deleted,
                        s.files_changed, s.bytes_written, l.description);
                }
                let disk = engine.layer_disk_size(&layers_dir)?;
                println!("Total layer disk: {} bytes ({:.1} KB)", disk, disk as f64 / 1024.0);
            }
        }
        Commands::Rebase { id, new_rootfs } => {
            let meta = container::state(&id)?;
            let bundle = PathBuf::from(&meta.bundle);
            let layers_dir = bundle.join("layers");
            let engine = detect_backend(&layers_dir)?;
            let config = overlay::OverlayConfig::load(&bundle)?;
            if meta.state == container::ContainerState::Running {
                container::stop(&id, None)?;
            }
            if !config.overlay.links.is_empty() {
                overlay::teardown(&meta.rootfs, &config.overlay)?;
            }
            engine.rebase(
                std::path::Path::new(&meta.rootfs),
                &layers_dir,
                std::path::Path::new(&new_rootfs),
            )?;
            if !config.overlay.links.is_empty() {
                overlay::prepare(&meta.rootfs, &config.overlay)?;
            }
            println!("Rebase complete (backend: {})", engine.name());
        }
        Commands::Bench { rootfs } => {
            run_benchmark(&rootfs)?;
        }
    }

    Ok(())
}

/// Detect which backend was used for existing layers.
/// Uses the presence of backend-specific markers.
fn detect_backend(layers_dir: &PathBuf) -> Result<Arc<dyn LayerBackend>> {
    if layers_dir.join("git-repo").join(".git").exists() {
        return Ok(Arc::new(GitBackend::new()));
    }
    if layers_dir.join("base.tar").exists() {
        return Ok(Arc::new(TarBackend::new()));
    }
    if layers_dir.join("base").exists() {
        return Ok(Arc::new(HardlinkBackend::new()));
    }
    // Default to diff
    Ok(Arc::new(DiffBackend::new()))
}

fn chrono_now() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let s = ts % 86400;
    let d = ts / 86400;
    format!("day{}+{:02}:{:02}:{:02}", d, s/3600, (s%3600)/60, s%60)
}

fn run_benchmark(rootfs: &str) -> Result<()> {
    use std::time::Instant;
    let rootfs_path = std::path::Path::new(rootfs);
    if !rootfs_path.exists() {
        return Err(anyhow!("Rootfs does not exist: {}", rootfs));
    }

    let backends: Vec<(&str, Arc<dyn LayerBackend>)> = vec![
        ("diff", Arc::new(DiffBackend::new())),
        ("tar", Arc::new(TarBackend::new())),
        ("hardlink", Arc::new(HardlinkBackend::new())),
    ];

    println!("=== runb Layer Backend Benchmark ===");
    println!("Rootfs: {} ({} files)", rootfs, layer::generate_manifest(rootfs_path)?.len());
    println!();

    let tmp_dir = std::path::Path::new("/tmp/runb-bench");
    if tmp_dir.exists() { std::fs::remove_dir_all(tmp_dir)?; }

    for (name, backend) in &backends {
        println!("--- Backend: {} ---", name);

        let layers_dir = tmp_dir.join(name);
        let bench_rootfs = tmp_dir.join(format!("rootfs-{}", name));
        layer::copy_recursive(rootfs_path, &bench_rootfs)?;

        // Init
        let t0 = Instant::now();
        backend.init(&bench_rootfs, &layers_dir)?;
        let init_ms = t0.elapsed().as_millis();
        println!("  init:     {} ms", init_ms);

        // Make a change
        std::fs::write(bench_rootfs.join("etc").join("test.conf"), "config=value\n")?;
        std::fs::create_dir_all(bench_rootfs.join("usr/local/bin"))?;
        std::fs::write(bench_rootfs.join("usr/local/bin/app"), "#!/bin/sh\necho ok\n")?;

        // Commit
        let t1 = Instant::now();
        backend.commit(&bench_rootfs, &layers_dir, "test commit")?;
        let commit_ms = t1.elapsed().as_millis();
        println!("  commit:   {} ms", commit_ms);

        // Make another change
        std::fs::write(bench_rootfs.join("etc").join("test.conf"), "config=v2\n")?;
        let t2 = Instant::now();
        backend.commit(&bench_rootfs, &layers_dir, "update config")?;
        let commit2_ms = t2.elapsed().as_millis();
        println!("  commit2:  {} ms", commit2_ms);

        // List
        let t3 = Instant::now();
        let layers = backend.list(&layers_dir)?;
        let list_ms = t3.elapsed().as_millis();
        println!("  list:     {} ms ({} layers)", list_ms, layers.len());

        // Apply
        let apply_rootfs = tmp_dir.join(format!("apply-{}", name));
        layer::copy_recursive(rootfs_path, &apply_rootfs)?;
        let t4 = Instant::now();
        for l in &layers {
            backend.apply(&apply_rootfs, &layers_dir, l.layer_number)?;
        }
        let apply_ms = t4.elapsed().as_millis();
        println!("  apply:    {} ms", apply_ms);

        // Verify
        let conf = std::fs::read_to_string(apply_rootfs.join("etc").join("test.conf"))?;
        let app = apply_rootfs.join("usr/local/bin/app").exists();
        println!("  verify:   test.conf={}, app={}", conf.trim(), if app { "exists" } else { "MISSING" });

        // Disk size
        let disk = backend.layer_disk_size(&layers_dir)?;
        println!("  disk:     {} bytes ({:.1} KB)", disk, disk as f64 / 1024.0);
        println!();
    }

    // Cleanup
    std::fs::remove_dir_all(tmp_dir).ok();

    println!("=== Summary ===");
    println!("diff:      Simple, small layers, no dedup");
    println!("tar:       Portable, compressed, Docker-compatible");
    println!("hardlink:  Fast access, space-efficient via sharing");
    println!("git:       (not benchmarked — needs git binary)");

    Ok(())
}

use std::fmt;

#[derive(Debug)]
pub enum EvoError {
    /// Package not found in source tree
    PackageNotFound(String),
    /// Patch stack is empty, nothing to drop
    PatchStackEmpty(String),
    /// Build failed for a package
    BuildFailed { package: String, reason: String },
    /// System is frozen, operation not allowed
    Frozen,
    /// Upstream rebase conflict
    RebaseConflict { package: String, patch: String },
}

impl fmt::Display for EvoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackageNotFound(pkg) => write!(f, "package '{}' not found in source tree", pkg),
            Self::PatchStackEmpty(pkg) => {
                write!(f, "patch stack for '{}' is empty, nothing to drop", pkg)
            }
            Self::BuildFailed { package, reason } => {
                write!(f, "build failed for '{}': {}", package, reason)
            }
            Self::Frozen => write!(f, "system is frozen — run `evo freeze --unfreeze` first"),
            Self::RebaseConflict { package, patch } => {
                write!(f, "rebase conflict in '{}' patch '{}'", package, patch)
            }
        }
    }
}

impl std::error::Error for EvoError {}

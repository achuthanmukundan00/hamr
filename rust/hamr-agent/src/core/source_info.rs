//! Port of `packages/coding-agent/src/core/source-info.ts`.
//!
//! Metadata about where a prompt template or other artifact was loaded from.

/// Scope of a source: who owns the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceScope {
    /// Installed in the user's global config directory.
    User,
    /// Lives in the project-local config directory.
    Project,
    /// Generated or temporary (not persisted in a config dir).
    Temporary,
}

/// Origin of a source — distinguishes bundled/packaged artifacts from
/// top-level user-provided ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceOrigin {
    /// Came from a package dependency.
    Package,
    /// Top-level / user-provided.
    TopLevel,
}

/// Describes where a prompt template or other artifact was loaded from.
#[derive(Debug, Clone)]
pub struct SourceInfo {
    /// Resolved absolute filesystem path.
    pub path: String,
    /// Generic source category (e.g. `"local"`).
    pub source: String,
    /// Scope (user, project, or temporary).
    pub scope: SourceScope,
    /// Origin (package or top-level).
    pub origin: SourceOrigin,
    /// Optional base directory for resolving relative paths.
    pub base_dir: Option<String>,
}

/// Options for creating a synthetic [`SourceInfo`].
#[derive(Debug, Clone)]
pub struct SyntheticSourceInfoOptions {
    pub source: String,
    pub scope: Option<SourceScope>,
    pub origin: Option<SourceOrigin>,
    pub base_dir: Option<String>,
}

/// Create a synthetic [`SourceInfo`] for a given path and options.
/// Used when the path doesn't come from a package-manager metadata entry.
pub fn create_synthetic_source_info(path: &str, options: SyntheticSourceInfoOptions) -> SourceInfo {
    SourceInfo {
        path: path.to_string(),
        source: options.source,
        scope: options.scope.unwrap_or(SourceScope::Temporary),
        origin: options.origin.unwrap_or(SourceOrigin::TopLevel),
        base_dir: options.base_dir,
    }
}

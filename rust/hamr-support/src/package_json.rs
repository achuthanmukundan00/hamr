//! Types and manipulation for `package.json` files.
//!
//! This module provides [`PackageJson`], a strongly-typed representation of an
//! npm `package.json` manifest. It covers all fields used by the Hamr workspace
//! packages and the published `@skaft/hamr` tarball.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Representation of an npm `package.json` manifest.
///
/// All fields are `Option<T>` — absent fields are omitted during serialization
/// via `#[serde(skip_serializing_if = "Option::is_none")]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub package_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exports: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundled_dependencies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engines: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributors: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hamr_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hamr: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pi: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<HashMap<String, String>>,
}

impl PackageJson {
    /// Read and parse a `package.json` from a file path.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let pkg: PackageJson = serde_json::from_str(&contents)?;
        Ok(pkg)
    }

    /// Serialize and write this `PackageJson` to a file path.
    pub fn to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, format!("{}\n", json))?;
        Ok(())
    }

    /// Create a shallow clone with `dependencies`, `dev_dependencies`, and
    /// `peer_dependencies` all set to `None`. Used when vendoring @hamr/* libs
    /// so the bundled copy doesn't claim dependencies.
    pub fn strip_deps(&self) -> Self {
        let mut stripped = self.clone();
        stripped.dependencies = None;
        stripped.dev_dependencies = None;
        stripped.peer_dependencies = None;
        stripped
    }

    /// Convenience constructor: an empty `PackageJson`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for PackageJson {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
            description: None,
            package_type: None,
            bin: None,
            main: None,
            types: None,
            exports: None,
            files: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            bundled_dependencies: None,
            optional_dependencies: None,
            overrides: None,
            engines: None,
            keywords: None,
            author: None,
            contributors: None,
            license: None,
            repository: None,
            hamr_config: None,
            hamr: None,
            pi: None,
            scripts: None,
        }
    }
}

//! Port of `packages/coding-agent/src/core/diagnostics.ts`.
//!
//! Resource collision and diagnostic types surfaced during extension loading.

use serde::{Deserialize, Serialize};

/// Describes a collision between two extensions claiming the same resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCollision {
    pub resource_type: ResourceType,
    /// Skill name, command/tool/flag name, prompt name, or theme name.
    pub name: String,
    pub winner_path: String,
    pub loser_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loser_source: Option<String>,
}

/// The kind of resource involved in a collision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceType {
    Extension,
    Skill,
    Prompt,
    Theme,
}

/// A diagnostic produced during resource loading / extension boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDiagnostic {
    #[serde(rename = "type")]
    pub diagnostic_type: DiagnosticType,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collision: Option<ResourceCollision>,
}

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticType {
    Warning,
    Error,
    Collision,
}

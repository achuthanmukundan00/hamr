//! Config and session migrations — port of `packages/coding-agent/src/migrations.ts`.
//!
//! Handles migrating old config/session formats to newer versions.
//! Schema versions are tracked and incrementally upgraded.

/// Current config schema version.
pub const CONFIG_SCHEMA_VERSION: u32 = 3;

/// Current session schema version (re-exported from session_manager).
pub use crate::core::session_manager::CURRENT_SESSION_VERSION;

/// Apply config migrations to bring old configs up to date.
///
/// Returns the migrated config value and any warnings.
pub fn migrate_config(config: &mut serde_json::Value) -> Vec<String> {
    let mut warnings = Vec::new();
    let version = config
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    // Version 0/1 → 2: rename "defaultModel" to "defaultModelId"
    if version < 2 {
        if let Some(model) = config.get("defaultModel").cloned() {
            if config.get("defaultModelId").is_none() {
                config["defaultModelId"] = model;
            }
        }
        if let Some(obj) = config.as_object_mut() {
            obj.remove("defaultModel");
        }
        config["schemaVersion"] = serde_json::json!(2);
        warnings
            .push("Migrated config from schema v0/1 to v2 (defaultModel → defaultModelId)".into());
    }

    // Version 2 → 3: add "quietStartup" default
    if version < 3 {
        if config.get("quietStartup").is_none() {
            config["quietStartup"] = serde_json::json!(false);
        }
        config["schemaVersion"] = serde_json::json!(3);
        warnings.push("Migrated config from schema v2 to v3 (added quietStartup default)".into());
    }

    warnings
}

/// Apply session entry migrations.
///
/// Session entries use `CURRENT_SESSION_VERSION` for version tracking.
pub fn migrate_session_entries(entries: &[serde_json::Value]) -> Vec<serde_json::Value> {
    entries
        .iter()
        .map(|entry| {
            let mut e = entry.clone();
            if let Some(version) = e.get("version").and_then(|v| v.as_u64()) {
                if (version as u32) < CURRENT_SESSION_VERSION {
                    // No field migrations needed yet; version bump is sufficient.
                    e["version"] = serde_json::json!(CURRENT_SESSION_VERSION);
                }
            }
            e
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_config_v1_to_v3() {
        let mut config = serde_json::json!({
            "schemaVersion": 1,
            "defaultModel": "claude-sonnet-4-5"
        });
        let warnings = migrate_config(&mut config);
        assert!(warnings.len() >= 1);
        assert_eq!(config["schemaVersion"], 3);
        assert!(config.get("defaultModel").is_none());
        assert_eq!(config["defaultModelId"], "claude-sonnet-4-5");
        assert_eq!(config["quietStartup"], false);
    }

    #[test]
    fn test_migrate_config_no_migration_needed() {
        let mut config = serde_json::json!({
            "schemaVersion": 3,
            "quietStartup": true
        });
        let warnings = migrate_config(&mut config);
        assert!(warnings.is_empty());
        assert_eq!(config["schemaVersion"], 3);
    }

    #[test]
    fn test_migrate_session_entries() {
        let entries = vec![
            serde_json::json!({"type": "session", "version": 2, "id": "abc"}),
            serde_json::json!({"type": "message", "id": "msg1"}),
        ];
        let migrated = migrate_session_entries(&entries);
        assert_eq!(migrated[0]["version"], CURRENT_SESSION_VERSION);
        assert!(migrated[1].get("version").is_none());
    }
}

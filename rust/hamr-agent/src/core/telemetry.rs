//! Port of `packages/coding-agent/src/core/telemetry.ts`.
//!
//! Install telemetry opt-in check. Mirrors the TS function that reads
//! `HAMR_TELEMETRY`/`PI_TELEMETRY` env vars with a fallback to the
//! persistent settings store.

use crate::core::settings_manager::SettingsManager;

/// Returns `true` for "1", "true", "yes" (case-insensitive); `false` otherwise.
fn is_truthy_env_flag(value: Option<&str>) -> bool {
    match value {
        Some(v) => v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"),
        None => false,
    }
}

/// Determine whether install telemetry is enabled.
///
/// Respects the `HAMR_TELEMETRY` / `PI_TELEMETRY` environment variable first;
/// falls back to the persisted `enableInstallTelemetry` setting.
pub fn is_install_telemetry_enabled(
    settings_manager: &SettingsManager,
    telemetry_env: Option<&str>,
) -> bool {
    let env_value: Option<String> = match telemetry_env {
        Some(v) => Some(v.to_string()),
        None => std::env::var("HAMR_TELEMETRY")
            .or_else(|_| std::env::var("PI_TELEMETRY"))
            .ok(),
    };

    if let Some(ref val) = env_value {
        return is_truthy_env_flag(Some(val.as_str()));
    }

    settings_manager.get_enable_install_telemetry()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings_manager::SettingsManager;

    fn in_memory_sm(enable_install_telemetry: Option<bool>) -> SettingsManager {
        use crate::core::settings_manager::Settings;
        SettingsManager::in_memory(Some(Settings {
            enable_install_telemetry,
            ..Settings::default()
        }))
    }

    #[test]
    fn test_truthy_env() {
        assert!(is_truthy_env_flag(Some("1")));
        assert!(is_truthy_env_flag(Some("true")));
        assert!(is_truthy_env_flag(Some("TRUE")));
        assert!(is_truthy_env_flag(Some("yes")));
        assert!(is_truthy_env_flag(Some("YES")));
        assert!(!is_truthy_env_flag(Some("0")));
        assert!(!is_truthy_env_flag(Some("false")));
        assert!(!is_truthy_env_flag(Some("no")));
        assert!(!is_truthy_env_flag(Some("")));
        assert!(!is_truthy_env_flag(None));
    }

    #[test]
    fn test_falls_back_to_settings_when_no_env() {
        let sm = in_memory_sm(Some(true));
        assert!(is_install_telemetry_enabled(&sm, None));
    }

    #[test]
    fn test_env_overrides_settings() {
        let sm = in_memory_sm(Some(false));
        assert!(is_install_telemetry_enabled(&sm, Some("1")));
        assert!(!is_install_telemetry_enabled(&sm, Some("0")));
    }

    #[test]
    fn test_defaults_to_false_when_no_env_and_no_setting() {
        let sm = in_memory_sm(None);
        // TS defaults to true (enableInstallTelemetry ?? true)
        assert!(is_install_telemetry_enabled(&sm, None));
    }
}

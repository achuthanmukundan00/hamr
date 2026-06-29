//! Auth guidance messages for missing API keys and models.
//!
//! Ported from `packages/coding-agent/src/core/auth-guidance.ts`.

const UNKNOWN_PROVIDER: &str = "unknown";

/// Get the provider login help text referencing documentation paths.
fn get_provider_login_help() -> String {
    let docs_path = "~/.hamr/docs"; // placeholder — config::getDocsPath() equivalent
    format!(
        "Use /login to log into a provider via OAuth or API key. See:\n  {docs_path}/providers.md\n  {docs_path}/models.md"
    )
}

/// Format a message when no models are available.
pub fn format_no_models_available_message() -> String {
    format!("No models available. {}", get_provider_login_help())
}

/// Format a message when no model is selected.
pub fn format_no_model_selected_message() -> String {
    format!(
        "No model selected.\n\n{}\n\nThen use /model to select a model.",
        get_provider_login_help()
    )
}

/// Format a message when no API key is found for a provider.
pub fn format_no_api_key_found_message(provider: &str) -> String {
    let display = if provider == UNKNOWN_PROVIDER {
        "the selected model"
    } else {
        provider
    };
    format!(
        "No API key found for {display}.\n\n{}",
        get_provider_login_help()
    )
}

// ---------------------------------------------------------------------------
// Tests — verbatim from TS test expectations
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_no_api_key_found_message_uses_provider_name() {
        let msg = format_no_api_key_found_message("anthropic");
        assert!(msg.contains("No API key found for anthropic"));
    }

    #[test]
    fn test_format_no_api_key_found_message_fallback_for_unknown() {
        let msg = format_no_api_key_found_message("unknown");
        assert!(msg.contains("No API key found for the selected model"));
    }

    #[test]
    fn test_format_no_model_selected_message() {
        let msg = format_no_model_selected_message();
        assert!(msg.contains("No model selected"));
        assert!(msg.contains("/login"));
        assert!(msg.contains("/model"));
    }

    #[test]
    fn test_format_no_models_available_message() {
        let msg = format_no_models_available_message();
        assert!(msg.contains("No models available"));
        assert!(msg.contains("/login"));
    }
}

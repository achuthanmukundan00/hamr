/// Theme engine — three-layer resolution system.
/// Built-in defaults → TOML config → agent runtime overrides.
pub mod capability;
pub mod config;
pub mod palette;
pub mod tokens;

use std::collections::HashMap;

/// Resolved theme providing render-time styling.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Token values (resolved through all 3 layers).
    values: HashMap<String, String>,
    /// Agent runtime overrides (highest priority).
    overrides: HashMap<String, String>,
    /// Path to the TOML config file for reloading.
    config_path: Option<String>,
    /// Current capability tier.
    tier: capability::CapabilityTier,
}

impl Theme {
    pub fn new(tier: capability::CapabilityTier) -> Self {
        Theme {
            values: HashMap::new(),
            overrides: HashMap::new(),
            config_path: None,
            tier,
        }
    }

    /// Load theme from built-in defaults + TOML config.
    pub fn load(config_path: Option<&str>, tier: capability::CapabilityTier) -> Self {
        let mut theme = Theme::new(tier);
        theme.config_path = config_path.map(|s| s.to_string());
        tokens::apply_defaults(&mut theme.values);
        if let Some(path) = config_path {
            config::load_toml(path, &mut theme.values);
        }
        theme
    }

    /// Get the effective value for a token, checking overrides first.
    fn resolve_value(&self, token: &str) -> Option<String> {
        self.overrides
            .get(token)
            .or_else(|| self.values.get(token))
            .cloned()
    }

    /// Apply a foreground color token to text.
    pub fn fg(&self, token: &str, text: &str) -> String {
        let color = self
            .resolve_value(token)
            .unwrap_or_else(|| token.to_string());
        palette::apply_fg(&color, text)
    }

    /// Apply a background color token to text.
    pub fn bg(&self, token: &str, text: &str) -> String {
        let color = self
            .resolve_value(token)
            .unwrap_or_else(|| token.to_string());
        palette::apply_bg(&color, text)
    }

    /// Apply bold styling to text.
    pub fn bold(&self, text: &str) -> String {
        format!("\x1b[1m{}\x1b[22m", text)
    }

    /// Apply dim styling to text.
    pub fn dim(&self, text: &str) -> String {
        format!("\x1b[2m{}\x1b[22m", text)
    }

    /// Resolve an icon token (with capability-aware fallback).
    pub fn icon(&self, token: &str) -> String {
        self.resolve_value(token)
            .unwrap_or_else(|| tokens::ascii_fallback(token).to_string())
    }

    /// Get the current capability tier.
    pub fn capability_tier(&self) -> capability::CapabilityTier {
        self.tier
    }

    /// Resolve a model name to its accent color hex.
    /// Tries model_<name> token first, falls back to model_default → accent.
    pub fn model_color(&self, model_name: &str) -> String {
        let key = format!("model_{}", model_name.to_lowercase());
        self.resolve_value(&key)
            .or_else(|| self.resolve_value("model_default"))
            .unwrap_or_else(|| "accent".to_string())
    }

    /// Get activity glyph by name (ready, thinking, tool_start, tool_done, error, user).
    pub fn activity_glyph(&self, name: &str) -> String {
        let key = format!("glyph_{}", name);
        self.icon(&key)
    }

    /// Get spinner frames as a Vec<String>.
    pub fn spinner_frames(&self) -> Vec<String> {
        let raw = self
            .resolve_value("loader_spinner_frames")
            .unwrap_or_else(|| "⠋,⠙,⠹,⠸,⠼,⠴,⠦,⠧,⠇,⠏".to_string());
        raw.split(',').map(|s| s.trim().to_string()).collect()
    }

    /// Override a token value at runtime (agent layer — highest priority).
    pub fn override_token(&mut self, key: &str, value: &str) {
        self.overrides.insert(key.to_string(), value.to_string());
    }

    /// Clear a single agent override, restoring the lower-layer value.
    pub fn clear_override(&mut self, key: &str) {
        self.overrides.remove(key);
    }

    /// Clear all agent overrides, restoring lower-layer values.
    pub fn clear_all_overrides(&mut self) {
        self.overrides.clear();
    }

    /// List all overrideable token keys.
    pub fn keys(&self) -> Vec<&str> {
        self.values.keys().map(|k| k.as_str()).collect()
    }

    /// Resolve a token value through all layers.
    pub fn resolve<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.resolve_value(key).and_then(|v| v.parse().ok())
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::new(capability::CapabilityTier::Baseline)
    }
}

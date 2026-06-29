//! Port of `packages/coding-agent/src/core/keybindings.ts`.
//!
//! App-level keybindings, migration of legacy key names, and the
//! `KeybindingsManager` that wraps the TUI keybinding engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types from @hamr/tui (stubs — integrate with sexy-tui-rs later)
// ---------------------------------------------------------------------------

/// A keybinding identifier string (e.g. "ctrl+p", "escape").
pub type KeyId = String;

/// Map from keybinding names to resolved key sequences.
pub type KeybindingsConfig = HashMap<String, serde_json::Value>;

/// Definition for a single keybinding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingDefinition {
    pub default_keys: serde_json::Value,
    pub description: String,
}

/// Registry of all known keybinding definitions.
pub type KeybindingDefinitions = HashMap<String, KeybindingDefinition>;

/// A single keybinding entry (name → raw keys).
pub type Keybinding = serde_json::Value;

// ---------------------------------------------------------------------------
// App-level keybinding definitions
// ---------------------------------------------------------------------------

/// Build the platform-aware KEYBINDINGS map.
fn build_keybindings() -> KeybindingDefinitions {
    let mut map = KeybindingDefinitions::new();

    // TUI keybindings (stubs — real ones come from sexy-tui-rs)
    let tui_keys: [(&str, &str, &str); 0] = [];

    for (name, keys, desc) in tui_keys {
        map.insert(
            name.to_string(),
            KeybindingDefinition {
                default_keys: serde_json::Value::String(keys.to_string()),
                description: desc.to_string(),
            },
        );
    }

    // App keybindings
    let is_windows = cfg!(target_os = "windows");

    map.insert(
        "app.interrupt".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("escape".into()),
            description: "Cancel or abort".into(),
        },
    );
    map.insert(
        "app.clear".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+c".into()),
            description: "Clear editor".into(),
        },
    );
    map.insert(
        "app.exit".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+d".into()),
            description: "Exit when editor is empty".into(),
        },
    );
    map.insert(
        "app.suspend".into(),
        KeybindingDefinition {
            default_keys: if is_windows {
                serde_json::Value::Array(vec![])
            } else {
                serde_json::Value::String("ctrl+z".into())
            },
            description: "Suspend to background".into(),
        },
    );
    map.insert(
        "app.thinking.cycle".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("shift+tab".into()),
            description: "Cycle thinking level".into(),
        },
    );
    map.insert(
        "app.model.cycleForward".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+p".into()),
            description: "Cycle to next model".into(),
        },
    );
    map.insert(
        "app.model.cycleBackward".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("shift+ctrl+p".into()),
            description: "Cycle to previous model".into(),
        },
    );
    map.insert(
        "app.model.select".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+l".into()),
            description: "Open model selector".into(),
        },
    );
    map.insert(
        "app.tools.expand".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+o".into()),
            description: "Toggle tool output".into(),
        },
    );
    map.insert(
        "app.thinking.toggle".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+t".into()),
            description: "Toggle thinking blocks".into(),
        },
    );
    map.insert(
        "app.session.toggleNamedFilter".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+n".into()),
            description: "Toggle named session filter".into(),
        },
    );
    map.insert(
        "app.editor.external".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+g".into()),
            description: "Open external editor".into(),
        },
    );
    map.insert(
        "app.message.followUp".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("alt+enter".into()),
            description: "Queue follow-up message".into(),
        },
    );
    map.insert(
        "app.message.dequeue".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("alt+up".into()),
            description: "Restore queued messages".into(),
        },
    );
    map.insert(
        "app.clipboard.pasteImage".into(),
        KeybindingDefinition {
            default_keys: if is_windows {
                serde_json::Value::String("alt+v".into())
            } else {
                serde_json::Value::String("ctrl+v".into())
            },
            description: "Paste image from clipboard".into(),
        },
    );
    map.insert(
        "app.session.new".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::Array(vec![]),
            description: "Start a new session".into(),
        },
    );
    map.insert(
        "app.session.tree".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::Array(vec![]),
            description: "Open session tree".into(),
        },
    );
    map.insert(
        "app.session.fork".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::Array(vec![]),
            description: "Fork current session".into(),
        },
    );
    map.insert(
        "app.session.resume".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::Array(vec![]),
            description: "Resume a session".into(),
        },
    );
    map.insert(
        "app.tree.foldOrUp".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::Array(vec![
                serde_json::Value::String("ctrl+left".into()),
                serde_json::Value::String("alt+left".into()),
            ]),
            description: "Fold tree branch or move up".into(),
        },
    );
    map.insert(
        "app.tree.unfoldOrDown".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::Array(vec![
                serde_json::Value::String("ctrl+right".into()),
                serde_json::Value::String("alt+right".into()),
            ]),
            description: "Unfold tree branch or move down".into(),
        },
    );
    map.insert(
        "app.tree.editLabel".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("shift+l".into()),
            description: "Edit tree label".into(),
        },
    );
    map.insert(
        "app.tree.toggleLabelTimestamp".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("shift+t".into()),
            description: "Toggle tree label timestamps".into(),
        },
    );
    map.insert(
        "app.session.togglePath".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+p".into()),
            description: "Toggle session path display".into(),
        },
    );
    map.insert(
        "app.session.toggleSort".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+s".into()),
            description: "Toggle session sort mode".into(),
        },
    );
    map.insert(
        "app.session.rename".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+r".into()),
            description: "Rename session".into(),
        },
    );
    map.insert(
        "app.session.delete".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+d".into()),
            description: "Delete session".into(),
        },
    );
    map.insert(
        "app.session.deleteNoninvasive".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+backspace".into()),
            description: "Delete session when query is empty".into(),
        },
    );
    map.insert(
        "app.models.save".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+s".into()),
            description: "Save model selection".into(),
        },
    );
    map.insert(
        "app.models.enableAll".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+a".into()),
            description: "Enable all models".into(),
        },
    );
    map.insert(
        "app.models.clearAll".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+x".into()),
            description: "Clear all models".into(),
        },
    );
    map.insert(
        "app.models.toggleProvider".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+p".into()),
            description: "Toggle all models for provider".into(),
        },
    );
    map.insert(
        "app.models.reorderUp".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("alt+up".into()),
            description: "Move model up in order".into(),
        },
    );
    map.insert(
        "app.models.reorderDown".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("alt+down".into()),
            description: "Move model down in order".into(),
        },
    );
    map.insert(
        "app.tree.filter.default".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+d".into()),
            description: "Tree filter: default view".into(),
        },
    );
    map.insert(
        "app.tree.filter.noTools".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+t".into()),
            description: "Tree filter: hide tool results".into(),
        },
    );
    map.insert(
        "app.tree.filter.userOnly".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+u".into()),
            description: "Tree filter: user messages only".into(),
        },
    );
    map.insert(
        "app.tree.filter.labeledOnly".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+l".into()),
            description: "Tree filter: labeled entries only".into(),
        },
    );
    map.insert(
        "app.tree.filter.all".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+a".into()),
            description: "Tree filter: show all entries".into(),
        },
    );
    map.insert(
        "app.tree.filter.cycleForward".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("ctrl+o".into()),
            description: "Tree filter: cycle forward".into(),
        },
    );
    map.insert(
        "app.tree.filter.cycleBackward".into(),
        KeybindingDefinition {
            default_keys: serde_json::Value::String("shift+ctrl+o".into()),
            description: "Tree filter: cycle backward".into(),
        },
    );

    map
}

/// Default keybinding definitions for the application.
pub static KEYBINDINGS: std::sync::LazyLock<KeybindingDefinitions> =
    std::sync::LazyLock::new(build_keybindings);

// ---------------------------------------------------------------------------
// Keybinding name migrations
// ---------------------------------------------------------------------------

/// Maps legacy flat keybinding names to their new namespaced equivalents.
static KEYBINDING_NAME_MIGRATIONS: std::sync::LazyLock<HashMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("cursorUp", "tui.editor.cursorUp");
        m.insert("cursorDown", "tui.editor.cursorDown");
        m.insert("cursorLeft", "tui.editor.cursorLeft");
        m.insert("cursorRight", "tui.editor.cursorRight");
        m.insert("cursorWordLeft", "tui.editor.cursorWordLeft");
        m.insert("cursorWordRight", "tui.editor.cursorWordRight");
        m.insert("cursorLineStart", "tui.editor.cursorLineStart");
        m.insert("cursorLineEnd", "tui.editor.cursorLineEnd");
        m.insert("jumpForward", "tui.editor.jumpForward");
        m.insert("jumpBackward", "tui.editor.jumpBackward");
        m.insert("pageUp", "tui.editor.pageUp");
        m.insert("pageDown", "tui.editor.pageDown");
        m.insert("deleteCharBackward", "tui.editor.deleteCharBackward");
        m.insert("deleteCharForward", "tui.editor.deleteCharForward");
        m.insert("deleteWordBackward", "tui.editor.deleteWordBackward");
        m.insert("deleteWordForward", "tui.editor.deleteWordForward");
        m.insert("deleteToLineStart", "tui.editor.deleteToLineStart");
        m.insert("deleteToLineEnd", "tui.editor.deleteToLineEnd");
        m.insert("yank", "tui.editor.yank");
        m.insert("yankPop", "tui.editor.yankPop");
        m.insert("undo", "tui.editor.undo");
        m.insert("newLine", "tui.input.newLine");
        m.insert("submit", "tui.input.submit");
        m.insert("tab", "tui.input.tab");
        m.insert("copy", "tui.input.copy");
        m.insert("selectUp", "tui.select.up");
        m.insert("selectDown", "tui.select.down");
        m.insert("selectPageUp", "tui.select.pageUp");
        m.insert("selectPageDown", "tui.select.pageDown");
        m.insert("selectConfirm", "tui.select.confirm");
        m.insert("selectCancel", "tui.select.cancel");
        m.insert("interrupt", "app.interrupt");
        m.insert("clear", "app.clear");
        m.insert("exit", "app.exit");
        m.insert("suspend", "app.suspend");
        m.insert("cycleThinkingLevel", "app.thinking.cycle");
        m.insert("cycleModelForward", "app.model.cycleForward");
        m.insert("cycleModelBackward", "app.model.cycleBackward");
        m.insert("selectModel", "app.model.select");
        m.insert("expandTools", "app.tools.expand");
        m.insert("toggleThinking", "app.thinking.toggle");
        m.insert("toggleSessionNamedFilter", "app.session.toggleNamedFilter");
        m.insert("externalEditor", "app.editor.external");
        m.insert("followUp", "app.message.followUp");
        m.insert("dequeue", "app.message.dequeue");
        m.insert("pasteImage", "app.clipboard.pasteImage");
        m.insert("newSession", "app.session.new");
        m.insert("tree", "app.session.tree");
        m.insert("fork", "app.session.fork");
        m.insert("resume", "app.session.resume");
        m.insert("treeFoldOrUp", "app.tree.foldOrUp");
        m.insert("treeUnfoldOrDown", "app.tree.unfoldOrDown");
        m.insert("treeEditLabel", "app.tree.editLabel");
        m.insert("treeToggleLabelTimestamp", "app.tree.toggleLabelTimestamp");
        m.insert("toggleSessionPath", "app.session.togglePath");
        m.insert("toggleSessionSort", "app.session.toggleSort");
        m.insert("renameSession", "app.session.rename");
        m.insert("deleteSession", "app.session.delete");
        m.insert("deleteSessionNoninvasive", "app.session.deleteNoninvasive");
        m
    });

fn is_legacy_keybinding_name(key: &str) -> bool {
    KEYBINDING_NAME_MIGRATIONS.contains_key(key)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_record(value: &serde_json::Value) -> bool {
    value.is_object()
}

/// Convert a raw JSON value into a `KeybindingsConfig` (HashMap of string to serde_json::Value).
fn to_keybindings_config(value: &serde_json::Value) -> KeybindingsConfig {
    let mut config = KeybindingsConfig::new();

    if let Some(obj) = value.as_object() {
        for (key, binding) in obj {
            if binding.is_string() {
                config.insert(key.clone(), binding.clone());
            } else if binding.is_array() {
                let arr = binding.as_array().unwrap();
                if arr.iter().all(|e| e.is_string()) {
                    config.insert(key.clone(), binding.clone());
                }
            }
        }
    }

    config
}

/// Migrate legacy keybinding names to their namespaced equivalents.
///
/// Returns the migrated config map and a boolean indicating whether any
/// migration occurred.
pub fn migrate_keybindings_config(
    raw_config: &serde_json::Value,
) -> (serde_json::Map<String, serde_json::Value>, bool) {
    let mut config = serde_json::Map::new();
    let mut migrated = false;

    if let Some(obj) = raw_config.as_object() {
        for (key, value) in obj {
            let next_key = if is_legacy_keybinding_name(key) {
                KEYBINDING_NAME_MIGRATIONS[key.as_str()].to_string()
            } else {
                key.clone()
            };

            if next_key != *key {
                migrated = true;
            }

            // If both old and new keys exist, skip the old one (keep the new)
            if key != &next_key && obj.contains_key(&next_key) {
                migrated = true;
                continue;
            }

            config.insert(next_key, value.clone());
        }
    }

    let ordered = order_keybindings_config(&config);
    (ordered, migrated)
}

/// Order keybindings config: known keys first (in definition order), then extras sorted.
fn order_keybindings_config(
    config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut ordered = serde_json::Map::new();

    // Known keybindings in definition order
    for keybinding_name in KEYBINDINGS.keys() {
        if let Some(value) = config.get(keybinding_name) {
            ordered.insert(keybinding_name.clone(), value.clone());
        }
    }

    // Extras (unknown keys) sorted
    let mut extras: Vec<String> = config
        .keys()
        .filter(|k| !ordered.contains_key(k.as_str()))
        .cloned()
        .collect();
    extras.sort();

    for key in extras {
        if let Some(value) = config.get(&key) {
            ordered.insert(key, value.clone());
        }
    }

    ordered
}

/// Load raw keybindings config from a JSON file. Returns None if the file
/// doesn't exist or can't be parsed as an object.
fn load_raw_config(path: &Path) -> Option<serde_json::Value> {
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    if parsed.is_object() {
        Some(parsed)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// KeybindingsManager
// ---------------------------------------------------------------------------

/// Manages app-level keybindings, wrapping the TUI keybinding engine.
/// Handles loading, migrating, and reloading user keybinding config files.
pub struct KeybindingsManager {
    /// User-provided keybinding overrides (post-migration).
    user_bindings: KeybindingsConfig,
    /// Path to the keybindings.json config file, if persisted.
    config_path: Option<PathBuf>,
    /// The merged effective bindings cache.
    effective_bindings: KeybindingsConfig,
}

impl KeybindingsManager {
    /// Create a new KeybindingsManager from user bindings and optional config path.
    pub fn new(user_bindings: KeybindingsConfig, config_path: Option<PathBuf>) -> Self {
        let effective = Self::resolve_bindings(&user_bindings);
        KeybindingsManager {
            user_bindings,
            config_path,
            effective_bindings: effective,
        }
    }

    /// Create a KeybindingsManager by loading user bindings from the default
    /// keybindings.json in the agent directory.
    pub fn create(agent_dir: &Path) -> Self {
        let config_path = agent_dir.join("keybindings.json");
        let user_bindings = Self::load_from_file(&config_path);
        Self::new(user_bindings, Some(config_path))
    }

    /// Reload user keybindings from the persisted config file.
    pub fn reload(&mut self) {
        if let Some(ref config_path) = self.config_path {
            self.set_user_bindings(Self::load_from_file(config_path));
        }
    }

    /// Get the fully resolved effective keybinding config.
    pub fn get_effective_config(&self) -> &KeybindingsConfig {
        &self.effective_bindings
    }

    /// Get the raw user bindings (post-migration).
    pub fn get_user_bindings(&self) -> &KeybindingsConfig {
        &self.user_bindings
    }

    /// Replace user bindings and recompute effective config.
    pub fn set_user_bindings(&mut self, bindings: KeybindingsConfig) {
        self.effective_bindings = Self::resolve_bindings(&bindings);
        self.user_bindings = bindings;
    }

    /// Load user keybindings from a JSON file, applying migration.
    fn load_from_file(path: &Path) -> KeybindingsConfig {
        let raw_config = match load_raw_config(path) {
            Some(v) => v,
            None => return KeybindingsConfig::new(),
        };

        let (migrated_map, _migrated) = migrate_keybindings_config(&raw_config);

        // Convert serde_json::Map back to our KeybindingsConfig
        let mut config = KeybindingsConfig::new();
        for (k, v) in &migrated_map {
            config.insert(k.clone(), v.clone());
        }
        config
    }

    /// Resolve bindings: start with defaults, overlay user bindings.
    fn resolve_bindings(user_bindings: &KeybindingsConfig) -> KeybindingsConfig {
        let mut resolved = KeybindingsConfig::new();

        // Start with defaults from KEYBINDINGS
        for (name, def) in KEYBINDINGS.iter() {
            resolved.insert(name.clone(), def.default_keys.clone());
        }

        // Overlay user bindings
        for (name, keys) in user_bindings {
            resolved.insert(name.clone(), keys.clone());
        }

        resolved
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_migrate_rewrites_old_key_names() {
        let mut raw = serde_json::Map::new();
        raw.insert(
            "cursorUp".into(),
            serde_json::Value::Array(vec![
                serde_json::Value::String("up".into()),
                serde_json::Value::String("ctrl+p".into()),
            ]),
        );
        raw.insert(
            "expandTools".into(),
            serde_json::Value::String("ctrl+x".into()),
        );

        let raw_value = serde_json::Value::Object(raw);
        let (migrated, _migrated_flag) = migrate_keybindings_config(&raw_value);

        assert_eq!(
            migrated.get("tui.editor.cursorUp"),
            Some(&serde_json::Value::Array(vec![
                serde_json::Value::String("up".into()),
                serde_json::Value::String("ctrl+p".into()),
            ]))
        );
        assert_eq!(
            migrated.get("app.tools.expand"),
            Some(&serde_json::Value::String("ctrl+x".into()))
        );
    }

    #[test]
    fn test_migrate_keeps_namespaced_when_both_exist() {
        let mut raw = serde_json::Map::new();
        raw.insert(
            "expandTools".into(),
            serde_json::Value::String("ctrl+x".into()),
        );
        raw.insert(
            "app.tools.expand".into(),
            serde_json::Value::String("ctrl+y".into()),
        );

        let raw_value = serde_json::Value::Object(raw);
        let (migrated, _migrated_flag) = migrate_keybindings_config(&raw_value);

        // Should keep the namespaced value
        assert_eq!(
            migrated.get("app.tools.expand"),
            Some(&serde_json::Value::String("ctrl+y".into()))
        );
        // Old name should not be present
        assert!(!migrated.contains_key("expandTools"));
    }

    #[test]
    fn test_loads_old_key_names_in_memory() {
        let temp_dir = TempDir::new().expect("failed to create temp dir");

        // Create a keybindings.json with old key names
        let mut raw = serde_json::Map::new();
        raw.insert(
            "selectConfirm".into(),
            serde_json::Value::String("enter".into()),
        );
        raw.insert(
            "interrupt".into(),
            serde_json::Value::String("ctrl+x".into()),
        );
        let keybindings_path = temp_dir.path().join("keybindings.json");
        let content = serde_json::to_string_pretty(&serde_json::Value::Object(raw)).unwrap();
        fs::write(&keybindings_path, content).unwrap();

        let mgr = KeybindingsManager::create(temp_dir.path());

        let user_bindings = mgr.get_user_bindings();
        assert_eq!(
            user_bindings.get("tui.select.confirm"),
            Some(&serde_json::Value::String("enter".into()))
        );
        assert_eq!(
            user_bindings.get("app.interrupt"),
            Some(&serde_json::Value::String("ctrl+x".into()))
        );

        let effective = mgr.get_effective_config();
        assert_eq!(
            effective.get("tui.select.confirm"),
            Some(&serde_json::Value::String("enter".into()))
        );
        assert_eq!(
            effective.get("app.interrupt"),
            Some(&serde_json::Value::String("ctrl+x".into()))
        );
    }
}

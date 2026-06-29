/// Keybinding management. Port of src/keybindings.ts.
use crate::keys::{matches_key, KeyId};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Type alias for a keybinding action name.
pub type Keybinding = String;

/// Map of named actions to bound key sequences.
pub type KeybindingsConfig = HashMap<String, Vec<KeyId>>;

/// A single keybinding definition.
#[derive(Debug, Clone)]
pub struct KeybindingDefinition {
    pub description: String,
    pub keys: Vec<KeyId>,
}

/// Collection of keybinding definitions.
pub type KeybindingDefinitions = HashMap<String, KeybindingDefinition>;

/// Resolved keybinding map.
pub type Keybindings = HashMap<String, Vec<KeyId>>;

/// Info about a direct user-binding conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindingConflict {
    pub key: KeyId,
    pub keybindings: Vec<String>,
}

/// Default TUI keybindings. Names and defaults match the TypeScript source.
pub static TUI_KEYBINDINGS: std::sync::LazyLock<KeybindingDefinitions> =
    std::sync::LazyLock::new(|| {
        let mut defs = HashMap::new();
        macro_rules! def {
            ($name:literal, [$($key:literal),+], $desc:literal) => {
                defs.insert($name.into(), KeybindingDefinition {
                    description: $desc.into(),
                    keys: vec![$($key),+],
                });
            };
            ($name:literal, $key:literal, $desc:literal) => {
                defs.insert($name.into(), KeybindingDefinition {
                    description: $desc.into(),
                    keys: vec![$key],
                });
            };
        }

        def!("tui.editor.cursorUp", "up", "Move cursor up");
        def!("tui.editor.cursorDown", "down", "Move cursor down");
        def!(
            "tui.editor.cursorLeft",
            ["left", "ctrl+b"],
            "Move cursor left"
        );
        def!(
            "tui.editor.cursorRight",
            ["right", "ctrl+f"],
            "Move cursor right"
        );
        def!(
            "tui.editor.cursorWordLeft",
            ["alt+left", "ctrl+left", "alt+b"],
            "Move cursor word left"
        );
        def!(
            "tui.editor.cursorWordRight",
            ["alt+right", "ctrl+right", "alt+f"],
            "Move cursor word right"
        );
        def!(
            "tui.editor.cursorLineStart",
            ["home", "ctrl+a"],
            "Move to line start"
        );
        def!(
            "tui.editor.cursorLineEnd",
            ["end", "ctrl+e"],
            "Move to line end"
        );
        def!(
            "tui.editor.jumpForward",
            "ctrl+]",
            "Jump forward to character"
        );
        def!(
            "tui.editor.jumpBackward",
            "ctrl+alt+]",
            "Jump backward to character"
        );
        def!("tui.editor.pageUp", "pageUp", "Page up");
        def!("tui.editor.pageDown", "pageDown", "Page down");
        def!(
            "tui.editor.deleteCharBackward",
            "backspace",
            "Delete character backward"
        );
        def!(
            "tui.editor.deleteCharForward",
            ["delete", "ctrl+d"],
            "Delete character forward"
        );
        def!(
            "tui.editor.deleteWordBackward",
            ["ctrl+w", "alt+backspace"],
            "Delete word backward"
        );
        def!(
            "tui.editor.deleteWordForward",
            ["alt+d", "alt+delete"],
            "Delete word forward"
        );
        def!(
            "tui.editor.deleteToLineStart",
            "ctrl+u",
            "Delete to line start"
        );
        def!("tui.editor.deleteToLineEnd", "ctrl+k", "Delete to line end");
        def!("tui.editor.yank", "ctrl+y", "Yank");
        def!("tui.editor.yankPop", "alt+y", "Yank pop");
        def!("tui.editor.undo", "ctrl+-", "Undo");
        def!(
            "tui.input.newLine",
            ["shift+enter", "ctrl+j"],
            "Insert newline"
        );
        def!("tui.input.submit", "enter", "Submit input");
        def!("tui.input.tab", "tab", "Tab / autocomplete");
        def!("tui.input.copy", "ctrl+c", "Copy selection");
        def!("tui.select.up", "up", "Move selection up");
        def!("tui.select.down", "down", "Move selection down");
        def!("tui.select.pageUp", "pageUp", "Selection page up");
        def!("tui.select.pageDown", "pageDown", "Selection page down");
        def!("tui.select.confirm", "enter", "Confirm selection");
        def!(
            "tui.select.cancel",
            ["escape", "ctrl+c"],
            "Cancel selection"
        );
        defs
    });

/// Manages keybinding resolution and direct user-binding conflict detection.
#[derive(Clone)]
pub struct KeybindingsManager {
    definitions: KeybindingDefinitions,
    user_bindings: KeybindingsConfig,
    keys_by_id: Keybindings,
    conflicts: Vec<KeybindingConflict>,
}

impl KeybindingsManager {
    pub fn new(definitions: KeybindingDefinitions) -> Self {
        let mut manager = KeybindingsManager {
            definitions,
            user_bindings: HashMap::new(),
            keys_by_id: HashMap::new(),
            conflicts: Vec::new(),
        };
        manager.rebuild();
        manager
    }

    pub fn with_user_bindings(
        definitions: KeybindingDefinitions,
        user_bindings: KeybindingsConfig,
    ) -> Self {
        let mut manager = KeybindingsManager {
            definitions,
            user_bindings,
            keys_by_id: HashMap::new(),
            conflicts: Vec::new(),
        };
        manager.rebuild();
        manager
    }

    fn normalize_keys(keys: &[KeyId]) -> Vec<KeyId> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for key in keys {
            if seen.insert(*key) {
                out.push(*key);
            }
        }
        out
    }

    fn rebuild(&mut self) {
        self.keys_by_id.clear();
        self.conflicts.clear();

        let mut user_claims: HashMap<KeyId, Vec<String>> = HashMap::new();
        for (keybinding, keys) in &self.user_bindings {
            if !self.definitions.contains_key(keybinding) {
                continue;
            }
            for key in Self::normalize_keys(keys) {
                let claimants = user_claims.entry(key).or_default();
                if !claimants.iter().any(|k| k == keybinding) {
                    claimants.push(keybinding.clone());
                }
            }
        }

        for (key, mut keybindings) in user_claims {
            if keybindings.len() > 1 {
                keybindings.sort();
                self.conflicts.push(KeybindingConflict { key, keybindings });
            }
        }
        self.conflicts.sort_by(|a, b| a.key.cmp(b.key));

        for (id, definition) in &self.definitions {
            let keys = self
                .user_bindings
                .get(id)
                .map(|keys| Self::normalize_keys(keys))
                .unwrap_or_else(|| Self::normalize_keys(&definition.keys));
            self.keys_by_id.insert(id.clone(), keys);
        }
    }

    /// Check if input data matches a specific action.
    pub fn matches(&self, data: &str, action: &str) -> bool {
        self.keys_by_id
            .get(action)
            .is_some_and(|keys| keys.iter().any(|k| matches_key(data, k)))
    }

    /// Find which action (if any) matches the input data.
    pub fn find_action(&self, data: &str) -> Option<&str> {
        for (action, keys) in &self.keys_by_id {
            if keys.iter().any(|k| matches_key(data, k)) {
                return Some(action);
            }
        }
        None
    }

    /// Backward-compatible direct binding setter.
    pub fn set_binding(&mut self, action: &str, keys: Vec<KeyId>) {
        self.user_bindings.insert(action.to_string(), keys);
        self.rebuild();
    }

    pub fn set_user_bindings<I>(&mut self, user_bindings: I)
    where
        I: IntoIterator<Item = (&'static str, Vec<KeyId>)>,
    {
        self.user_bindings = user_bindings
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        self.rebuild();
    }

    pub fn get_user_bindings(&self) -> KeybindingsConfig {
        self.user_bindings.clone()
    }

    pub fn get_resolved_bindings(&self) -> KeybindingsConfig {
        self.keys_by_id.clone()
    }

    pub fn get_keys(&self, keybinding: &str) -> Vec<KeyId> {
        self.keys_by_id.get(keybinding).cloned().unwrap_or_default()
    }

    pub fn get_definition(&self, keybinding: &str) -> Option<&KeybindingDefinition> {
        self.definitions.get(keybinding)
    }

    pub fn get_conflicts(&self) -> Vec<KeybindingConflict> {
        self.conflicts.clone()
    }

    pub fn check_conflicts(&self) -> Vec<KeybindingConflict> {
        self.get_conflicts()
    }

    pub fn get_bindings(&self) -> &Keybindings {
        &self.keys_by_id
    }
}

static KEYBINDINGS_INSTANCE: Mutex<Option<KeybindingsManager>> = Mutex::new(None);

fn lock_or_recover<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn set_keybindings(manager: KeybindingsManager) {
    *lock_or_recover(&KEYBINDINGS_INSTANCE) = Some(manager);
}

pub fn get_keybindings() -> KeybindingsManager {
    let mut guard = lock_or_recover(&KEYBINDINGS_INSTANCE);
    if guard.is_none() {
        *guard = Some(KeybindingsManager::new(TUI_KEYBINDINGS.clone()));
    }
    guard.clone().unwrap()
}

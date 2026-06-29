//! TUI component for managing package resources (enable/disable).
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/config-selector.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::raw_key_hint;
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Input, Spacer, visible_width,
};
use crate::modes::interactive::theme::theme::theme;

/// Type of resource being managed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Extensions,
    Skills,
    Prompts,
    Themes,
}

impl ResourceType {
    pub fn label(&self) -> &'static str {
        match self {
            ResourceType::Extensions => "Extensions",
            ResourceType::Skills => "Skills",
            ResourceType::Prompts => "Prompts",
            ResourceType::Themes => "Themes",
        }
    }

    pub fn type_order(&self) -> u8 {
        match self {
            ResourceType::Extensions => 0,
            ResourceType::Skills => 1,
            ResourceType::Prompts => 2,
            ResourceType::Themes => 3,
        }
    }
}

/// Metadata about a resource's origin and scope.
#[derive(Debug, Clone)]
pub struct PathMetadata {
    pub origin: String, // "package" or "top-level"
    pub scope: String,  // "user" or "project"
    pub source: String, // package source identifier
    pub base_dir: Option<String>,
}

/// A resolved resource entry.
#[derive(Debug, Clone)]
pub struct ResourceItem {
    pub path: String,
    pub enabled: bool,
    pub metadata: PathMetadata,
    pub resource_type: ResourceType,
    pub display_name: String,
    pub group_key: String,
    pub subgroup_key: String,
}

/// A group of resources within a subgroup.
#[derive(Debug, Clone)]
pub struct ResourceSubgroup {
    pub type_: ResourceType,
    pub label: String,
    pub items: Vec<ResourceItem>,
}

/// A top-level resource group (by origin + scope + source).
#[derive(Debug, Clone)]
pub struct ResourceGroup {
    pub key: String,
    pub label: String,
    pub scope: String,
    pub origin: String,
    pub source: String,
    pub subgroups: Vec<ResourceSubgroup>,
}

/// A flat entry in the filtered display list.
#[derive(Debug, Clone)]
pub enum FlatEntry {
    Group(ResourceGroup),
    Subgroup(ResourceSubgroup),
    Item(ResourceItem),
}

/// Config selector header displaying title and hints.
struct ConfigSelectorHeader;

impl Component for ConfigSelectorHeader {
    fn render(&self, width: u16) -> Vec<String> {
        let title = theme().bold("Resource Configuration");
        let sep = theme().fg("muted", " · ");
        let hint = format!(
            "{}{}{}{}",
            raw_key_hint("space", "toggle"),
            sep,
            raw_key_hint("esc", "close"),
            ""
        );
        let hint_width = visible_width(&hint);
        let title_width = visible_width(&title);
        let spacing = ((width as i32) - (title_width as i32) - (hint_width as i32)).max(1) as usize;

        vec![
            format!("{}{}{}", title, " ".repeat(spacing), hint),
            theme().fg("muted", "Type to filter resources"),
        ]
    }

    fn invalidate(&mut self) {}
}

/// Config selector component for managing package resources.
pub struct ConfigSelectorComponent {
    container: Container,
    /// Resource groups built from resolved paths.
    _groups: Vec<ResourceGroup>,
    /// Flat list of all entries.
    _flat_items: Vec<FlatEntry>,
    /// Filtered entries for display.
    _filtered_items: Vec<FlatEntry>,
    selected_index: usize,
    search_input: Input,
    max_visible: usize,
}

impl ConfigSelectorComponent {
    pub fn new(
        _resolved_paths: &serde_json::Value, // TODO: use real ResolvedPaths type
        _cwd: &str,
        _agent_dir: &str,
        terminal_height: Option<u16>,
    ) -> Self {
        let chrome = 8;
        let max_visible = ((terminal_height.unwrap_or(24) as usize).saturating_sub(chrome)).max(5);

        let mut container = Container::new();

        // Layout
        container.add_child(Box::new(Spacer::new(1)));
        container.add_child(Box::new(DynamicBorder::new(None)));
        container.add_child(Box::new(Spacer::new(1)));
        container.add_child(Box::new(ConfigSelectorHeader));
        container.add_child(Box::new(Spacer::new(1)));

        // Search input + list (placeholder)
        container.add_child(Box::new(Container::new()));

        container.add_child(Box::new(Spacer::new(1)));
        container.add_child(Box::new(DynamicBorder::new(None)));

        Self {
            container,
            _groups: Vec::new(),
            _flat_items: Vec::new(),
            _filtered_items: Vec::new(),
            selected_index: 0,
            search_input: Input::new(),
            max_visible,
        }
    }

    /// Handle input navigation. Returns true if event was consumed.
    pub fn handle_input(&mut self, _key_data: &str) -> bool {
        // Stub: full keybinding matching in real TUI integration
        false
    }
}

impl Component for ConfigSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.container.render(width)
    }

    fn invalidate(&mut self) {
        self.container.invalidate();
    }
}

//! Port of `packages/coding-agent/src/modes/interactive/components/tree-selector.ts`.
//!
//! Tree list component with selection and ASCII art visualization.
//! Component that renders a session tree selector for navigation.

use std::collections::{HashMap, HashSet};

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::keybinding_hints::key_hint;
use crate::modes::interactive::components::tui_shim::{
    Component, Focusable, Input, Spacer, Text, get_keybindings, truncate_to_width, visible_width,
};
use crate::modes::interactive::theme::theme::theme;

// ============================================================================
// Types
// ============================================================================

/// Filter mode for tree display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Default,
    NoTools,
    UserOnly,
    LabeledOnly,
    All,
}

impl FilterMode {
    fn status_label(&self) -> &'static str {
        match self {
            FilterMode::Default => "",
            FilterMode::NoTools => " [no-tools]",
            FilterMode::UserOnly => " [user]",
            FilterMode::LabeledOnly => " [labeled]",
            FilterMode::All => " [all]",
        }
    }
}

/// A session entry as stored in the session tree.
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub entry_type: String,
    pub role: Option<String>,
    pub content_preview: String,
    pub tool_call_display: Option<String>,
    pub stop_reason: Option<String>,
    pub error_message: Option<String>,
    pub model_id: Option<String>,
    pub thinking_level: Option<String>,
    pub custom_type: Option<String>,
    pub summary: Option<String>,
    pub tokens_before: Option<f64>,
    pub name: Option<String>,
    pub label_value: Option<String>,
    pub timestamp: String,
}

/// Tree node for getTree() - defensive copy of session structure.
#[derive(Debug, Clone)]
pub struct SessionTreeNode {
    pub entry: SessionEntry,
    pub children: Vec<SessionTreeNode>,
    pub label: Option<String>,
    pub label_timestamp: Option<String>,
}

/// Gutter info: position and whether to show │.
#[derive(Debug, Clone)]
struct GutterInfo {
    position: usize,
    show: bool,
}

/// Flattened tree node for navigation.
#[derive(Debug, Clone)]
struct FlatNode {
    node: SessionTreeNode,
    indent: usize,
    show_connector: bool,
    is_last: bool,
    gutters: Vec<GutterInfo>,
    is_virtual_root_child: bool,
}

/// Tool call info for lookup.
#[derive(Debug, Clone)]
struct ToolCallInfo {
    name: String,
    _arguments: HashMap<String, String>,
}

const TREE_GUTTER_WIDTH: u16 = 2;
const MIN_VISIBLE_ANCHOR_CONTENT_WIDTH: u16 = 4;
const MAX_VISIBLE_ANCHOR_CONTENT_WIDTH: u16 = 20;
const MIN_ANCHOR_CONTEXT_WIDTH: u16 = 2;
const MAX_ANCHOR_CONTEXT_WIDTH: u16 = 12;

// ============================================================================
// Horizontal viewport
// ============================================================================

struct HorizontalViewportRow {
    gutter: String,
    body: String,
    anchor_col: u16,
    body_width: u16,
    is_selected: bool,
}

fn render_horizontal_viewport(rows: &[HorizontalViewportRow], width: u16) -> Vec<String> {
    let viewport_width = (width.saturating_sub(TREE_GUTTER_WIDTH) as i32).max(0) as u16;
    let max_body_width = rows.iter().map(|r| r.body_width).max().unwrap_or(0);
    let max_horizontal_scroll: i32 = max_body_width as i32 - viewport_width as i32;
    let max_horizontal_scroll = max_horizontal_scroll.max(0) as u16;

    let selected_row = rows.iter().find(|r| r.is_selected);

    let mut horizontal_scroll = 0u16;
    if let Some(sel) = selected_row {
        if max_horizontal_scroll > 0 {
            let min_visible_anchor_content_width = MAX_VISIBLE_ANCHOR_CONTENT_WIDTH
                .min(viewport_width / 3)
                .max(MIN_VISIBLE_ANCHOR_CONTENT_WIDTH);
            if sel.anchor_col > viewport_width.saturating_sub(min_visible_anchor_content_width) {
                let anchor_context_width = MAX_ANCHOR_CONTEXT_WIDTH
                    .min(viewport_width / 4)
                    .max(MIN_ANCHOR_CONTEXT_WIDTH);
                horizontal_scroll =
                    max_horizontal_scroll.min(sel.anchor_col.saturating_sub(anchor_context_width));
            }
        }
    }

    rows.iter()
        .map(|row| {
            if horizontal_scroll > 0 {
                // Simple truncation for stub - real impl uses sliceByColumn
                let start = horizontal_scroll as usize;
                let end = (horizontal_scroll + viewport_width) as usize;
                let end = end.min(row.body.len());
                let body_part = if start < row.body.len() {
                    &row.body[start..end]
                } else {
                    ""
                };
                let line = format!("{}{}\x1b[0m", row.gutter, body_part);
                truncate_to_width(&line, width, "")
            } else {
                let line = format!("{}{}", row.gutter, row.body);
                truncate_to_width(&line, width, "")
            }
        })
        .collect()
}

// ============================================================================
// TreeList
// ============================================================================

/// Tree list component with selection and ASCII art visualization.
pub struct TreeList {
    flat_nodes: Vec<FlatNode>,
    filtered_nodes: Vec<FlatNode>,
    selected_index: usize,
    current_leaf_id: Option<String>,
    max_visible_lines: usize,
    filter_mode: FilterMode,
    search_query: String,
    tool_call_map: HashMap<String, ToolCallInfo>,
    multiple_roots: bool,
    show_label_timestamps: bool,
    active_path_ids: HashSet<String>,
    visible_parent_map: HashMap<String, Option<String>>,
    visible_children_map: HashMap<Option<String>, Vec<String>>,
    last_selected_id: Option<String>,
    folded_nodes: HashSet<String>,

    pub on_select: Option<Box<dyn Fn(String) + Send + Sync>>,
    pub on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_label_edit: Option<Box<dyn Fn(String, Option<String>) + Send + Sync>>,
}

impl TreeList {
    pub fn new(
        tree: Vec<SessionTreeNode>,
        current_leaf_id: Option<String>,
        max_visible_lines: usize,
        initial_selected_id: Option<String>,
        initial_filter_mode: Option<FilterMode>,
    ) -> Self {
        let filter_mode = initial_filter_mode.unwrap_or(FilterMode::Default);
        let multiple_roots = tree.len() > 1;

        let mut tree_list = TreeList {
            flat_nodes: Vec::new(),
            filtered_nodes: Vec::new(),
            selected_index: 0,
            current_leaf_id: current_leaf_id.clone(),
            max_visible_lines,
            filter_mode,
            search_query: String::new(),
            tool_call_map: HashMap::new(),
            multiple_roots,
            show_label_timestamps: false,
            active_path_ids: HashSet::new(),
            visible_parent_map: HashMap::new(),
            visible_children_map: HashMap::new(),
            last_selected_id: None,
            folded_nodes: HashSet::new(),
            on_select: None,
            on_cancel: None,
            on_label_edit: None,
        };

        tree_list.flat_nodes = tree_list.flatten_tree(&tree);
        tree_list.build_active_path();
        tree_list.apply_filter();

        let target_id = initial_selected_id.or(current_leaf_id);
        tree_list.selected_index = tree_list.find_nearest_visible_index(target_id.as_deref());
        tree_list.last_selected_id = tree_list
            .filtered_nodes
            .get(tree_list.selected_index)
            .map(|n| n.node.entry.id.clone());

        tree_list
    }

    // -----------------------------------------------------------------------
    // Flatten tree into navigable nodes
    // -----------------------------------------------------------------------

    fn flatten_tree(&mut self, roots: &[SessionTreeNode]) -> Vec<FlatNode> {
        let mut result: Vec<FlatNode> = Vec::new();
        self.tool_call_map.clear();

        let contains_active = self.compute_contains_active(roots);
        let multiple_roots = roots.len() > 1;

        let mut ordered_roots: Vec<&SessionTreeNode> = roots.iter().collect();
        ordered_roots.sort_by(|a, b| {
            let a_active = contains_active.get(&a.entry.id).copied().unwrap_or(false);
            let b_active = contains_active.get(&b.entry.id).copied().unwrap_or(false);
            b_active.cmp(&a_active)
        });

        type StackItem<'a> = (
            &'a SessionTreeNode,
            usize,
            bool,
            bool,
            bool,
            Vec<GutterInfo>,
            bool,
        );
        let mut stack: Vec<StackItem> = Vec::new();

        for i in (0..ordered_roots.len()).rev() {
            let is_last = i == ordered_roots.len() - 1;
            stack.push((
                ordered_roots[i],
                if multiple_roots { 1 } else { 0 },
                multiple_roots,
                multiple_roots,
                is_last,
                Vec::new(),
                multiple_roots,
            ));
        }

        while let Some((
            node,
            indent,
            just_branched,
            show_connector,
            is_last,
            gutters,
            is_virtual_root_child,
        )) = stack.pop()
        {
            result.push(FlatNode {
                node: node.clone(),
                indent,
                show_connector,
                is_last,
                gutters: gutters.clone(),
                is_virtual_root_child,
            });

            let children = &node.children;
            let multiple_children = children.len() > 1;

            let mut ordered_children: Vec<&SessionTreeNode> = children.iter().collect();
            ordered_children.sort_by(|a, b| {
                let a_active = contains_active.get(&a.entry.id).copied().unwrap_or(false);
                let b_active = contains_active.get(&b.entry.id).copied().unwrap_or(false);
                b_active.cmp(&a_active)
            });

            let child_indent = if multiple_children {
                indent + 1
            } else if just_branched && indent > 0 {
                indent + 1
            } else {
                indent
            };

            let connector_displayed = show_connector && !is_virtual_root_child;
            let current_display_indent = if self.multiple_roots {
                indent.saturating_sub(1).max(0)
            } else {
                indent
            };
            let connector_position = current_display_indent.saturating_sub(1).max(0);
            let child_gutters: Vec<GutterInfo> = if connector_displayed {
                let mut g = gutters.clone();
                g.push(GutterInfo {
                    position: connector_position,
                    show: !is_last,
                });
                g
            } else {
                gutters.clone()
            };

            for i in (0..ordered_children.len()).rev() {
                let child_is_last = i == ordered_children.len() - 1;
                stack.push((
                    ordered_children[i],
                    child_indent,
                    multiple_children,
                    multiple_children,
                    child_is_last,
                    child_gutters.clone(),
                    false,
                ));
            }
        }

        result
    }

    fn compute_contains_active(&self, roots: &[SessionTreeNode]) -> HashMap<String, bool> {
        let mut result = HashMap::new();
        let leaf_id = &self.current_leaf_id;

        let mut all_nodes: Vec<&SessionTreeNode> = Vec::new();
        let mut stack: Vec<&SessionTreeNode> = roots.iter().collect();
        while let Some(node) = stack.pop() {
            all_nodes.push(node);
            for child in node.children.iter().rev() {
                stack.push(child);
            }
        }

        for node in all_nodes.iter().rev() {
            let mut has = leaf_id.as_ref().map_or(false, |id| node.entry.id == *id);
            for child in &node.children {
                if result.get(&child.entry.id).copied().unwrap_or(false) {
                    has = true;
                }
            }
            result.insert(node.entry.id.clone(), has);
        }

        result
    }

    fn build_active_path(&mut self) {
        self.active_path_ids.clear();
        let leaf_id = match &self.current_leaf_id {
            Some(id) => id.clone(),
            None => return,
        };

        let entry_map: HashMap<String, &FlatNode> = self
            .flat_nodes
            .iter()
            .map(|n| (n.node.entry.id.clone(), n))
            .collect();

        let mut current_id = Some(leaf_id);
        while let Some(id) = current_id {
            self.active_path_ids.insert(id.clone());
            current_id = entry_map
                .get(&id)
                .and_then(|n| n.node.entry.parent_id.clone());
        }
    }

    // -----------------------------------------------------------------------
    // Filtering
    // -----------------------------------------------------------------------

    fn apply_filter(&mut self) {
        if !self.filtered_nodes.is_empty() {
            self.last_selected_id = self
                .filtered_nodes
                .get(self.selected_index)
                .map(|n| n.node.entry.id.clone())
                .or(self.last_selected_id.clone());
        }

        let search_tokens: Vec<String> = self
            .search_query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();

        self.filtered_nodes = self
            .flat_nodes
            .iter()
            .filter(|flat_node| {
                let entry = &flat_node.node.entry;
                let is_current_leaf = self
                    .current_leaf_id
                    .as_ref()
                    .map_or(false, |id| entry.id == *id);

                if entry.entry_type == "message"
                    && entry.role.as_deref() == Some("assistant")
                    && !is_current_leaf
                {
                    let has_text = !entry.content_preview.trim().is_empty();
                    let is_error_or_aborted = entry
                        .stop_reason
                        .as_deref()
                        .map_or(false, |s| s != "stop" && s != "toolUse");
                    if !has_text && !is_error_or_aborted {
                        return false;
                    }
                }

                let is_settings_entry = matches!(
                    entry.entry_type.as_str(),
                    "label" | "custom" | "model_change" | "thinking_level_change" | "session_info"
                );

                let passes_filter = match self.filter_mode {
                    FilterMode::UserOnly => {
                        entry.entry_type == "message" && entry.role.as_deref() == Some("user")
                    }
                    FilterMode::NoTools => {
                        !is_settings_entry
                            && !(entry.entry_type == "message"
                                && entry.role.as_deref() == Some("toolResult"))
                    }
                    FilterMode::LabeledOnly => flat_node.node.label.is_some(),
                    FilterMode::All => true,
                    FilterMode::Default => !is_settings_entry,
                };

                if !passes_filter {
                    return false;
                }

                if !search_tokens.is_empty() {
                    let node_text = self.get_searchable_text(&flat_node.node).to_lowercase();
                    return search_tokens.iter().all(|token| node_text.contains(token));
                }

                true
            })
            .cloned()
            .collect();

        if !self.folded_nodes.is_empty() {
            let mut skip_set: HashSet<String> = HashSet::new();
            for flat_node in &self.flat_nodes {
                let id = &flat_node.node.entry.id;
                if let Some(pid) = &flat_node.node.entry.parent_id {
                    if self.folded_nodes.contains(pid) || skip_set.contains(pid) {
                        skip_set.insert(id.clone());
                    }
                }
            }
            self.filtered_nodes
                .retain(|n| !skip_set.contains(&n.node.entry.id));
        }

        self.recalculate_visual_structure();

        if let Some(ref last_id) = self.last_selected_id.clone() {
            self.selected_index = self.find_nearest_visible_index(Some(last_id));
        } else {
            self.selected_index = self
                .selected_index
                .min(self.filtered_nodes.len().saturating_sub(1));
        }

        if !self.filtered_nodes.is_empty() {
            self.last_selected_id = self
                .filtered_nodes
                .get(self.selected_index)
                .map(|n| n.node.entry.id.clone());
        }
    }

    fn recalculate_visual_structure(&mut self) {
        if self.filtered_nodes.is_empty() {
            return;
        }

        let visible_ids: HashSet<String> = self
            .filtered_nodes
            .iter()
            .map(|n| n.node.entry.id.clone())
            .collect();

        let entry_map: HashMap<String, &FlatNode> = self
            .flat_nodes
            .iter()
            .map(|n| (n.node.entry.id.clone(), n))
            .collect();

        let find_visible_ancestor = |node_id: &str| -> Option<String> {
            let mut current_id = entry_map.get(node_id)?.node.entry.parent_id.clone();
            while let Some(ref id) = current_id {
                if visible_ids.contains(id) {
                    return Some(id.clone());
                }
                current_id = entry_map.get(id)?.node.entry.parent_id.clone();
            }
            None
        };

        let mut visible_parent: HashMap<String, Option<String>> = HashMap::new();
        let mut visible_children: HashMap<Option<String>, Vec<String>> = HashMap::new();
        visible_children.insert(None, Vec::new());

        for flat_node in &self.filtered_nodes {
            let node_id = &flat_node.node.entry.id;
            let ancestor_id = find_visible_ancestor(node_id);
            visible_parent.insert(node_id.clone(), ancestor_id.clone());
            visible_children
                .entry(ancestor_id)
                .or_default()
                .push(node_id.clone());
        }

        let visible_root_ids = visible_children.get(&None).cloned().unwrap_or_default();
        self.multiple_roots = visible_root_ids.len() > 1;

        let mut new_nodes: Vec<FlatNode> = self.filtered_nodes.clone();
        let mut filtered_node_map: HashMap<String, usize> = HashMap::new();
        for (i, n) in self.filtered_nodes.iter().enumerate() {
            filtered_node_map.insert(n.node.entry.id.clone(), i);
        }

        type StackItem = (String, usize, bool, bool, bool, Vec<GutterInfo>, bool);
        let mut stack: Vec<StackItem> = Vec::new();

        for i in (0..visible_root_ids.len()).rev() {
            let is_last = i == visible_root_ids.len() - 1;
            stack.push((
                visible_root_ids[i].clone(),
                if self.multiple_roots { 1 } else { 0 },
                self.multiple_roots,
                self.multiple_roots,
                is_last,
                Vec::new(),
                self.multiple_roots,
            ));
        }

        while let Some((
            node_id,
            indent,
            just_branched,
            show_connector,
            is_last,
            gutters,
            is_virtual_root_child,
        )) = stack.pop()
        {
            if let Some(&idx) = filtered_node_map.get(&node_id) {
                let node = &mut new_nodes[idx];
                node.indent = indent;
                node.show_connector = show_connector;
                node.is_last = is_last;
                node.gutters = gutters.clone();
                node.is_virtual_root_child = is_virtual_root_child;
            }

            let children = visible_children
                .get(&Some(node_id.clone()))
                .cloned()
                .unwrap_or_default();
            let multiple_children = children.len() > 1;

            let child_indent = if multiple_children {
                indent + 1
            } else if just_branched && indent > 0 {
                indent + 1
            } else {
                indent
            };

            let connector_displayed = show_connector && !is_virtual_root_child;
            let current_display_indent = if self.multiple_roots {
                indent.saturating_sub(1).max(0)
            } else {
                indent
            };
            let connector_position = current_display_indent.saturating_sub(1).max(0);
            let child_gutters = if connector_displayed {
                let mut g = gutters.clone();
                g.push(GutterInfo {
                    position: connector_position,
                    show: !is_last,
                });
                g
            } else {
                gutters.clone()
            };

            for i in (0..children.len()).rev() {
                let child_is_last = i == children.len() - 1;
                stack.push((
                    children[i].clone(),
                    child_indent,
                    multiple_children,
                    multiple_children,
                    child_is_last,
                    child_gutters.clone(),
                    false,
                ));
            }
        }

        self.filtered_nodes = new_nodes;
        self.visible_parent_map = visible_parent;
        self.visible_children_map = visible_children;
    }

    fn find_nearest_visible_index(&self, entry_id: Option<&str>) -> usize {
        if self.filtered_nodes.is_empty() {
            return 0;
        }

        let entry_id = match entry_id {
            Some(id) => id,
            None => return self.filtered_nodes.len().saturating_sub(1),
        };

        let entry_map: HashMap<String, &FlatNode> = self
            .flat_nodes
            .iter()
            .map(|n| (n.node.entry.id.clone(), n))
            .collect();

        let visible_id_to_index: HashMap<String, usize> = self
            .filtered_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.node.entry.id.clone(), i))
            .collect();

        let mut current_id = Some(entry_id.to_string());
        while let Some(ref id) = current_id {
            if let Some(&idx) = visible_id_to_index.get(id) {
                return idx;
            }
            current_id = entry_map
                .get(id)
                .and_then(|n| n.node.entry.parent_id.clone());
        }

        self.filtered_nodes.len().saturating_sub(1)
    }

    fn get_searchable_text(&self, node: &SessionTreeNode) -> String {
        let entry = &node.entry;
        let mut parts: Vec<String> = Vec::new();

        if let Some(ref label) = node.label {
            parts.push(label.clone());
        }

        match entry.entry_type.as_str() {
            "message" => {
                if let Some(ref role) = entry.role {
                    parts.push(role.clone());
                }
                parts.push(entry.content_preview.clone());
            }
            "custom_message" => {
                if let Some(ref ct) = entry.custom_type {
                    parts.push(ct.clone());
                }
                parts.push(entry.content_preview.clone());
            }
            "compaction" => parts.push("compaction".to_string()),
            "branch_summary" => {
                parts.push("branch summary".to_string());
                if let Some(ref s) = entry.summary {
                    parts.push(s.clone());
                }
            }
            "session_info" => {
                parts.push("title".to_string());
                if let Some(ref name) = entry.name {
                    parts.push(name.clone());
                }
            }
            "model_change" => {
                parts.push("model".to_string());
                if let Some(ref mid) = entry.model_id {
                    parts.push(mid.clone());
                }
            }
            "thinking_level_change" => {
                parts.push("thinking".to_string());
                if let Some(ref tl) = entry.thinking_level {
                    parts.push(tl.clone());
                }
            }
            "custom" => {
                parts.push("custom".to_string());
                if let Some(ref ct) = entry.custom_type {
                    parts.push(ct.clone());
                }
            }
            "label" => {
                parts.push("label".to_string());
                if let Some(ref lv) = entry.label_value {
                    parts.push(lv.clone());
                }
            }
            _ => {}
        }

        parts.join(" ")
    }

    pub fn get_search_query(&self) -> &str {
        &self.search_query
    }

    pub fn get_selected_node(&self) -> Option<&SessionTreeNode> {
        self.filtered_nodes
            .get(self.selected_index)
            .map(|n| &n.node)
    }

    pub fn update_node_label(
        &mut self,
        entry_id: &str,
        label: Option<String>,
        label_timestamp: Option<String>,
    ) {
        for flat_node in &mut self.flat_nodes {
            if flat_node.node.entry.id == entry_id {
                flat_node.node.label = label.clone();
                flat_node.node.label_timestamp = label.or(label_timestamp);
                break;
            }
        }
    }

    fn get_entry_display_text(&self, node: &SessionTreeNode) -> String {
        let entry = &node.entry;
        let normalize = |s: &str| s.replace(['\n', '\t'], " ").trim().to_string();

        match entry.entry_type.as_str() {
            "message" => {
                let role = entry.role.as_deref().unwrap_or("");
                match role {
                    "user" => format!("user: {}", normalize(&entry.content_preview)),
                    "assistant" => {
                        let text = normalize(&entry.content_preview);
                        if !text.is_empty() {
                            format!("assistant: {}", text)
                        } else if entry.stop_reason.as_deref() == Some("aborted") {
                            "assistant: (aborted)".to_string()
                        } else if let Some(ref err) = entry.error_message {
                            let short = normalize(err);
                            let short = if short.len() > 80 {
                                &short[..80]
                            } else {
                                &short
                            };
                            format!("assistant: {}", short)
                        } else {
                            "assistant: (no content)".to_string()
                        }
                    }
                    "toolResult" => entry
                        .tool_call_display
                        .clone()
                        .unwrap_or_else(|| "[tool]".to_string()),
                    _ => format!("[{}]", role),
                }
            }
            "custom_message" => {
                let ct = entry.custom_type.as_deref().unwrap_or("");
                format!("[{}]: {}", ct, normalize(&entry.content_preview))
            }
            "compaction" => {
                let tokens = (entry.tokens_before.unwrap_or(0.0) / 1000.0).round() as u64;
                format!("[compaction: {}k tokens]", tokens)
            }
            "branch_summary" => format!(
                "[branch summary]: {}",
                normalize(entry.summary.as_deref().unwrap_or(""))
            ),
            "model_change" => format!("[model: {}]", entry.model_id.as_deref().unwrap_or("")),
            "thinking_level_change" => format!(
                "[thinking: {}]",
                entry.thinking_level.as_deref().unwrap_or("")
            ),
            "custom" => format!("[custom: {}]", entry.custom_type.as_deref().unwrap_or("")),
            "label" => format!(
                "[label: {}]",
                entry.label_value.as_deref().unwrap_or("(cleared)")
            ),
            "session_info" => {
                if let Some(ref name) = entry.name {
                    format!("[title: {}]", name)
                } else {
                    "[title: empty]".to_string()
                }
            }
            _ => String::new(),
        }
    }

    fn is_foldable(&self, entry_id: &str) -> bool {
        let children = self.visible_children_map.get(&Some(entry_id.to_string()));
        if children.map_or(true, |c| c.is_empty()) {
            return false;
        }
        let parent_id = self.visible_parent_map.get(entry_id).cloned().flatten();
        if parent_id.is_none() {
            return true;
        }
        let siblings = self
            .visible_children_map
            .get(&parent_id)
            .map(|v| v.len())
            .unwrap_or(0);
        siblings > 1
    }

    fn find_branch_segment_start(&self, direction: &str) -> usize {
        let selected_id = match self.filtered_nodes.get(self.selected_index) {
            Some(n) => n.node.entry.id.clone(),
            None => return self.selected_index,
        };

        let index_by_entry_id: HashMap<String, usize> = self
            .filtered_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.node.entry.id.clone(), i))
            .collect();

        let mut current_id = selected_id;
        if direction == "down" {
            loop {
                let children = self
                    .visible_children_map
                    .get(&Some(current_id.clone()))
                    .cloned()
                    .unwrap_or_default();
                if children.is_empty() {
                    return *index_by_entry_id
                        .get(&current_id)
                        .unwrap_or(&self.selected_index);
                }
                if children.len() > 1 {
                    return *index_by_entry_id
                        .get(&children[0])
                        .unwrap_or(&self.selected_index);
                }
                current_id = children[0].clone();
            }
        }

        // direction == "up"
        loop {
            let parent_id = self.visible_parent_map.get(&current_id).cloned().flatten();
            match parent_id {
                None => {
                    return *index_by_entry_id
                        .get(&current_id)
                        .unwrap_or(&self.selected_index);
                }
                Some(pid) => {
                    let children = self
                        .visible_children_map
                        .get(&Some(pid.clone()))
                        .cloned()
                        .unwrap_or_default();
                    if children.len() > 1 {
                        let segment_start = *index_by_entry_id
                            .get(&current_id)
                            .unwrap_or(&self.selected_index);
                        if segment_start < self.selected_index {
                            return segment_start;
                        }
                    }
                    current_id = pid;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Input handling
    // -----------------------------------------------------------------------

    pub fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();

        if kb.matches(key_data, "tui.select.up") {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_nodes.len().saturating_sub(1)
            } else {
                self.selected_index - 1
            };
        } else if kb.matches(key_data, "tui.select.down") {
            self.selected_index =
                if self.selected_index >= self.filtered_nodes.len().saturating_sub(1) {
                    0
                } else {
                    self.selected_index + 1
                };
        } else if kb.matches(key_data, "app.tree.foldOrUp") {
            let current_id = self
                .filtered_nodes
                .get(self.selected_index)
                .map(|n| n.node.entry.id.clone());
            if let Some(ref id) = current_id {
                if self.is_foldable(id) && !self.folded_nodes.contains(id) {
                    self.folded_nodes.insert(id.clone());
                    self.apply_filter();
                    return;
                }
            }
            self.selected_index = self.find_branch_segment_start("up");
        } else if kb.matches(key_data, "app.tree.unfoldOrDown") {
            let current_id = self
                .filtered_nodes
                .get(self.selected_index)
                .map(|n| n.node.entry.id.clone());
            if let Some(ref id) = current_id {
                if self.folded_nodes.contains(id) {
                    self.folded_nodes.remove(id);
                    self.apply_filter();
                    return;
                }
            }
            self.selected_index = self.find_branch_segment_start("down");
        } else if kb.matches(key_data, "tui.editor.cursorLeft")
            || kb.matches(key_data, "tui.select.pageUp")
        {
            self.selected_index = self
                .selected_index
                .saturating_sub(self.max_visible_lines)
                .max(0);
        } else if kb.matches(key_data, "tui.editor.cursorRight")
            || kb.matches(key_data, "tui.select.pageDown")
        {
            self.selected_index = (self.selected_index + self.max_visible_lines)
                .min(self.filtered_nodes.len().saturating_sub(1));
        } else if kb.matches(key_data, "tui.select.confirm") {
            let selected_id = self
                .filtered_nodes
                .get(self.selected_index)
                .map(|n| n.node.entry.id.clone());
            if let Some(id) = selected_id {
                if let Some(ref cb) = self.on_select {
                    cb(id);
                }
            }
        } else if kb.matches(key_data, "tui.select.cancel") {
            if !self.search_query.is_empty() {
                self.search_query.clear();
                self.folded_nodes.clear();
                self.apply_filter();
            } else if let Some(ref cb) = self.on_cancel {
                cb();
            }
        } else if kb.matches(key_data, "app.tree.filter.default") {
            self.filter_mode = FilterMode::Default;
            self.folded_nodes.clear();
            self.apply_filter();
        } else if kb.matches(key_data, "app.tree.filter.noTools") {
            self.filter_mode = if self.filter_mode == FilterMode::NoTools {
                FilterMode::Default
            } else {
                FilterMode::NoTools
            };
            self.folded_nodes.clear();
            self.apply_filter();
        } else if kb.matches(key_data, "app.tree.filter.userOnly") {
            self.filter_mode = if self.filter_mode == FilterMode::UserOnly {
                FilterMode::Default
            } else {
                FilterMode::UserOnly
            };
            self.folded_nodes.clear();
            self.apply_filter();
        } else if kb.matches(key_data, "app.tree.filter.labeledOnly") {
            self.filter_mode = if self.filter_mode == FilterMode::LabeledOnly {
                FilterMode::Default
            } else {
                FilterMode::LabeledOnly
            };
            self.folded_nodes.clear();
            self.apply_filter();
        } else if kb.matches(key_data, "app.tree.filter.all") {
            self.filter_mode = if self.filter_mode == FilterMode::All {
                FilterMode::Default
            } else {
                FilterMode::All
            };
            self.folded_nodes.clear();
            self.apply_filter();
        } else if kb.matches(key_data, "app.tree.filter.cycleBackward") {
            let modes = [
                FilterMode::Default,
                FilterMode::NoTools,
                FilterMode::UserOnly,
                FilterMode::LabeledOnly,
                FilterMode::All,
            ];
            let current = modes
                .iter()
                .position(|m| *m == self.filter_mode)
                .unwrap_or(0);
            self.filter_mode = modes[(current + modes.len() - 1) % modes.len()];
            self.folded_nodes.clear();
            self.apply_filter();
        } else if kb.matches(key_data, "app.tree.filter.cycleForward") {
            let modes = [
                FilterMode::Default,
                FilterMode::NoTools,
                FilterMode::UserOnly,
                FilterMode::LabeledOnly,
                FilterMode::All,
            ];
            let current = modes
                .iter()
                .position(|m| *m == self.filter_mode)
                .unwrap_or(0);
            self.filter_mode = modes[(current + 1) % modes.len()];
            self.folded_nodes.clear();
            self.apply_filter();
        } else if kb.matches(key_data, "tui.editor.deleteCharBackward") {
            if !self.search_query.is_empty() {
                self.search_query.pop();
                self.folded_nodes.clear();
                self.apply_filter();
            }
        } else if kb.matches(key_data, "app.tree.editLabel") {
            let selected = self.filtered_nodes.get(self.selected_index).cloned();
            if let Some(node) = selected {
                if let Some(ref cb) = self.on_label_edit {
                    cb(node.node.entry.id.clone(), node.node.label.clone());
                }
            }
        } else if kb.matches(key_data, "app.tree.toggleLabelTimestamp") {
            self.show_label_timestamps = !self.show_label_timestamps;
        } else {
            let has_control_chars = key_data.chars().any(|ch| {
                let code = ch as u32;
                code < 32 || code == 0x7f || (code >= 0x80 && code <= 0x9f)
            });
            if !has_control_chars && !key_data.is_empty() {
                self.search_query.push_str(key_data);
                self.folded_nodes.clear();
                self.apply_filter();
            }
        }
    }
}

impl Component for TreeList {
    fn render(&self, _width: u16) -> Vec<String> {
        let t = theme();
        let mut lines: Vec<String> = Vec::new();

        if self.filtered_nodes.is_empty() {
            lines.push(t.fg("muted", "  No entries found"));
            lines.push(t.fg("muted", "  (0/0)"));
            return lines;
        }

        let start_index = 0usize.max(
            self.selected_index
                .saturating_sub(self.max_visible_lines / 2)
                .min(
                    self.filtered_nodes
                        .len()
                        .saturating_sub(self.max_visible_lines),
                ),
        );
        let end_index = (start_index + self.max_visible_lines).min(self.filtered_nodes.len());

        let mut rendered_rows: Vec<HorizontalViewportRow> = Vec::new();
        for i in start_index..end_index {
            let flat_node = &self.filtered_nodes[i];
            let entry = &flat_node.node.entry;
            let is_selected = i == self.selected_index;

            let cursor = if is_selected {
                t.fg("accent", "› ")
            } else {
                "  ".to_string()
            };

            let display_indent = if self.multiple_roots {
                flat_node.indent.saturating_sub(1).max(0)
            } else {
                flat_node.indent
            };

            let connector = if flat_node.show_connector && !flat_node.is_virtual_root_child {
                if flat_node.is_last {
                    "└─ "
                } else {
                    "├─ "
                }
            } else {
                ""
            };
            let connector_position: i32 = if !connector.is_empty() {
                (display_indent as i32) - 1
            } else {
                -1
            };

            let total_chars = display_indent * 3;
            let mut prefix_chars: Vec<char> = Vec::new();
            let is_folded = self.folded_nodes.contains(&entry.id);

            for col in 0..total_chars {
                let level = col / 3;
                let pos_in_level = col % 3;

                let gutter = flat_node.gutters.iter().find(|g| g.position == level);
                if let Some(g) = gutter {
                    if pos_in_level == 0 {
                        prefix_chars.push(if g.show { '│' } else { ' ' });
                    } else {
                        prefix_chars.push(' ');
                    }
                } else if connector_position == level as i32 {
                    if pos_in_level == 0 {
                        prefix_chars.push(if flat_node.is_last { '└' } else { '├' });
                    } else if pos_in_level == 1 {
                        let foldable = self.is_foldable(&entry.id);
                        prefix_chars.push(if is_folded {
                            '⊞'
                        } else if foldable {
                            '⊟'
                        } else {
                            '─'
                        });
                    } else {
                        prefix_chars.push(' ');
                    }
                } else {
                    prefix_chars.push(' ');
                }
            }
            let prefix: String = prefix_chars.iter().collect();

            let shows_fold_in_connector =
                flat_node.show_connector && !flat_node.is_virtual_root_child;
            let fold_marker = if is_folded && !shows_fold_in_connector {
                t.fg("accent", "⊞ ")
            } else {
                String::new()
            };

            let is_on_active_path = self.active_path_ids.contains(&entry.id);
            let path_marker = if is_on_active_path {
                t.fg("accent", "• ")
            } else {
                String::new()
            };

            let label = if let Some(ref l) = flat_node.node.label {
                t.fg("warning", &format!("[{}] ", l))
            } else {
                String::new()
            };

            let content = self.get_entry_display_text(&flat_node.node);
            let prefix_part = format!("{}{}{}", t.fg("dim", &prefix), fold_marker, path_marker);
            let anchor_col = visible_width(&prefix_part);

            let mut gutter = cursor;
            let mut body = format!("{}{}{}", prefix_part, label, content);

            if is_selected {
                gutter = t.bg("selectedBg", &gutter);
                body = t.bg("selectedBg", &body);
            }

            rendered_rows.push(HorizontalViewportRow {
                gutter,
                body,
                anchor_col,
                body_width: visible_width(&t.bg("", "")), // approximate
                is_selected,
            });
        }

        // Fix body_width for each row (can't compute inside the loop without borrowing issues)
        for row in &mut rendered_rows {
            row.body_width = visible_width(&row.body);
        }

        let viewport_width = _width;
        lines.extend(render_horizontal_viewport(&rendered_rows, viewport_width));

        let status_marker = self.filter_mode.status_label();
        lines.push(truncate_to_width(
            &t.fg(
                "muted",
                &format!(
                    "  ({}/{}){}",
                    self.selected_index + 1,
                    self.filtered_nodes.len(),
                    status_marker
                ),
            ),
            _width,
            "",
        ));

        lines
    }

    fn invalidate(&mut self) {}
}

// ============================================================================
// LabelInput stub
// ============================================================================

struct LabelInput {
    entry_id: String,
    input: Input,
    pub on_submit: Option<Box<dyn Fn(String, Option<String>) + Send + Sync>>,
    pub on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
    focused: bool,
}

impl LabelInput {
    fn new(entry_id: String, current_label: Option<&str>) -> Self {
        let mut input = Input::new();
        if let Some(label) = current_label {
            input.set_value(label);
        }
        LabelInput {
            entry_id,
            input,
            on_submit: None,
            on_cancel: None,
            focused: false,
        }
    }

    fn handle_input(&mut self, key_data: &str) {
        let kb = get_keybindings();
        if kb.matches(key_data, "tui.select.confirm") {
            let value = self.input.get_value().trim().to_string();
            if let Some(ref cb) = self.on_submit {
                cb(
                    self.entry_id.clone(),
                    if value.is_empty() { None } else { Some(value) },
                );
            }
        } else if kb.matches(key_data, "tui.select.cancel") {
            if let Some(ref cb) = self.on_cancel {
                cb();
            }
        }
    }
}

impl Component for LabelInput {
    fn render(&self, width: u16) -> Vec<String> {
        let t = theme();
        let indent = "  ";
        let available_width = (width as usize).saturating_sub(indent.len()) as u16;
        let mut lines: Vec<String> = Vec::new();
        lines.push(truncate_to_width(
            &format!("{}{}", indent, t.fg("muted", "Label (empty to remove):")),
            width,
            "",
        ));
        for line in self.input.render(available_width) {
            lines.push(truncate_to_width(&format!("{}{}", indent, line), width, ""));
        }
        lines.push(truncate_to_width(
            &format!(
                "{}{}  {}",
                indent,
                key_hint("tui.select.confirm", "save"),
                key_hint("tui.select.cancel", "cancel")
            ),
            width,
            "",
        ));
        lines
    }

    fn invalidate(&mut self) {}
}

impl Focusable for LabelInput {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.input.set_focused(focused);
    }
}

// ============================================================================
// TreeSelectorComponent
// ============================================================================

/// Component that renders a session tree selector for navigation.
pub struct TreeSelectorComponent {
    tree_list: TreeList,
    label_input: Option<LabelInput>,
    focused: bool,
    on_label_change: Option<Box<dyn Fn(String, Option<String>) + Send + Sync>>,
}

impl TreeSelectorComponent {
    pub fn new(
        tree: Vec<SessionTreeNode>,
        current_leaf_id: Option<String>,
        terminal_height: usize,
        on_select: Box<dyn Fn(String) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
        on_label_change: Option<Box<dyn Fn(String, Option<String>) + Send + Sync>>,
        initial_selected_id: Option<String>,
        initial_filter_mode: Option<FilterMode>,
    ) -> Self {
        let max_visible_lines = (terminal_height / 2).max(5);

        let tree_list = TreeList::new(
            tree,
            current_leaf_id,
            max_visible_lines,
            initial_selected_id,
            initial_filter_mode,
        );

        // Capture callbacks
        let on_label_change_clone = on_label_change.map(|cb| {
            Box::new(move |id: String, label: Option<String>| cb(id, label))
                as Box<dyn Fn(String, Option<String>) + Send + Sync>
        });

        let _on_select_stub = on_select;
        let _on_cancel_stub: Box<dyn Fn() + Send + Sync> = on_cancel;

        // We need to set callbacks on tree_list after creation.
        // In the real TUI these would be event-based.

        TreeSelectorComponent {
            tree_list,
            label_input: None,
            focused: false,
            on_label_change: on_label_change_clone,
        }
    }

    pub fn handle_input(&mut self, key_data: &str) {
        if let Some(ref mut label_input) = self.label_input {
            label_input.handle_input(key_data);
        } else {
            self.tree_list.handle_input(key_data);
        }
    }

    pub fn get_tree_list(&self) -> &TreeList {
        &self.tree_list
    }
}

impl Component for TreeSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        let t = theme();
        let mut lines: Vec<String> = Vec::new();

        lines.extend(Spacer::new(1).render(width));
        lines.extend(DynamicBorder::new(None).render(width));
        lines.extend(Text::new(t.bold("  Session Tree"), 1, 0).render(width));

        // Tree help line
        lines.push(t.fg(
            "muted",
            "  move (↑↓) · page (←→) · branch (fold/unfold) · label · filters · cycle",
        ));

        // Search line
        let query = self.tree_list.get_search_query();
        if !query.is_empty() {
            lines.push(format!(
                "  {} {}",
                t.fg("muted", "Type to search:"),
                t.fg("accent", query)
            ));
        } else {
            lines.push(format!("  {}", t.fg("muted", "Type to search:")));
        }

        lines.extend(DynamicBorder::new(None).render(width));
        lines.extend(Spacer::new(1).render(width));

        if let Some(ref label_input) = self.label_input {
            lines.extend(label_input.render(width));
        } else {
            lines.extend(self.tree_list.render(width));
        }

        lines.extend(Spacer::new(1).render(width));
        lines.extend(DynamicBorder::new(None).render(width));

        lines
    }

    fn invalidate(&mut self) {
        self.tree_list.invalidate();
    }
}

impl Focusable for TreeSelectorComponent {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if let Some(ref mut label_input) = self.label_input {
            label_input.set_focused(focused);
        }
    }
}

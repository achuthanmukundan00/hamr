//! Component that renders a session selector with search, scope toggling, and session management.
//!
//! Port of `packages/coding-agent/src/modes/interactive/components/session-selector.ts`.

use crate::modes::interactive::components::dynamic_border::DynamicBorder;
use crate::modes::interactive::components::session_selector_search::{
    NameFilter, SessionInfo, SortMode, filter_and_sort_sessions, has_session_name,
};
use crate::modes::interactive::components::tui_shim::{
    Component, Container, Focusable, Input, Spacer, Text, get_keybindings,
};
use crate::modes::interactive::theme::theme::THEME;

/// Scope of sessions to show.
#[derive(Clone, Copy, PartialEq)]
enum SessionScope {
    Current,
    All,
}

/// A flat node for display with tree structure info.
struct FlatSessionNode {
    session: SessionInfo,
    depth: usize,
    is_last: bool,
    ancestor_continues: Vec<bool>,
}

/// Shorten a path by replacing the home directory with ~.
fn shorten_path(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if path.starts_with(&home) {
            return format!("~{}", &path[home.len()..]);
        }
    }
    path.to_string()
}

/// Format a date as a relative time string.
fn format_session_date(date_ms: i64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let diff_ms = now_ms - date_ms;
    let diff_mins = diff_ms / 60000;
    let diff_hours = diff_ms / 3600000;
    let diff_days = diff_ms / 86400000;

    if diff_mins < 1 {
        "now".to_string()
    } else if diff_mins < 60 {
        format!("{}m", diff_mins)
    } else if diff_hours < 24 {
        format!("{}h", diff_hours)
    } else if diff_days < 7 {
        format!("{}d", diff_days)
    } else if diff_days < 30 {
        format!("{}w", diff_days / 7)
    } else if diff_days < 365 {
        format!("{}mo", diff_days / 30)
    } else {
        format!("{}y", diff_days / 365)
    }
}

/// Build a tree prefix for a flat session node.
fn build_tree_prefix(node: &FlatSessionNode) -> String {
    if node.depth == 0 {
        return String::new();
    }
    let mut parts = String::new();
    for continues in &node.ancestor_continues {
        parts.push_str(if *continues { "│  " } else { "   " });
    }
    parts.push_str(if node.is_last { "└─ " } else { "├─ " });
    parts
}

/// A session selector header that shows sort, scope, and filter info.
struct SessionSelectorHeader {
    scope: SessionScope,
    sort_mode: SortMode,
    name_filter: NameFilter,
    loading: bool,
    load_progress: Option<(usize, usize)>,
    show_path: bool,
    confirming_delete_path: Option<String>,
    status_message: Option<StatusMessage>,
    show_rename_hint: bool,
}

struct StatusMessage {
    kind: StatusKind,
    message: String,
}

enum StatusKind {
    Info,
    Error,
}

impl SessionSelectorHeader {
    fn new(scope: SessionScope, sort_mode: SortMode, name_filter: NameFilter) -> Self {
        Self {
            scope,
            sort_mode,
            name_filter,
            loading: false,
            load_progress: None,
            show_path: false,
            confirming_delete_path: None,
            status_message: None,
            show_rename_hint: false,
        }
    }
}

/// The main session selector component.
pub struct SessionSelectorComponent {
    layout: Container,
    header: SessionSelectorHeader,
    scope: SessionScope,
    sort_mode: SortMode,
    name_filter: NameFilter,
    current_sessions: Vec<SessionInfo>,
    all_sessions: Vec<SessionInfo>,
    filtered_sessions: Vec<FlatSessionNode>,
    selected_index: usize,
    search_input: Input,
    session_list_pos: usize,
    show_cwd: bool,
    show_path: bool,
    confirming_delete_path: Option<String>,
    max_visible: usize,
    on_select_callback: Box<dyn Fn(String) + Send + Sync>,
    on_cancel_callback: Box<dyn Fn() + Send + Sync>,
    current_session_path: Option<String>,
    can_rename: bool,
    focused: bool,
}

impl SessionSelectorComponent {
    /// Create a new session selector.
    ///
    /// * `current_sessions` - sessions in the current directory
    /// * `all_sessions` - all sessions across all directories
    /// * `on_select` - called with the selected session path
    /// * `on_cancel` - called on cancel
    /// * `current_session_path` - path of the currently active session
    pub fn new(
        current_sessions: Vec<SessionInfo>,
        all_sessions: Vec<SessionInfo>,
        on_select: Box<dyn Fn(String) + Send + Sync>,
        on_cancel: Box<dyn Fn() + Send + Sync>,
        current_session_path: Option<String>,
    ) -> Self {
        let scope = SessionScope::Current;
        let sort_mode = SortMode::Threaded;
        let name_filter = NameFilter::All;
        let can_rename = true;

        let header = SessionSelectorHeader::new(scope, sort_mode, name_filter);
        let mut layout = Container::new();

        // Build base layout
        layout.add_child(Box::new(Spacer::new(1)));
        layout.add_child(Box::new(DynamicBorder::new(Some(std::sync::Arc::new(
            |s: &str| THEME.fg("accent", s),
        )))));
        layout.add_child(Box::new(Spacer::new(1)));

        // Session list container
        let session_list_pos = layout.children().len();
        layout.add_child(Box::new(Container::new()));

        layout.add_child(Box::new(Spacer::new(1)));
        layout.add_child(Box::new(DynamicBorder::new(Some(std::sync::Arc::new(
            |s: &str| THEME.fg("accent", s),
        )))));

        let mut result = Self {
            layout,
            header,
            scope,
            sort_mode,
            name_filter,
            current_sessions,
            all_sessions,
            filtered_sessions: Vec::new(),
            selected_index: 0,
            search_input: Input::new(),
            session_list_pos,
            show_cwd: false,
            show_path: false,
            confirming_delete_path: None,
            max_visible: 10,
            on_select_callback: on_select,
            on_cancel_callback: on_cancel,
            current_session_path,
            can_rename,
            focused: false,
        };

        result.filter_sessions("");
        result
    }

    /// Filter and sort sessions based on the current scope and query.
    fn filter_sessions(&mut self, query: &str) {
        let sessions = if self.scope == SessionScope::Current {
            &self.current_sessions
        } else {
            &self.all_sessions
        };

        let filtered = filter_and_sort_sessions(sessions, query, self.sort_mode, self.name_filter);

        // Threaded mode without search: show flat list (tree rendering simplified for stub)
        self.filtered_sessions = filtered
            .iter()
            .map(|session| FlatSessionNode {
                session: session.clone(),
                depth: 0,
                is_last: true,
                ancestor_continues: Vec::new(),
            })
            .collect();

        self.selected_index = self
            .selected_index
            .min(self.filtered_sessions.len().saturating_sub(1));
        self.update_list();
    }

    fn is_current_session_path(&self, path: &str) -> bool {
        match &self.current_session_path {
            Some(current) => current == path,
            None => false,
        }
    }

    /// Rebuild the visible session list.
    fn update_list(&mut self) {
        let mut new_list = Container::new();

        if self.filtered_sessions.is_empty() {
            let empty_message = if self.scope == SessionScope::Current {
                "  No sessions in current folder. Press Tab to view all."
            } else {
                "  No sessions found"
            };
            new_list.add_child(Box::new(Text::new(THEME.fg("muted", empty_message), 0, 0)));
        } else {
            let start_index = self
                .selected_index
                .saturating_sub(self.max_visible / 2)
                .min(
                    self.filtered_sessions
                        .len()
                        .saturating_sub(self.max_visible),
                );
            let end_index = (start_index + self.max_visible).min(self.filtered_sessions.len());

            for i in start_index..end_index {
                let node = &self.filtered_sessions[i];
                let session = &node.session;
                let is_selected = i == self.selected_index;
                let is_confirming_delete = self
                    .confirming_delete_path
                    .as_ref()
                    .map_or(false, |p| p == &session.path);
                let is_current = self.is_current_session_path(&session.path);

                let prefix = build_tree_prefix(node);
                let cursor = if is_selected {
                    THEME.fg("accent", "› ")
                } else {
                    "  ".to_string()
                };

                let display_text = session
                    .name
                    .as_deref()
                    .unwrap_or(&session.first_message)
                    .replace(|c: char| c.is_control() && c != '\n', " ")
                    .trim()
                    .to_string();
                let age = format_session_date(session.modified);
                let msg_count = session.message_count.to_string();
                let _right_part = format!("{}  {}", msg_count, age);

                // Style: error if confirming delete, accent if current, warning if named
                let is_named = has_session_name(session);
                let styled_msg = if is_confirming_delete {
                    THEME.fg("error", &display_text)
                } else if is_current {
                    THEME.fg("accent", &display_text)
                } else if is_named {
                    THEME.fg("warning", &display_text)
                } else {
                    display_text
                };

                let styled_msg = if is_selected {
                    THEME.bold(&styled_msg)
                } else {
                    styled_msg
                };

                let line = format!("{}{}  {}", cursor, prefix, styled_msg);
                new_list.add_child(Box::new(Text::new(line, 0, 0)));
            }

            // Scroll indicator
            if start_index > 0 || end_index < self.filtered_sessions.len() {
                let scroll_info = format!(
                    "  ({}/{})",
                    self.selected_index + 1,
                    self.filtered_sessions.len()
                );
                new_list.add_child(Box::new(Text::new(THEME.fg("muted", &scroll_info), 0, 0)));
            }
        }

        if self.session_list_pos < self.layout.children().len() {
            self.layout.children_mut()[self.session_list_pos] = Box::new(new_list);
        }
    }

    /// Toggle the session scope between current and all.
    fn toggle_scope(&mut self) {
        self.scope = match self.scope {
            SessionScope::Current => {
                self.show_cwd = true;
                SessionScope::All
            }
            SessionScope::All => {
                self.show_cwd = false;
                SessionScope::Current
            }
        };
        self.header.scope = self.scope;
        self.filter_sessions("");
    }

    /// Cycle sort mode: threaded -> recent -> relevance -> threaded
    fn toggle_sort_mode(&mut self) {
        self.sort_mode = match self.sort_mode {
            SortMode::Threaded => SortMode::Recent,
            SortMode::Recent => SortMode::Relevance,
            SortMode::Relevance => SortMode::Threaded,
        };
        self.header.sort_mode = self.sort_mode;
        {
            let query = self.search_input.get_value().to_string();
            self.filter_sessions(&query);
        }
    }

    /// Toggle name filter between all and named.
    fn toggle_name_filter(&mut self) {
        self.name_filter = match self.name_filter {
            NameFilter::All => NameFilter::Named,
            NameFilter::Named => NameFilter::All,
        };
        self.header.name_filter = self.name_filter;
        {
            let query = self.search_input.get_value().to_string();
            self.filter_sessions(&query);
        }
    }

    /// Handle keyboard input for the session selector.
    pub fn handle_input(&mut self, data: &str) {
        let kb = get_keybindings();

        if kb.matches(data, "tui.input.tab") {
            self.toggle_scope();
            return;
        }

        if kb.matches(data, "app.session.toggleSort") {
            self.toggle_sort_mode();
            return;
        }

        if kb.matches(data, "app.session.toggleNamedFilter") {
            self.toggle_name_filter();
            return;
        }

        // Up arrow
        if kb.matches(data, "tui.select.up") {
            self.selected_index = self.selected_index.saturating_sub(1);
            self.update_list();
            return;
        }

        // Down arrow
        if kb.matches(data, "tui.select.down") {
            if self.filtered_sessions.is_empty() {
                return;
            }
            if self.selected_index + 1 < self.filtered_sessions.len() {
                self.selected_index += 1;
            }
            self.update_list();
            return;
        }

        // Enter
        if kb.matches(data, "tui.select.confirm") {
            if let Some(node) = self.filtered_sessions.get(self.selected_index) {
                (self.on_select_callback)(node.session.path.clone());
            }
            return;
        }

        // Escape
        if kb.matches(data, "tui.select.cancel") {
            (self.on_cancel_callback)();
            return;
        }

        // Pass to search input
        {
            let query = self.search_input.get_value().to_string();
            self.filter_sessions(&query);
        }
    }
}

impl Component for SessionSelectorComponent {
    fn render(&self, width: u16) -> Vec<String> {
        self.layout.render(width)
    }

    fn invalidate(&mut self) {
        self.layout.invalidate();
    }
}

impl Focusable for SessionSelectorComponent {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

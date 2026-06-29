//! TUI component abstractions.
//!
//! These types provide a stable API surface for 40+ interactive-mode components.
//! They are intentionally framework-agnostic — the actual terminal rendering is
//! handled by `sexy-tui-rs` via `InteractiveMode`, which wires the two together.
//!
//! When `feature = "tui"` is enabled, `sexy-tui-rs` is compiled and available
//! for the `InteractiveMode` to use. The shim types here remain as the component
//! model because the 40 consumer files were ported against this API.
//!
//! Over time, the shim types can be replaced with direct `sexy-tui-rs` re-exports
//! as API surfaces converge.

use std::any::Any;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

pub trait Component: Send + Sync + Any {
    fn render(&self, width: u16) -> Vec<String>;
    fn invalidate(&mut self);
}

impl<F> Component for F
where
    F: Fn(u16) -> Vec<String> + Send + Sync + Any,
{
    fn render(&self, width: u16) -> Vec<String> {
        self(width)
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// TUI handle
// ---------------------------------------------------------------------------

pub trait TuiHandle: Send + Sync {
    fn request_render(&self);
    fn request_render_full(&self, _full: bool) {
        self.request_render();
    }
    fn stop(&self);
    fn start(&self);
}

// ---------------------------------------------------------------------------
// Container
// ---------------------------------------------------------------------------

pub struct Container {
    children: Vec<Box<dyn Component>>,
}

impl Container {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
    pub fn add_child(&mut self, child: Box<dyn Component>) {
        self.children.push(child);
    }
    pub fn remove_child(&mut self, child: &Box<dyn Component>) {
        let child_ptr = &**child as *const (dyn Component + '_) as *const ();
        self.children
            .retain(|c| std::ptr::addr_of!(**c) as *const () != child_ptr);
    }
    pub fn clear(&mut self) {
        self.children.clear();
    }
    pub fn children(&self) -> &[Box<dyn Component>] {
        &self.children
    }
    pub fn children_mut(&mut self) -> &mut Vec<Box<dyn Component>> {
        &mut self.children
    }
}

impl Component for Container {
    fn render(&self, width: u16) -> Vec<String> {
        let mut lines = Vec::new();
        for child in &self.children {
            lines.extend(child.render(width));
        }
        lines
    }
    fn invalidate(&mut self) {
        for child in &mut self.children {
            child.invalidate();
        }
    }
}

// ---------------------------------------------------------------------------
// Focusable
// ---------------------------------------------------------------------------

pub trait Focusable {
    fn is_focused(&self) -> bool;
    fn set_focused(&mut self, focused: bool);
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

pub struct Text {
    text: String,
    indent: u16,
    margin: u16,
}
impl Text {
    pub fn new(text: impl Into<String>, indent: u16, margin: u16) -> Self {
        Self {
            text: text.into(),
            indent,
            margin,
        }
    }
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}
impl Component for Text {
    fn render(&self, _width: u16) -> Vec<String> {
        let indent = " ".repeat(self.indent as usize);
        let margin = " ".repeat(self.margin as usize);
        self.text
            .lines()
            .map(|line| format!("{}{}{}", margin, indent, line))
            .collect()
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// Spacer
// ---------------------------------------------------------------------------

pub struct Spacer {
    lines: u16,
}
impl Spacer {
    pub fn new(lines: u16) -> Self {
        Self { lines }
    }
    pub fn set_lines(&mut self, lines: u16) {
        self.lines = lines;
    }
}
impl Component for Spacer {
    fn render(&self, _width: u16) -> Vec<String> {
        vec!["".to_string(); self.lines as usize]
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// CardBox
// ---------------------------------------------------------------------------

pub struct CardBox {
    pad_x: u16,
    pad_y: u16,
    bg_fn: Option<Arc<dyn Fn(&str) -> String + Send + Sync>>,
    children: Container,
}
impl CardBox {
    pub fn new(
        pad_x: u16,
        pad_y: u16,
        bg_fn: Option<Arc<dyn Fn(&str) -> String + Send + Sync>>,
    ) -> Self {
        Self {
            pad_x,
            pad_y,
            bg_fn,
            children: Container::new(),
        }
    }
    pub fn add_child(&mut self, child: Box<dyn Component>) {
        self.children.add_child(child);
    }
    pub fn clear(&mut self) {
        self.children.clear();
    }
}
impl Component for CardBox {
    fn render(&self, width: u16) -> Vec<String> {
        let inner_width = width.saturating_sub(2 * self.pad_x);
        let inner_lines = self.children.render(inner_width);
        let pad_x = " ".repeat(self.pad_x as usize);
        let mut lines = Vec::new();
        for _ in 0..self.pad_y {
            lines.push(" ".repeat(width as usize));
        }
        for line in &inner_lines {
            let full = format!(
                "{}{}{}",
                &pad_x,
                line,
                " ".repeat(width.saturating_sub(pad_x.len() as u16 + line.len() as u16) as usize)
            );
            lines.push(full);
        }
        for _ in 0..self.pad_y {
            lines.push(" ".repeat(width as usize));
        }
        lines
    }
    fn invalidate(&mut self) {
        self.children.invalidate();
    }
}

// ---------------------------------------------------------------------------
// Markdown + MarkdownTheme
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MarkdownTheme {
    pub color: Option<String>,
    pub italic: bool,
}
impl MarkdownTheme {
    pub fn new() -> Self {
        Self {
            color: None,
            italic: false,
        }
    }
    pub fn set_color(&mut self, color: Option<String>) {
        self.color = color;
    }
}
impl Default for MarkdownTheme {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Markdown {
    text: String,
    indent: u16,
    margin: u16,
    theme: MarkdownTheme,
}
impl Markdown {
    pub fn new(text: impl Into<String>, indent: u16, margin: u16, theme: MarkdownTheme) -> Self {
        Self {
            text: text.into(),
            indent,
            margin,
            theme,
        }
    }
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}
impl Component for Markdown {
    fn render(&self, _width: u16) -> Vec<String> {
        let indent = " ".repeat(self.indent as usize);
        let margin = " ".repeat(self.margin as usize);
        self.text
            .lines()
            .map(|line| format!("{}{}{}", margin, indent, line))
            .collect()
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

pub struct Input {
    value: String,
    pub on_submit: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_escape: Option<Box<dyn Fn() + Send + Sync>>,
    focused: bool,
}
impl Input {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            on_submit: None,
            on_escape: None,
            focused: false,
        }
    }
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
    }
    pub fn get_value(&self) -> &str {
        &self.value
    }
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    pub fn handle_input(&mut self, _data: &str) {}
}
impl Component for Input {
    fn render(&self, _width: u16) -> Vec<String> {
        let cursor = if self.focused { "▌" } else { " " };
        vec![format!("> {}{}", self.value, cursor)]
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// Editor
// ---------------------------------------------------------------------------

pub struct Editor {
    text: String,
    pub on_submit: Option<Box<dyn Fn(String) + Send + Sync>>,
    focused: bool,
}
impl Editor {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            on_submit: None,
            focused: false,
        }
    }
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
    pub fn get_text(&self) -> &str {
        &self.text
    }
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    pub fn is_showing_autocomplete(&self) -> bool {
        false
    }
    pub fn handle_input(&mut self, _data: &str) {}
}
impl Component for Editor {
    fn render(&self, _width: u16) -> Vec<String> {
        let cursor = if self.focused { " ▌" } else { "" };
        self.text
            .lines()
            .enumerate()
            .map(|(i, line)| {
                if i == self.text.lines().count() - 1 {
                    format!("{}{}", line, cursor)
                } else {
                    line.to_string()
                }
            })
            .collect()
    }
    fn invalidate(&mut self) {}
}
impl Focusable for Editor {
    fn is_focused(&self) -> bool {
        self.focused
    }
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

pub struct Loader {
    message: String,
    running: bool,
    tick: u64,
}
impl Loader {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            running: true,
            tick: 0,
        }
    }
    pub fn stop(&mut self) {
        self.running = false;
    }
    pub fn tick(&mut self) {
        self.tick += 1;
    }
}
impl Component for Loader {
    fn render(&self, _width: u16) -> Vec<String> {
        if !self.running {
            return vec![];
        }
        let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = spinners[(self.tick % spinners.len() as u64) as usize];
        vec![format!("{} {}", spinner, self.message)]
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// CancellableLoader
// ---------------------------------------------------------------------------

pub struct CancellableLoader {
    loader: Loader,
    aborted: bool,
}
impl CancellableLoader {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            loader: Loader::new(message),
            aborted: false,
        }
    }
    pub fn stop(&mut self) {
        self.loader.stop();
    }
    pub fn signal(&self) -> bool {
        self.aborted
    }
    pub fn handle_input(&mut self, _data: &str) {}
    pub fn dispose(&mut self) {
        self.stop();
    }
}
impl Component for CancellableLoader {
    fn render(&self, width: u16) -> Vec<String> {
        self.loader.render(width)
    }
    fn invalidate(&mut self) {
        self.loader.invalidate();
    }
}

// ---------------------------------------------------------------------------
// TruncatedText
// ---------------------------------------------------------------------------

pub struct TruncatedText {
    text: String,
    indent: u16,
    margin: u16,
}
impl TruncatedText {
    pub fn new(text: impl Into<String>, indent: u16, margin: u16) -> Self {
        Self {
            text: text.into(),
            indent,
            margin,
        }
    }
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}
impl Component for TruncatedText {
    fn render(&self, _width: u16) -> Vec<String> {
        vec![self.text.clone()]
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// SelectItem + SelectListLayoutOptions + SelectList
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SelectItem {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Clone)]
pub struct SelectListLayoutOptions {
    pub min_primary_column_width: u16,
    pub max_primary_column_width: u16,
}
impl Default for SelectListLayoutOptions {
    fn default() -> Self {
        Self {
            min_primary_column_width: 0,
            max_primary_column_width: 60,
        }
    }
}

pub struct SelectList {
    items: Vec<SelectItem>,
    selected_index: usize,
    max_visible: usize,
    pub on_select: Option<Box<dyn Fn(SelectItem) + Send + Sync>>,
    pub on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_selection_change: Option<Box<dyn Fn(SelectItem) + Send + Sync>>,
}
impl SelectList {
    pub fn new(
        items: Vec<SelectItem>,
        max_visible: usize,
        _on_select: impl Fn(SelectItem) + Send + Sync + 'static,
        _on_cancel: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            items,
            selected_index: 0,
            max_visible,
            on_select: None,
            on_cancel: None,
            on_selection_change: None,
        }
    }
    pub fn set_selected_index(&mut self, index: usize) {
        self.selected_index = index;
    }
    pub fn handle_input(&mut self, _data: &str) {}
    pub fn selected_item(&self) -> Option<&SelectItem> {
        self.items.get(self.selected_index)
    }
}
impl Component for SelectList {
    fn render(&self, _width: u16) -> Vec<String> {
        let mut lines = Vec::new();
        let start = self
            .selected_index
            .saturating_sub(self.max_visible / 2)
            .min(self.items.len().saturating_sub(self.max_visible));
        let end = (start + self.max_visible).min(self.items.len());
        for i in start..end {
            let item = &self.items[i];
            let prefix = if i == self.selected_index {
                "→ "
            } else {
                "  "
            };
            lines.push(format!("{}{}", prefix, item.label));
        }
        if start > 0 || end < self.items.len() {
            lines.push(format!(
                "  ({}/{})",
                self.selected_index + 1,
                self.items.len()
            ));
        }
        lines
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// SettingItem + SettingsList
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SettingItem {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub current_value: String,
    pub values: Option<Vec<String>>,
    pub section: Option<String>,
}
impl SettingItem {
    pub fn new(
        id: String,
        label: String,
        description: &str,
        current_value: String,
        values: Option<Vec<String>>,
        section: Option<String>,
    ) -> Self {
        Self {
            id,
            label,
            description: if description.is_empty() {
                None
            } else {
                Some(description.to_string())
            },
            current_value,
            values,
            section,
        }
    }
}

pub struct SettingsList {
    items: Vec<SettingItem>,
    max_visible: usize,
    pub on_select: Option<Box<dyn Fn(String, String) + Send + Sync>>,
    pub on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
}
impl SettingsList {
    pub fn new(
        items: Vec<SettingItem>,
        max_visible: usize,
        _on_select: impl Fn(String, String) + Send + Sync + 'static,
        _on_cancel: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            items,
            max_visible,
            on_select: None,
            on_cancel: None,
        }
    }
    pub fn handle_input(&mut self, _data: &str) {}
}
impl Component for SettingsList {
    fn render(&self, _width: u16) -> Vec<String> {
        vec!["settings-list".to_string()]
    }
    fn invalidate(&mut self) {}
}

// ---------------------------------------------------------------------------
// Keybindings + Key
// ---------------------------------------------------------------------------

/// When the `tui` feature is enabled, delegate to the real `sexy-tui-rs`
/// `KeybindingsManager`. Otherwise, provide a stub that always returns false.
#[cfg(feature = "tui")]
pub use self::keybindings_real::*;
#[cfg(not(feature = "tui"))]
pub use self::keybindings_stub::*;

#[cfg(feature = "tui")]
mod keybindings_real {
    use sexy_tui_rs::keybindings::{KeybindingsManager, TUI_KEYBINDINGS};
    use std::sync::Mutex;

    pub struct Keybindings {
        inner: KeybindingsManager,
    }

    impl Keybindings {
        pub fn new() -> Self {
            Self {
                inner: KeybindingsManager::new(TUI_KEYBINDINGS.clone()),
            }
        }

        pub fn matches(&self, data: &str, binding: &str) -> bool {
            self.inner.matches(data, binding)
        }

        pub fn get_keys(&self, binding: &str) -> Vec<String> {
            self.inner
                .get_keys(binding)
                .into_iter()
                .map(|k| k.to_string())
                .collect()
        }
    }

    static KEYBINDINGS_INSTANCE: Mutex<Option<Keybindings>> = Mutex::new(None);

    pub fn get_keybindings() -> &'static Keybindings {
        let mut guard = KEYBINDINGS_INSTANCE
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            *guard = Some(Keybindings::new());
        }
        // SAFETY: we never drop the Keybindings, so the reference remains valid
        unsafe { &*(guard.as_ref().unwrap() as *const Keybindings) }
    }

    pub fn matches_key(data: &str, key: &str) -> bool {
        sexy_tui_rs::matches_key(data, key)
    }
}

#[cfg(not(feature = "tui"))]
mod keybindings_stub {
    pub struct Keybindings;
    impl Keybindings {
        pub fn new() -> Self {
            Self
        }
        pub fn matches(&self, _data: &str, _binding: &str) -> bool {
            false
        }
        pub fn get_keys(&self, binding: &str) -> Vec<String> {
            vec![binding.to_string()]
        }
    }
    pub fn get_keybindings() -> &'static Keybindings {
        static KB: Keybindings = Keybindings;
        &KB
    }
    pub fn matches_key(_data: &str, _key: &str) -> bool {
        false
    }
}

pub struct Key;
impl Key {
    pub fn ctrl(ch: char) -> String {
        format!("ctrl+{}", ch)
    }
    pub fn escape() -> &'static str {
        "escape"
    }
}

// ---------------------------------------------------------------------------
// TerminalCapabilities
// ---------------------------------------------------------------------------

pub struct TerminalCapabilities {
    pub images: bool,
}
pub fn get_capabilities() -> TerminalCapabilities {
    TerminalCapabilities { images: false }
}

// ---------------------------------------------------------------------------
// Fuzzy
// ---------------------------------------------------------------------------

pub struct FuzzyMatchResult {
    pub matches: bool,
    pub score: f64,
}
pub fn fuzzy_match(_query: &str, _target: &str) -> FuzzyMatchResult {
    FuzzyMatchResult {
        matches: true,
        score: 0.0,
    }
}
pub fn fuzzy_filter<T: Clone, F: Fn(&T) -> String>(
    _items: &[T],
    _query: &str,
    _extract: F,
) -> Vec<T> {
    _items.to_vec()
}

// ---------------------------------------------------------------------------
// AnyComponent
// ---------------------------------------------------------------------------

pub trait AnyComponent: Component + Focusable {}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

pub fn visible_width(s: &str) -> u16 {
    let mut in_escape = false;
    let mut width = 0u16;
    for ch in s.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() && ch != '[' && ch != ';' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            width += 1;
        }
    }
    width
}

pub fn truncate_to_width(s: &str, width: u16, ellipsis: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    let mut visible = 0u16;
    let ellipsis_width = visible_width(ellipsis);
    let effective_width = width.saturating_sub(ellipsis_width);
    for ch in s.chars() {
        if in_escape {
            out.push(ch);
            if ch.is_ascii_alphabetic() && ch != '[' && ch != ';' {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
            out.push(ch);
        } else {
            if visible >= effective_width {
                break;
            }
            visible += 1;
            out.push(ch);
        }
    }
    if visible_width(&out) + ellipsis_width < width {
        s.to_string()
    } else {
        out.push_str(ellipsis);
        out
    }
}

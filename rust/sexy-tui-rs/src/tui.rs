use std::cell::RefCell;
/// Core TUI implementation with differential rendering.
/// Pi-identical port of @earendil-works/pi-tui src/tui.ts (1641 lines).
use std::collections::HashSet;
use std::rc::Rc;

use crate::keys::{is_key_release, matches_key};
use crate::terminal::Terminal;
use crate::terminal_image::{delete_all_kitty_images, delete_kitty_image, is_image_line};
use crate::utils::{extract_segments, slice_with_width, visible_width};

/// Zero-width APC escape sequence used as a cursor position marker.
pub const CURSOR_MARKER: &str = "\x1b_pi:c\x07";

const SEGMENT_RESET: &str = "\x1b[0m\x1b]8;;\x07";
const KITTY_SEQUENCE_PREFIX: &str = "\x1b_G";

// =============================================================================
// Component Trait
// =============================================================================

pub trait Component {
    fn render(&self, width: u16) -> Vec<String>;
    fn handle_input(&mut self, _data: &str) {}
    fn wants_key_release(&self) -> bool {
        false
    }
    fn invalidate(&mut self);
}

/// Components that can receive focus and display a hardware cursor for IME.
pub trait Focusable {
    fn set_focused(&mut self, focused: bool);
    fn is_focused(&self) -> bool;
}

// =============================================================================
// Container
// =============================================================================

pub struct Container {
    pub children: Vec<Box<dyn Component>>,
}

impl Container {
    pub fn new() -> Self {
        Container {
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Box<dyn Component>) {
        self.children.push(child);
    }

    pub fn remove_child(&mut self, idx: usize) {
        if idx < self.children.len() {
            self.children.remove(idx);
        }
    }

    pub fn clear(&mut self) {
        self.children.clear();
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
    fn handle_input(&mut self, _data: &str) {}
    fn invalidate(&mut self) {
        for child in &mut self.children {
            child.invalidate();
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Overlay Support
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayAnchor {
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    LeftCenter,
    RightCenter,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Copy)]
pub struct OverlayMargin {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl OverlayMargin {
    pub fn all(value: u16) -> Self {
        OverlayMargin {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

pub struct OverlayOptions {
    pub width: Option<u16>,
    pub min_width: Option<u16>,
    pub max_height: Option<u16>,
    pub anchor: OverlayAnchor,
    pub offset_x: i16,
    pub offset_y: i16,
    pub row: Option<u16>,
    pub col: Option<u16>,
    pub margin: Option<OverlayMargin>,
    pub non_capturing: bool,
    pub visible: Option<Box<dyn Fn(u16, u16) -> bool>>,
}

impl Clone for OverlayOptions {
    fn clone(&self) -> Self {
        OverlayOptions {
            width: self.width,
            min_width: self.min_width,
            max_height: self.max_height,
            anchor: self.anchor,
            offset_x: self.offset_x,
            offset_y: self.offset_y,
            row: self.row,
            col: self.col,
            margin: self.margin,
            non_capturing: self.non_capturing,
            // NOTE: `visible` callback cannot be cloned (Fn trait objects are not Clone).
            // Cloned options lose the visibility predicate. Callers should not rely on
            // cloned OverlayOptions retaining the visibility callback.
            visible: None,
        }
    }
}

impl std::fmt::Debug for OverlayOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayOptions")
            .field("width", &self.width)
            .field("anchor", &self.anchor)
            .field("non_capturing", &self.non_capturing)
            .finish()
    }
}

impl Default for OverlayOptions {
    fn default() -> Self {
        OverlayOptions {
            width: None,
            min_width: None,
            max_height: None,
            anchor: OverlayAnchor::Center,
            offset_x: 0,
            offset_y: 0,
            row: None,
            col: None,
            margin: None,
            non_capturing: false,
            visible: None,
        }
    }
}

pub struct OverlayUnfocusOptions {
    pub target: Option<Box<dyn Component>>,
}

pub struct OverlayHandle {
    pub id: usize,
    hidden: Rc<RefCell<bool>>,
}

impl OverlayHandle {
    fn new(id: usize) -> Self {
        OverlayHandle {
            id,
            hidden: Rc::new(RefCell::new(false)),
        }
    }

    pub fn hide(&mut self) {
        *self.hidden.borrow_mut() = true;
    }
    pub fn set_hidden(&mut self, hidden: bool) {
        *self.hidden.borrow_mut() = hidden;
    }
    pub fn is_hidden(&self) -> bool {
        *self.hidden.borrow()
    }
}

impl Clone for OverlayHandle {
    fn clone(&self) -> Self {
        OverlayHandle {
            id: self.id,
            hidden: self.hidden.clone(),
        }
    }
}

// =============================================================================
// Overlay Stack Entry
// =============================================================================

struct OverlayStackEntry {
    id: usize,
    component: Box<dyn Component>,
    options: Option<OverlayOptions>,
    pre_focus_id: Option<usize>,
    hidden: Rc<RefCell<bool>>,
    focus_order: u64,
}

// =============================================================================
// Kitty Image Helpers
// =============================================================================

fn extract_kitty_image_ids(line: &str) -> Vec<u32> {
    let mut ids = Vec::new();
    let mut search_from = 0;
    while let Some(pos) = line[search_from..].find(KITTY_SEQUENCE_PREFIX) {
        let start = search_from + pos;
        let params_start = start + KITTY_SEQUENCE_PREFIX.len();
        if let Some(semi) = line[params_start..].find(';') {
            let params = &line[params_start..params_start + semi];
            for param in params.split(',') {
                if let Some(eq) = param.find('=') {
                    let key = &param[..eq];
                    let val = &param[eq + 1..];
                    if key == "i" {
                        if let Ok(id) = val.parse::<u32>() {
                            ids.push(id);
                        }
                    }
                }
            }
        }
        search_from = start + 1;
    }
    ids
}

fn get_kitty_image_reserved_rows(lines: &[String], index: usize) -> usize {
    let line = match lines.get(index) {
        Some(l) => l,
        None => return 1,
    };
    let params_start = if let Some(pos) = line.find(KITTY_SEQUENCE_PREFIX) {
        pos + KITTY_SEQUENCE_PREFIX.len()
    } else {
        return 1;
    };
    let rows: usize = line[params_start..]
        .split(';')
        .filter_map(|p| {
            if p.starts_with("r=") {
                p[2..].parse::<usize>().ok()
            } else {
                None
            }
        })
        .next()
        .unwrap_or(1);

    if rows <= 1 {
        return 1;
    }
    let max_rows = rows.min(lines.len() - index);
    let mut reserved = 1;
    while reserved < max_rows {
        let next = lines
            .get(index + reserved)
            .map(|s| s.as_str())
            .unwrap_or("");
        if is_image_line(next) || visible_width(next) > 0 {
            break;
        }
        reserved += 1;
    }
    reserved
}

// =============================================================================
// TUI — Main Interface
// =============================================================================

type InputListener = Box<dyn FnMut(&str) -> Option<String>>;

pub struct TUI {
    pub terminal: Box<dyn Terminal>,
    container: Container,
    previous_lines: Vec<String>,
    previous_kitty_image_ids: HashSet<u32>,
    previous_width: u16,
    previous_height: u16,
    focused_component_id: Option<usize>,
    input_listeners: Vec<InputListener>,
    pub on_debug: Option<Box<dyn Fn()>>,
    render_requested: bool,
    cursor_row: usize,
    hardware_cursor_row: usize,
    show_hardware_cursor: bool,
    clear_on_shrink: bool,
    max_lines_rendered: usize,
    previous_viewport_top: usize,
    full_redraw_count: u64,
    stopped: bool,
    focus_order_counter: u64,
    overlay_stack: Vec<OverlayStackEntry>,
    next_overlay_id: usize,
}

impl TUI {
    pub fn new(terminal: Box<dyn Terminal>) -> Self {
        TUI {
            terminal,
            container: Container::new(),
            previous_lines: Vec::new(),
            previous_kitty_image_ids: HashSet::new(),
            previous_width: 0,
            previous_height: 0,
            focused_component_id: None,
            input_listeners: Vec::new(),
            on_debug: None,
            render_requested: false,
            cursor_row: 0,
            hardware_cursor_row: 0,
            show_hardware_cursor: std::env::var("PI_HARDWARE_CURSOR").is_ok(),
            clear_on_shrink: std::env::var("PI_CLEAR_ON_SHRINK")
                .map(|v| v != "0")
                .unwrap_or(true),
            max_lines_rendered: 0,
            previous_viewport_top: 0,
            full_redraw_count: 0,
            stopped: false,
            focus_order_counter: 0,
            overlay_stack: Vec::new(),
            next_overlay_id: 1,
        }
    }

    pub fn full_redraws(&self) -> u64 {
        self.full_redraw_count
    }

    pub fn get_show_hardware_cursor(&self) -> bool {
        self.show_hardware_cursor
    }

    pub fn set_show_hardware_cursor(&mut self, enabled: bool) {
        if self.show_hardware_cursor == enabled {
            return;
        }
        self.show_hardware_cursor = enabled;
        if !enabled {
            self.terminal.hide_cursor();
        }
        self.request_render();
    }

    pub fn get_clear_on_shrink(&self) -> bool {
        self.clear_on_shrink
    }

    pub fn set_clear_on_shrink(&mut self, enabled: bool) {
        self.clear_on_shrink = enabled;
    }

    pub fn columns(&self) -> u16 {
        self.terminal.columns()
    }

    pub fn rows(&self) -> u16 {
        self.terminal.rows()
    }

    pub fn add_child(&mut self, child: Box<dyn Component>) {
        self.container.add_child(child);
    }

    pub fn resize(&mut self, columns: u16, rows: u16) {
        self.terminal.set_size(columns, rows);
        self.request_render();
    }

    pub fn remove_child(&mut self, idx: usize) {
        self.container.remove_child(idx);
    }

    pub fn children(&self) -> &[Box<dyn Component>] {
        &self.container.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<Box<dyn Component>> {
        &mut self.container.children
    }

    pub fn set_focus(&mut self, id: Option<usize>) {
        self.focused_component_id = id;
    }

    pub fn add_input_listener(&mut self, f: Box<dyn FnMut(&str) -> Option<String>>) {
        self.input_listeners.push(f);
    }

    // ── Overlays ──

    fn is_overlay_visible(entry: &OverlayStackEntry, term_cols: u16, term_rows: u16) -> bool {
        if *entry.hidden.borrow() {
            return false;
        }
        if let Some(ref opts) = entry.options {
            if let Some(ref visible_fn) = opts.visible {
                return visible_fn(term_cols, term_rows);
            }
        }
        true
    }

    pub fn show_overlay(
        &mut self,
        component: Box<dyn Component>,
        options: Option<OverlayOptions>,
    ) -> OverlayHandle {
        let id = self.next_overlay_id;
        self.next_overlay_id += 1;
        let handle = OverlayHandle::new(id);
        self.focus_order_counter += 1;

        let non_capturing = options.as_ref().map(|o| o.non_capturing).unwrap_or(false);
        let term_cols = self.terminal.columns();
        let term_rows = self.terminal.rows();

        let entry = OverlayStackEntry {
            id,
            component,
            options,
            pre_focus_id: self.focused_component_id,
            hidden: handle.hidden.clone(),
            focus_order: self.focus_order_counter,
        };
        self.overlay_stack.push(entry);

        if !non_capturing
            && Self::is_overlay_visible(self.overlay_stack.last().unwrap(), term_cols, term_rows)
        {
            self.set_focus(Some(id));
        }
        self.terminal.hide_cursor();
        self.request_render();
        handle
    }

    pub fn hide_overlay(&mut self) {
        let removed = match self.overlay_stack.pop() {
            Some(o) => o,
            None => return,
        };
        let removed_id = removed.id;

        // Retarget any overlays that pointed to the removed one
        self.retarget_overlay_pre_focus(removed_id);

        if self.focused_component_id == Some(removed_id) {
            // Restore focus: find topmost visible capturing overlay, or fall back to pre_focus
            let tc = self.terminal.columns();
            let tr = self.terminal.rows();
            let next_focus = {
                let top_visible = self.overlay_stack.iter().rev().find(|e| {
                    let nc = e.options.as_ref().map(|o| o.non_capturing).unwrap_or(false);
                    !*e.hidden.borrow() && Self::is_overlay_visible(e, tc, tr) && !nc
                });
                if let Some(entry) = top_visible {
                    Some(entry.id)
                } else {
                    removed.pre_focus_id
                }
            };
            self.set_focus(next_focus);
        }
        if self.overlay_stack.is_empty() {
            self.terminal.hide_cursor();
        }
        self.request_render();
    }

    /// Follow the pre_focus chain to check if a component is an ancestor of an overlay.
    #[allow(dead_code)]
    fn is_overlay_focus_ancestor(
        entry: &OverlayStackEntry,
        component_id: usize,
        stack: &[OverlayStackEntry],
    ) -> bool {
        let mut current_id = entry.pre_focus_id;
        let mut visited = std::collections::HashSet::new();
        while let Some(id) = current_id {
            if !visited.insert(id) {
                break;
            } // cycle guard
            if id == component_id {
                return true;
            }
            // Find the overlay entry whose id matches
            current_id = stack
                .iter()
                .find(|e| e.id == id)
                .and_then(|e| e.pre_focus_id);
        }
        false
    }

    /// Update all overlays whose pre_focus points to the removed overlay.
    fn retarget_overlay_pre_focus(&mut self, removed_id: usize) {
        // Find the removed entry's pre_focus (entry already popped, so look in stack)
        let fallback = self
            .overlay_stack
            .iter()
            .find(|e| e.id == removed_id)
            .and_then(|e| e.pre_focus_id);

        for entry in &mut self.overlay_stack {
            if entry.pre_focus_id == Some(removed_id) {
                entry.pre_focus_id = fallback;
            }
        }
    }

    pub fn has_overlay(&self) -> bool {
        let tc = self.terminal.columns();
        let tr = self.terminal.rows();
        self.overlay_stack
            .iter()
            .any(|e| !*e.hidden.borrow() && Self::is_overlay_visible(e, tc, tr))
    }

    // ── Start / Stop ──

    pub fn start(&mut self) {
        self.stopped = false;
        self.terminal.hide_cursor();
        self.request_render();
    }

    pub fn stop(&mut self) {
        self.stopped = true;
        if !self.previous_lines.is_empty() {
            let target_row = self.previous_lines.len();
            let line_diff = target_row as i16 - self.hardware_cursor_row as i16;
            if line_diff > 0 {
                self.terminal.write(&format!("\x1b[{}B", line_diff));
            } else if line_diff < 0 {
                self.terminal.write(&format!("\x1b[{}A", -line_diff));
            }
            self.terminal.write("\r\n");
        }
        self.terminal.show_cursor();
        self.terminal.stop();
    }

    // ── Render Scheduling ──

    pub fn request_render(&mut self) {
        if self.stopped || self.render_requested {
            return;
        }

        // The TypeScript TUI coalesces renders with an event-loop timer. This
        // Rust TUI is synchronous and has no callback that can fire after the
        // caller returns. Deferring here therefore loses the render entirely.
        // Render immediately until scheduling is owned by an async TUI driver.
        self.render_requested = true;
        self.render_requested = false;
        self.do_render();
    }

    // ── Input Handling ──

    pub fn handle_input(&mut self, data: &str) {
        let mut current = data.to_string();
        for listener in &mut self.input_listeners {
            match listener(&current) {
                Some(modified) => current = modified,
                None => return,
            }
        }

        if matches_key(&current, "shift+ctrl+d") {
            if let Some(ref debug) = self.on_debug {
                debug();
                return;
            }
        }

        let tc = self.terminal.columns();
        let tr = self.terminal.rows();

        let mut redirect_needed = false;
        for entry in &self.overlay_stack {
            if !Self::is_overlay_visible(entry, tc, tr) {
                redirect_needed = true;
                break;
            }
        }
        if redirect_needed {
            self.set_focus(None);
        }

        let focus_id = self.focused_component_id;
        let mut handled = false;

        for entry in &mut self.overlay_stack {
            if !*entry.hidden.borrow() && Self::is_overlay_visible(entry, tc, tr) {
                let is_focused = focus_id == Some(entry.id);
                let is_capturing = !entry
                    .options
                    .as_ref()
                    .map(|o| o.non_capturing)
                    .unwrap_or(false);
                if is_focused || !is_capturing {
                    if is_key_release(&current) && !entry.component.wants_key_release() {
                        return;
                    }
                    entry.component.handle_input(&current);
                    handled = true;
                    break;
                }
            }
        }

        if !handled {
            if let Some(id) = focus_id {
                if let Some(child) = self.container.children.get_mut(id) {
                    if !is_key_release(&current) || child.wants_key_release() {
                        child.handle_input(&current);
                    }
                }
            }
        }
        self.request_render();
    }

    // =========================================================================
    // Rendering
    // =========================================================================

    fn do_render(&mut self) {
        if self.stopped {
            return;
        }
        let width = self.terminal.columns();
        let height = self.terminal.rows();
        let width_changed = self.previous_width != 0 && self.previous_width != width;
        let height_changed = self.previous_height != 0 && self.previous_height != height;
        let previous_buffer_length = if self.previous_height > 0 {
            self.previous_viewport_top + self.previous_height as usize
        } else {
            height as usize
        };
        let prev_viewport_top = if height_changed {
            if previous_buffer_length > height as usize {
                previous_buffer_length - height as usize
            } else {
                0
            }
        } else {
            self.previous_viewport_top
        };

        let mut new_lines = self.container.render(width);

        if !self.overlay_stack.is_empty() {
            new_lines = self.composite_overlays(new_lines, width, height);
        }

        let cursor_pos = self.extract_cursor_position(&mut new_lines, height);

        for line in &mut new_lines {
            if !is_image_line(line) {
                line.push_str(SEGMENT_RESET);
            }
        }

        // First render
        if self.previous_lines.is_empty() && !width_changed && !height_changed {
            self.full_render(&new_lines, false);
            self.position_hardware_cursor(cursor_pos, new_lines.len());
            self.previous_lines = new_lines;
            self.previous_kitty_image_ids = self.collect_kitty_image_ids(&self.previous_lines);
            self.previous_width = width;
            self.previous_height = height;
            return;
        }

        // Width change
        if width_changed {
            self.full_render(&new_lines, true);
            self.position_hardware_cursor(cursor_pos, new_lines.len());
            self.previous_lines = new_lines;
            self.previous_kitty_image_ids = self.collect_kitty_image_ids(&self.previous_lines);
            self.previous_width = width;
            self.previous_height = height;
            return;
        }

        // Height change (not Termux)
        let is_termux = std::env::var("TERMUX_VERSION").is_ok();
        if height_changed && !is_termux {
            self.full_render(&new_lines, true);
            self.position_hardware_cursor(cursor_pos, new_lines.len());
            self.previous_lines = new_lines;
            self.previous_kitty_image_ids = self.collect_kitty_image_ids(&self.previous_lines);
            self.previous_width = width;
            self.previous_height = height;
            return;
        }

        // Content shrank
        if self.clear_on_shrink
            && new_lines.len() < self.max_lines_rendered
            && self.overlay_stack.is_empty()
        {
            self.full_render(&new_lines, true);
            self.position_hardware_cursor(cursor_pos, new_lines.len());
            self.previous_lines = new_lines;
            self.previous_kitty_image_ids = self.collect_kitty_image_ids(&self.previous_lines);
            self.previous_width = width;
            self.previous_height = height;
            return;
        }

        // Find changed range
        let mut first_changed: i32 = -1;
        let mut last_changed: i32 = -1;
        let max_lines = new_lines.len().max(self.previous_lines.len());
        for i in 0..max_lines {
            let old = self.previous_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let new = new_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            if old != new {
                if first_changed == -1 {
                    first_changed = i as i32;
                }
                last_changed = i as i32;
            }
        }
        let appended = new_lines.len() > self.previous_lines.len();
        if appended {
            if first_changed == -1 {
                first_changed = self.previous_lines.len() as i32;
            }
            last_changed = new_lines.len() as i32 - 1;
        }

        if first_changed == -1 {
            self.position_hardware_cursor(cursor_pos, new_lines.len());
            self.previous_viewport_top = prev_viewport_top;
            self.previous_height = height;
            return;
        }

        let first_changed = first_changed as usize;
        let last_changed = last_changed as usize;

        // All changes in deleted lines
        if first_changed >= new_lines.len() {
            if self.previous_lines.len() > new_lines.len() {
                let target_row = if new_lines.is_empty() {
                    0
                } else {
                    new_lines.len() - 1
                };
                if target_row < prev_viewport_top {
                    self.full_render(&new_lines, true);
                    self.position_hardware_cursor(cursor_pos, new_lines.len());
                    self.previous_lines = new_lines;
                    self.previous_kitty_image_ids =
                        self.collect_kitty_image_ids(&self.previous_lines);
                    self.previous_width = width;
                    self.previous_height = height;
                    return;
                }
                let mut buffer = format!(
                    "\x1b[?2026h{}",
                    self.delete_changed_kitty_images(first_changed, last_changed)
                );
                let line_diff = target_row as i16 - self.hardware_cursor_row as i16;
                if line_diff > 0 {
                    buffer.push_str(&format!("\x1b[{}B", line_diff));
                } else if line_diff < 0 {
                    buffer.push_str(&format!("\x1b[{}A", -line_diff));
                }
                buffer.push('\r');
                let extra = self.previous_lines.len() - new_lines.len();
                if extra > height as usize {
                    self.full_render(&new_lines, true);
                    self.position_hardware_cursor(cursor_pos, new_lines.len());
                    self.previous_lines = new_lines;
                    self.previous_kitty_image_ids =
                        self.collect_kitty_image_ids(&self.previous_lines);
                    self.previous_width = width;
                    self.previous_height = height;
                    return;
                }
                if extra > 0 && !new_lines.is_empty() {
                    buffer.push_str("\x1b[1B");
                }
                for _ in 0..extra {
                    buffer.push_str("\r\x1b[2K\x1b[1B");
                }
                let move_back = if new_lines.is_empty() { extra } else { extra };
                if move_back > 0 {
                    buffer.push_str(&format!("\x1b[{}A", move_back));
                }
                buffer.push_str("\x1b[?2026l");
                self.terminal.write(&buffer);
                self.cursor_row = target_row;
                self.hardware_cursor_row = target_row;
            }
            self.position_hardware_cursor(cursor_pos, new_lines.len());
            self.previous_lines = new_lines;
            self.previous_kitty_image_ids = self.collect_kitty_image_ids(&self.previous_lines);
            self.previous_width = width;
            self.previous_height = height;
            self.previous_viewport_top = prev_viewport_top;
            return;
        }

        // Change above viewport
        if first_changed < prev_viewport_top {
            self.full_render(&new_lines, true);
            self.position_hardware_cursor(cursor_pos, new_lines.len());
            self.previous_lines = new_lines;
            self.previous_kitty_image_ids = self.collect_kitty_image_ids(&self.previous_lines);
            self.previous_width = width;
            self.previous_height = height;
            return;
        }

        // Incremental render
        let mut buffer = format!(
            "\x1b[?2026h{}",
            self.delete_changed_kitty_images(first_changed, last_changed)
        );
        let append_start =
            appended && first_changed == self.previous_lines.len() && first_changed > 0;
        let move_target = if append_start {
            first_changed - 1
        } else {
            first_changed
        };
        let prev_viewport_bottom = prev_viewport_top + height as usize - 1;

        if move_target > prev_viewport_bottom {
            let current_screen_row = (self.hardware_cursor_row.saturating_sub(prev_viewport_top))
                .min(height as usize - 1);
            let move_to_bottom = height as usize - 1 - current_screen_row;
            if move_to_bottom > 0 {
                buffer.push_str(&format!("\x1b[{}B", move_to_bottom));
            }
            for _ in prev_viewport_bottom..move_target {
                buffer.push_str("\r\n");
            }
        }

        let line_diff = move_target as i16 - self.hardware_cursor_row as i16;
        if line_diff > 0 {
            buffer.push_str(&format!("\x1b[{}B", line_diff));
        } else if line_diff < 0 {
            buffer.push_str(&format!("\x1b[{}A", -line_diff));
        }
        buffer.push_str(if append_start { "\r\n" } else { "\r" });

        let render_end = last_changed.min(new_lines.len() - 1);
        for i in first_changed..=render_end {
            if i > first_changed {
                buffer.push_str("\r\n");
            }
            let line = &new_lines[i];
            if is_image_line(line) {
                let reserved = get_kitty_image_reserved_rows(&new_lines, i);
                if reserved > 1 {
                    buffer.push_str("\x1b[2K");
                    for _ in 1..reserved {
                        buffer.push_str("\r\n\x1b[2K");
                    }
                    buffer.push_str(&format!("\x1b[{}A", reserved - 1));
                    buffer.push_str(line);
                    buffer.push_str(&format!("\x1b[{}B", reserved - 1));
                    continue;
                }
            }
            buffer.push_str("\x1b[2K");
            #[cfg(debug_assertions)]
            {
                if !is_image_line(line) {
                    let vw = visible_width(line);
                    if vw > width as usize {
                        panic!("Line {} exceeds width ({} > {}): {:?}", i, vw, width, line);
                    }
                }
            }
            buffer.push_str(line);
        }

        if self.previous_lines.len() > new_lines.len() {
            let extra = self.previous_lines.len() - new_lines.len();
            for _ in 0..extra {
                buffer.push_str("\r\n\x1b[2K");
            }
            buffer.push_str(&format!("\x1b[{}A", extra));
        }

        buffer.push_str("\x1b[?2026l");
        self.terminal.write(&buffer);

        self.cursor_row = if new_lines.is_empty() {
            0
        } else {
            new_lines.len() - 1
        };
        self.hardware_cursor_row = render_end;
        self.max_lines_rendered = self.max_lines_rendered.max(new_lines.len());
        self.previous_viewport_top = prev_viewport_top.max(if render_end >= height as usize {
            render_end - height as usize + 1
        } else {
            0
        });

        self.position_hardware_cursor(cursor_pos, new_lines.len());
        self.previous_lines = new_lines;
        self.previous_kitty_image_ids = self.collect_kitty_image_ids(&self.previous_lines);
        self.previous_width = width;
        self.previous_height = height;
    }

    fn full_render(&mut self, lines: &[String], clear: bool) {
        self.full_redraw_count += 1;
        let mut buffer = String::from("\x1b[?2026h");
        if clear {
            buffer.push_str(&delete_all_kitty_images());
            buffer.push_str("\x1b[2J\x1b[H\x1b[3J");
        }
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                buffer.push_str("\r\n");
            }
            if is_image_line(line) {
                let reserved = get_kitty_image_reserved_rows(lines, i);
                if reserved > 1 && reserved <= self.previous_height as usize {
                    for _ in 1..reserved {
                        buffer.push_str("\r\n");
                    }
                    buffer.push_str(&format!("\x1b[{}A", reserved - 1));
                    buffer.push_str(line);
                    buffer.push_str(&format!("\x1b[{}B", reserved - 1));
                    continue;
                }
            }
            buffer.push_str(line);
        }
        buffer.push_str("\x1b[?2026l");
        self.terminal.write(&buffer);
        self.cursor_row = if lines.is_empty() { 0 } else { lines.len() - 1 };
        self.hardware_cursor_row = self.cursor_row;
        if clear {
            self.max_lines_rendered = lines.len();
        } else {
            self.max_lines_rendered = self.max_lines_rendered.max(lines.len());
        }
        let buffer_len = (self.previous_height as usize).max(lines.len());
        self.previous_viewport_top = if buffer_len > self.previous_height as usize {
            buffer_len - self.previous_height as usize
        } else {
            0
        };
    }

    // ── Overlay Compositing ──

    fn composite_overlays(
        &self,
        mut lines: Vec<String>,
        term_width: u16,
        term_height: u16,
    ) -> Vec<String> {
        if self.overlay_stack.is_empty() {
            return lines;
        }

        let mut visible: Vec<&OverlayStackEntry> = self
            .overlay_stack
            .iter()
            .filter(|e| !*e.hidden.borrow() && Self::is_overlay_visible(e, term_width, term_height))
            .collect();
        visible.sort_by_key(|e| e.focus_order);

        let mut rendered: Vec<(usize, usize, u16, Vec<String>)> = Vec::new();
        let mut min_lines_needed = lines.len();

        for entry in &visible {
            let opts = entry.options.as_ref();
            let mut overlay_width = opts.and_then(|o| o.width).unwrap_or(80).min(term_width);
            if let Some(min_w) = opts.and_then(|o| o.min_width) {
                overlay_width = overlay_width.max(min_w);
            }
            overlay_width = overlay_width.min(term_width).max(1);

            let mut overlay_lines = entry.component.render(overlay_width);
            if let Some(max_h) = opts.and_then(|o| o.max_height) {
                overlay_lines.truncate(max_h.min(term_height) as usize);
            }

            let (row, col) = self.resolve_overlay_position(
                opts,
                overlay_lines.len(),
                term_width,
                term_height,
                overlay_width,
            );
            min_lines_needed = min_lines_needed.max(row + overlay_lines.len());
            rendered.push((row, col, overlay_width, overlay_lines));
        }

        let working_height = lines.len().max(term_height as usize).max(min_lines_needed);
        while lines.len() < working_height {
            lines.push(String::new());
        }
        let viewport_start = if working_height > term_height as usize {
            working_height - term_height as usize
        } else {
            0
        };

        for (row, col, overlay_width, overlay_lines) in &rendered {
            for (i, overlay_line) in overlay_lines.iter().enumerate() {
                let idx = viewport_start + row + i;
                if idx < lines.len() {
                    let truncated = if visible_width(overlay_line) > *overlay_width as usize {
                        slice_with_width(overlay_line, 0, *overlay_width as usize, true).0
                    } else {
                        overlay_line.clone()
                    };
                    lines[idx] = self.composite_line_at(
                        &lines[idx],
                        &truncated,
                        *col,
                        *overlay_width as usize,
                        term_width as usize,
                    );
                }
            }
        }

        lines
    }

    fn composite_line_at(
        &self,
        base_line: &str,
        overlay_line: &str,
        start_col: usize,
        overlay_width: usize,
        total_width: usize,
    ) -> String {
        if is_image_line(base_line) {
            return base_line.to_string();
        }
        let after_start = start_col + overlay_width;
        let seg = extract_segments(
            base_line,
            start_col,
            after_start,
            total_width.saturating_sub(after_start),
            true,
        );
        let (overlay_text, overlay_w) = slice_with_width(overlay_line, 0, overlay_width, true);
        let before_pad = start_col.saturating_sub(seg.before_width);
        let overlay_pad = overlay_width.saturating_sub(overlay_w);
        let actual_before = start_col.max(seg.before_width);
        let actual_overlay = overlay_width.max(overlay_w);
        let after_target = total_width
            .saturating_sub(actual_before)
            .saturating_sub(actual_overlay);
        let after_pad = after_target.saturating_sub(seg.after_width);

        let result = format!(
            "{}{}{}{}{}{}{}{}",
            seg.before,
            " ".repeat(before_pad),
            SEGMENT_RESET,
            overlay_text,
            " ".repeat(overlay_pad),
            SEGMENT_RESET,
            seg.after,
            " ".repeat(after_pad)
        );

        let result_width = visible_width(&result);
        if result_width <= total_width {
            result
        } else {
            slice_with_width(&result, 0, total_width, true).0
        }
    }

    fn resolve_overlay_position(
        &self,
        opts: Option<&OverlayOptions>,
        overlay_height: usize,
        term_width: u16,
        term_height: u16,
        overlay_width: u16,
    ) -> (usize, usize) {
        let default_opts = OverlayOptions::default();
        let o = opts.unwrap_or(&default_opts);
        let margin = o.margin.unwrap_or(OverlayMargin::all(0));
        let margin_top = margin.top as usize;
        let margin_left = margin.left as usize;
        let margin_right = margin.right as usize;
        let margin_bottom = margin.bottom as usize;

        let avail_width = (term_width as usize)
            .saturating_sub(margin_left + margin_right)
            .max(1);
        let avail_height = (term_height as usize)
            .saturating_sub(margin_top + margin_bottom)
            .max(1);
        let effective_height = overlay_height.min(avail_height);

        let row = if let Some(r) = o.row {
            r as usize
        } else {
            match o.anchor {
                OverlayAnchor::TopLeft | OverlayAnchor::TopCenter | OverlayAnchor::TopRight => {
                    margin_top
                }
                OverlayAnchor::BottomLeft
                | OverlayAnchor::BottomCenter
                | OverlayAnchor::BottomRight => margin_top + avail_height - effective_height,
                _ => margin_top + (avail_height - effective_height) / 2,
            }
        };

        let col = if let Some(c) = o.col {
            c as usize
        } else {
            match o.anchor {
                OverlayAnchor::TopLeft | OverlayAnchor::LeftCenter | OverlayAnchor::BottomLeft => {
                    margin_left
                }
                OverlayAnchor::TopRight
                | OverlayAnchor::RightCenter
                | OverlayAnchor::BottomRight => margin_left + avail_width - overlay_width as usize,
                _ => margin_left + (avail_width - overlay_width as usize) / 2,
            }
        };

        let row = (row as i32 + o.offset_y as i32)
            .max(margin_top as i32)
            .min((term_height as usize - margin_bottom - effective_height) as i32)
            as usize;
        let col = (col as i32 + o.offset_x as i32)
            .max(margin_left as i32)
            .min((term_width as usize - margin_right - overlay_width as usize) as i32)
            as usize;

        (row, col)
    }

    fn extract_cursor_position(
        &self,
        lines: &mut Vec<String>,
        height: u16,
    ) -> Option<(usize, usize)> {
        let viewport_top = if lines.len() > height as usize {
            lines.len() - height as usize
        } else {
            0
        };
        for row in (viewport_top..lines.len()).rev() {
            if let Some(marker_idx) = lines[row].find(CURSOR_MARKER) {
                let col = visible_width(&lines[row][..marker_idx]);
                lines[row] = format!(
                    "{}{}",
                    &lines[row][..marker_idx],
                    &lines[row][marker_idx + CURSOR_MARKER.len()..]
                );
                return Some((row, col));
            }
        }
        None
    }

    fn position_hardware_cursor(&mut self, cursor_pos: Option<(usize, usize)>, total_lines: usize) {
        let (target_row, target_col) = match cursor_pos {
            Some((r, c)) if total_lines > 0 => (r.min(total_lines - 1), c),
            _ => {
                self.terminal.hide_cursor();
                return;
            }
        };
        let row_delta = target_row as i16 - self.hardware_cursor_row as i16;
        let mut buf = String::new();
        if row_delta > 0 {
            buf.push_str(&format!("\x1b[{}B", row_delta));
        } else if row_delta < 0 {
            buf.push_str(&format!("\x1b[{}A", -row_delta));
        }
        buf.push_str(&format!("\x1b[{}G", target_col + 1));
        if !buf.is_empty() {
            self.terminal.write(&buf);
        }
        self.hardware_cursor_row = target_row;
        if self.show_hardware_cursor {
            self.terminal.show_cursor();
        } else {
            self.terminal.hide_cursor();
        }
    }

    fn collect_kitty_image_ids(&self, lines: &[String]) -> HashSet<u32> {
        let mut ids = HashSet::new();
        for line in lines {
            for id in extract_kitty_image_ids(line) {
                ids.insert(id);
            }
        }
        ids
    }

    fn delete_changed_kitty_images(&self, first_changed: usize, last_changed: usize) -> String {
        if first_changed >= self.previous_lines.len() {
            return String::new();
        }
        let max_line = last_changed.min(self.previous_lines.len() - 1);
        let mut ids = HashSet::new();
        for i in first_changed..=max_line {
            if let Some(line) = self.previous_lines.get(i) {
                for id in extract_kitty_image_ids(line) {
                    ids.insert(id);
                }
            }
        }
        let mut buf = String::new();
        for id in ids {
            buf.push_str(&delete_kitty_image(id));
        }
        buf
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::virtual_terminal::VirtualTerminal;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct RecordingComponent {
        inputs: Rc<RefCell<Vec<String>>>,
    }

    struct SharedTextComponent {
        text: Rc<RefCell<String>>,
    }

    impl Component for RecordingComponent {
        fn render(&self, _width: u16) -> Vec<String> {
            Vec::new()
        }

        fn handle_input(&mut self, data: &str) {
            self.inputs.borrow_mut().push(data.to_string());
        }

        fn invalidate(&mut self) {}
    }

    impl Component for SharedTextComponent {
        fn render(&self, _width: u16) -> Vec<String> {
            vec![self.text.borrow().clone()]
        }

        fn invalidate(&mut self) {}
    }

    #[test]
    fn handle_input_routes_only_to_focused_child() {
        let terminal = Box::new(VirtualTerminal::new(80, 24));
        let mut tui = TUI::new(terminal);
        let first_inputs = Rc::new(RefCell::new(Vec::new()));
        let second_inputs = Rc::new(RefCell::new(Vec::new()));

        tui.add_child(Box::new(RecordingComponent {
            inputs: Rc::clone(&first_inputs),
        }));
        tui.add_child(Box::new(RecordingComponent {
            inputs: Rc::clone(&second_inputs),
        }));

        tui.handle_input("ignored");
        assert!(first_inputs.borrow().is_empty());
        assert!(second_inputs.borrow().is_empty());

        tui.set_focus(Some(0));
        tui.handle_input("first");
        assert_eq!(first_inputs.borrow().as_slice(), &["first"]);
        assert!(second_inputs.borrow().is_empty());

        tui.set_focus(Some(1));
        tui.handle_input("second");
        assert_eq!(first_inputs.borrow().as_slice(), &["first"]);
        assert_eq!(second_inputs.borrow().as_slice(), &["second"]);
    }

    #[test]
    fn resize_updates_terminal_dimensions_and_requests_render() {
        let terminal = Box::new(VirtualTerminal::new(80, 24));
        let mut tui = TUI::new(terminal);

        tui.resize(120, 40);

        assert_eq!(tui.columns(), 120);
        assert_eq!(tui.rows(), 40);
    }

    #[test]
    fn rapid_render_requests_are_not_dropped() {
        let terminal = VirtualTerminal::new(80, 24);
        let inspector = terminal.clone();
        let text = Rc::new(RefCell::new("first".to_string()));
        let mut tui = TUI::new(Box::new(terminal));
        tui.add_child(Box::new(SharedTextComponent {
            text: Rc::clone(&text),
        }));

        tui.start();
        *text.borrow_mut() = "second".to_string();
        tui.request_render();

        assert!(inspector
            .get_viewport()
            .iter()
            .any(|line| line.contains("second")));
    }
}

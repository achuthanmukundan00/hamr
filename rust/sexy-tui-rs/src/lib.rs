//! sexy-tui-rs — Rust port of @earendil-works/pi-tui
//!
//! A minimal terminal UI framework with differential rendering,
//! synchronized output, and an enhanced declarative theming system.
//!
//! Forked and ported to Rust by @achuthanmukundan00.

pub mod autocomplete;
pub mod editor_component;
pub mod fuzzy;
pub mod keybindings;
pub mod keys;
pub mod kill_ring;
pub mod native_modifiers;
pub mod stdin_buffer;
pub mod terminal;
pub mod terminal_colors;
pub mod terminal_image;
pub mod theme;
pub mod tui;
pub mod undo_stack;
pub mod utils;
pub mod virtual_terminal;
pub mod widgets;
pub mod word_navigation;

// Re-exports matching the TS src/index.ts public API
pub use autocomplete::{
    AutocompleteItem, AutocompleteProvider, AutocompleteSuggestions, CombinedAutocompleteProvider,
    SlashCommand,
};
pub use editor_component::EditorComponent;
pub use fuzzy::{fuzzy_filter, fuzzy_match, FuzzyMatch};
pub use keybindings::{
    get_keybindings, set_keybindings, Keybinding, KeybindingConflict, KeybindingDefinition,
    KeybindingDefinitions, Keybindings, KeybindingsConfig, KeybindingsManager, TUI_KEYBINDINGS,
};
pub use keys::{
    decode_kitty_printable, is_key_release, is_key_repeat, is_kitty_protocol_active, matches_key,
    parse_key, set_kitty_protocol_active, Key, KeyEventType,
};
pub use stdin_buffer::{StdinBuffer, StdinBufferOptions};
pub use terminal::{
    is_keyboard_protocol_negotiation_sequence_prefix, parse_keyboard_protocol_negotiation_sequence,
    ProcessTerminal, Terminal,
};
pub use terminal_colors::{
    parse_osc11_background_color, parse_terminal_color_scheme_report, RgbColor, TerminalColorScheme,
};
pub use terminal_image::{
    allocate_image_id, calculate_image_rows, delete_all_kitty_images, delete_kitty_image,
    detect_capabilities, encode_iterm2, encode_kitty, get_capabilities, get_cell_dimensions,
    get_gif_dimensions, get_image_dimensions, get_jpeg_dimensions, get_png_dimensions,
    get_webp_dimensions, hyperlink, image_fallback, is_image_line, render_image,
    reset_capabilities_cache, set_capabilities, set_cell_dimensions, CellDimensions,
    ImageDimensions, ImageProtocol, ImageRenderOptions, TerminalCapabilities,
};
pub use tui::{
    Component, Container, Focusable, OverlayAnchor, OverlayHandle, OverlayMargin, OverlayOptions,
    OverlayUnfocusOptions, CURSOR_MARKER, TUI,
};
pub use utils::{truncate_to_width, visible_width, wrap_text_with_ansi};
pub use widgets::{
    BorderStyle, CancellableLoader, Card, Editor, EditorOptions, EditorTheme, Image, ImageOptions,
    ImageTheme, Input, Loader, LoaderIndicatorOptions, Markdown, MarkdownOptions, MarkdownTheme,
    Panel, SelectItem, SelectList, SelectListTheme, SettingItem, SettingsList, SettingsListTheme,
    Spacer, Text, TruncatedText,
};

/// Deprecated alias — use `Panel` instead.
#[deprecated(
    since = "0.1.1",
    note = "Renamed to Panel to avoid shadowing std::boxed::Box"
)]
pub type Box = Panel;

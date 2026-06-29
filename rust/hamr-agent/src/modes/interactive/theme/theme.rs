//! Theme system — colors, card config, model brand, syntax highlighting.
//!
//! Port of `packages/coding-agent/src/modes/interactive/theme/theme.ts`.

use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// ThemeColor / ThemeBg type aliases
// ============================================================================

/// Foreground color token names.
pub type ThemeColor = &'static str;

/// Background color token names.
pub type ThemeBg = &'static str;

// ============================================================================
// CardConfig
// ============================================================================

/// Resolved message-card presentation. Drives how user/assistant/tool cards are
/// laid out (labels, glyph, indents, padding, shading) so the look lives in the
/// theme (data) rather than hardcoded in the components.
#[derive(Debug, Clone)]
pub struct CardConfig {
    /// Whether to render the glyph + label heading above card bodies.
    pub show_headings: bool,
    /// "model" → active model glyph; "" → no glyph; any other string → literal glyph.
    pub heading_glyph: String,
    /// Glyph used for prompt (user message) card headings. "model" → active model
    /// glyph; "" → no glyph; any other string → literal glyph. Default: "⚒".
    pub prompt_heading_glyph: Option<String>,
    pub prompt_label: String,
    pub response_label: String,
    pub thought_label: String,
    /// Left padding of the heading label within the card.
    pub heading_indent: u16,
    /// Left padding of the body (markdown) within the card.
    pub body_indent: u16,
    /// Left indent applied to tool card headings within the card Box.
    pub tool_indent: u16,
    /// Left indent applied to tool card results (visually separates result from heading).
    pub tool_result_indent: u16,
    /// Horizontal/vertical padding of the card Box.
    pub card_pad_x: u16,
    pub card_pad_y: u16,
    /// Whether message/tool cards paint full-width background surfaces.
    pub shaded_surfaces: bool,
    /// Whether the THOUGHT card uses the dedicated thinking background when shaded surfaces are enabled.
    pub thinking_shaded: bool,
    /// When true, no spacer is inserted between consecutive cards.
    pub gapless_cards: bool,
    /// Whether to render the THOUGHT heading in thinking cards.
    pub show_thought_heading: bool,
}

impl Default for CardConfig {
    fn default() -> Self {
        Self {
            show_headings: true,
            heading_glyph: "model".to_string(),
            prompt_heading_glyph: Some("⚒".to_string()),
            prompt_label: "PROMPT".to_string(),
            response_label: "RESPONSE".to_string(),
            thought_label: "THOUGHT".to_string(),
            heading_indent: 1,
            body_indent: 1,
            tool_indent: 1,
            tool_result_indent: 1,
            card_pad_x: 1,
            card_pad_y: 1,
            shaded_surfaces: false,
            thinking_shaded: false,
            gapless_cards: false,
            show_thought_heading: true,
        }
    }
}

// ============================================================================
// ModelBrand
// ============================================================================

#[derive(Debug, Clone)]
pub struct ModelBrand {
    pub color: &'static str,
    pub emoji: &'static str,
    pub nerd: &'static str,
    pub unicode: &'static str,
    pub ascii_glyph: &'static str,
}

/// Determine the model brand for a provider + model label.
pub fn model_brand_for(provider: &str, model_label: Option<&str>) -> ModelBrand {
    let lower = format!(
        "{} {}",
        provider.to_lowercase(),
        model_label.unwrap_or("").to_lowercase()
    );

    if lower.contains("claude")
        || lower.contains("opus")
        || lower.contains("sonnet")
        || lower.contains("haiku")
        || lower.contains("fable")
        || lower.contains("mythos")
        || lower.contains("anthropic")
    {
        ModelBrand {
            color: "#d08030",
            emoji: "✳",
            nerd: "✳",
            unicode: "✳",
            ascii_glyph: "C",
        }
    } else if lower.contains("mistral") || lower.contains("codestral") || lower.contains("devstral")
    {
        ModelBrand {
            color: "#f06030",
            emoji: "◧",
            nerd: "◧",
            unicode: "◧",
            ascii_glyph: "M",
        }
    } else if lower.contains("deepseek") {
        ModelBrand {
            color: "#005faf",
            emoji: "🐋",
            nerd: "🐋",
            unicode: "◗",
            ascii_glyph: "D",
        }
    } else if lower.contains("gemma") {
        ModelBrand {
            color: "#5098e8",
            emoji: "✧",
            nerd: "✧",
            unicode: "✧",
            ascii_glyph: "g",
        }
    } else if lower.contains("gemini") || lower.contains("google") {
        ModelBrand {
            color: "#4285f4",
            emoji: "✦",
            nerd: "✦",
            unicode: "✦",
            ascii_glyph: "G",
        }
    } else if lower.contains("qwen") {
        ModelBrand {
            color: "#875fff",
            emoji: "⬡",
            nerd: "⬡",
            unicode: "⬡",
            ascii_glyph: "Q",
        }
    } else if lower.contains("glm") || lower.contains("zhipu") || lower.contains("zai") {
        ModelBrand {
            color: "#00afaf",
            emoji: "◎",
            nerd: "◎",
            unicode: "◎",
            ascii_glyph: "Z",
        }
    } else if lower.contains("llama") || lower.contains("meta") {
        ModelBrand {
            color: "#0087ff",
            emoji: "∞",
            nerd: "∞",
            unicode: "∞",
            ascii_glyph: "L",
        }
    } else if lower.contains("minimax") {
        ModelBrand {
            color: "#ff4444",
            emoji: "▽",
            nerd: "▽",
            unicode: "▽",
            ascii_glyph: "I",
        }
    } else if lower.contains("grok") || lower.contains("xai") || lower.contains("groq") {
        ModelBrand {
            color: "#eeeeee",
            emoji: "✕",
            nerd: "✕",
            unicode: "✕",
            ascii_glyph: "X",
        }
    } else if lower.contains("moonshot") || lower.contains("kimi") {
        ModelBrand {
            color: "#aaaaaa",
            emoji: "☾",
            nerd: "☾",
            unicode: "☾",
            ascii_glyph: "K",
        }
    } else if lower.contains("gpt") || lower.contains("openai") || lower.contains("codex") {
        ModelBrand {
            color: "#cccccc",
            emoji: "❁",
            nerd: "❁",
            unicode: "❁",
            ascii_glyph: "O",
        }
    } else {
        ModelBrand {
            color: "#61afef",
            emoji: "◆",
            nerd: "◆",
            unicode: "◆",
            ascii_glyph: "?",
        }
    }
}

// ============================================================================
// Color Utilities
// ============================================================================

/// Parse a hex color string to (r, g, b).
fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

type ColorFn = Arc<dyn Fn(&str) -> String + Send + Sync>;

// ============================================================================
// Theme
// ============================================================================

/// The resolved theme with foreground/background colors and card layout.
pub struct Theme {
    pub name: Option<String>,
    pub source_path: Option<String>,
    /// Whether to tint card surfaces with model brand colors.
    pub model_adaptive: bool,
    /// Card layout configuration.
    pub cards: CardConfig,
    /// Foreground color map: token name → ANSI escape.
    fg_colors: HashMap<&'static str, ColorFn>,
    /// Background color map: token name → ANSI escape.
    bg_colors: HashMap<&'static str, ColorFn>,
    /// Background hex color map: token name → raw hex string.
    bg_hex: HashMap<&'static str, String>,
}

/// Helper to build a ColorFn from a hex color + foreground/background toggle.
fn hex_to_ansi_fn(hex: &str, is_bg: bool) -> ColorFn {
    let (r, g, b) = hex_to_rgb(hex);
    let ansi = if is_bg {
        format!("\x1b[48;2;{};{};{}m", r, g, b)
    } else {
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    };
    let reset = if is_bg { "\x1b[49m" } else { "\x1b[39m" };
    Arc::new(move |s: &str| -> String { format!("{}{}{}", ansi, s, reset) })
}

/// Helper to build an identity ColorFn (no styling).
fn identity_fn() -> ColorFn {
    Arc::new(|s: &str| -> String { s.to_string() })
}

impl Theme {
    /// Create a theme with the given resolved colors.
    pub fn new(
        name: Option<String>,
        source_path: Option<String>,
        model_adaptive: bool,
        cards: CardConfig,
        fg_colors: HashMap<&'static str, String>,
        bg_colors: HashMap<&'static str, String>,
    ) -> Self {
        let fg_fns: HashMap<&'static str, ColorFn> = fg_colors
            .iter()
            .map(|(k, v)| (*k, hex_to_ansi_fn(v, false)))
            .collect();

        let mut bg_fns: HashMap<&'static str, ColorFn> = HashMap::new();
        let mut bg_hex_map: HashMap<&'static str, String> = HashMap::new();
        for (k, v) in &bg_colors {
            bg_fns.insert(*k, hex_to_ansi_fn(v, true));
            bg_hex_map.insert(*k, v.clone());
        }

        Theme {
            name,
            source_path,
            model_adaptive,
            cards,
            fg_colors: fg_fns,
            bg_colors: bg_fns,
            bg_hex: bg_hex_map,
        }
    }

    /// Create a default "hamr" theme.
    pub fn default_theme() -> Self {
        let mut fg = HashMap::new();
        let mut bg = HashMap::new();

        // Core UI colors
        fg.insert("accent", "#875fff".to_string());
        fg.insert("border", "#444444".to_string());
        fg.insert("borderAccent", "#875fff".to_string());
        fg.insert("borderMuted", "#333333".to_string());
        fg.insert("success", "#98c379".to_string());
        fg.insert("error", "#e06c75".to_string());
        fg.insert("warning", "#e5c07b".to_string());
        fg.insert("muted", "#5c6370".to_string());
        fg.insert("dim", "#3b4048".to_string());
        fg.insert("text", "#abb2bf".to_string());
        fg.insert("thinkingText", "#5c6370".to_string());

        // Content text
        fg.insert("userMessageText", "#abb2bf".to_string());
        fg.insert("customMessageText", "#abb2bf".to_string());
        fg.insert("customMessageLabel", "#875fff".to_string());
        fg.insert("toolTitle", "#e5c07b".to_string());
        fg.insert("toolOutput", "#abb2bf".to_string());

        // Markdown
        fg.insert("mdHeading", "#e5c07b".to_string());
        fg.insert("mdLink", "#61afef".to_string());
        fg.insert("mdLinkUrl", "#5c6370".to_string());
        fg.insert("mdCode", "#e06c75".to_string());
        fg.insert("mdCodeBlock", "#abb2bf".to_string());
        fg.insert("mdCodeBlockBorder", "#444444".to_string());
        fg.insert("mdQuote", "#5c6370".to_string());
        fg.insert("mdQuoteBorder", "#444444".to_string());
        fg.insert("mdHr", "#444444".to_string());
        fg.insert("mdListBullet", "#875fff".to_string());

        // Tool diffs
        fg.insert("toolDiffAdded", "#98c379".to_string());
        fg.insert("toolDiffRemoved", "#e06c75".to_string());
        fg.insert("toolDiffContext", "#abb2bf".to_string());

        // Syntax
        fg.insert("syntaxComment", "#5c6370".to_string());
        fg.insert("syntaxKeyword", "#c678dd".to_string());
        fg.insert("syntaxFunction", "#61afef".to_string());
        fg.insert("syntaxVariable", "#e06c75".to_string());
        fg.insert("syntaxString", "#98c379".to_string());
        fg.insert("syntaxNumber", "#d19a66".to_string());
        fg.insert("syntaxType", "#e5c07b".to_string());
        fg.insert("syntaxOperator", "#56b6c2".to_string());
        fg.insert("syntaxPunctuation", "#abb2bf".to_string());

        // Thinking levels
        fg.insert("thinkingOff", "#444444".to_string());
        fg.insert("thinkingMinimal", "#5c6370".to_string());
        fg.insert("thinkingLow", "#61afef".to_string());
        fg.insert("thinkingMedium", "#98c379".to_string());
        fg.insert("thinkingHigh", "#e5c07b".to_string());
        fg.insert("thinkingXhigh", "#c678dd".to_string());

        // Bash mode
        fg.insert("bashMode", "#e06c75".to_string());

        // Backgrounds
        bg.insert("selectedBg", "#3e3e3e".to_string());
        bg.insert("userMessageBg", "#282c34".to_string());
        bg.insert("customMessageBg", "#282c34".to_string());
        bg.insert("toolPendingBg", "#282c34".to_string());
        bg.insert("toolSuccessBg", "#282c34".to_string());
        bg.insert("toolErrorBg", "#282c34".to_string());
        bg.insert("toolDiffAddedBg", "#2c3e2c".to_string());
        bg.insert("toolDiffRemovedBg", "#3e2c2c".to_string());
        bg.insert("toolWarningBg", "#282c34".to_string());
        bg.insert("editorBg", "#1e1e1e".to_string());
        bg.insert("editorSelection", "#3e3e3e".to_string());
        bg.insert("statusBarBg", "#21252b".to_string());
        bg.insert("surfaceBg", "#282c34".to_string());
        bg.insert("cardBg", "#282c34".to_string());
        bg.insert("thinkingBg", "#282c34".to_string());

        // Editor colors
        fg.insert("editorFg", "#abb2bf".to_string());
        fg.insert("editorCursor", "#528bff".to_string());
        fg.insert("editorLineNumber", "#5c6370".to_string());

        Theme::new(
            Some("hamr".to_string()),
            None,
            true,
            CardConfig::default(),
            fg,
            bg,
        )
    }

    /// Apply a foreground color to text.
    pub fn fg(&self, color: ThemeColor, text: &str) -> String {
        self.fg_colors
            .get(color)
            .map(|f| f(text))
            .unwrap_or_else(|| text.to_string())
    }

    /// Apply a background color to text.
    pub fn bg(&self, color: ThemeBg, text: &str) -> String {
        self.bg_colors
            .get(color)
            .map(|f| f(text))
            .unwrap_or_else(|| text.to_string())
    }

    /// Get the raw ANSI background escape for a background token.
    pub fn get_bg_ansi(&self, token: ThemeBg) -> String {
        self.bg_hex
            .get(token)
            .map(|hex| {
                let (r, g, b) = hex_to_rgb(hex);
                format!("\x1b[48;2;{};{};{}m", r, g, b)
            })
            .unwrap_or_else(|| "\x1b[49m".to_string())
    }

    /// Get the raw hex for a background color key (for color blending).
    pub fn get_bg_hex(&self, color: ThemeBg) -> Option<&str> {
        self.bg_hex.get(color).map(|s| s.as_str())
    }

    /// Wrap text in bold.
    pub fn bold(&self, text: &str) -> String {
        format!("\x1b[1m{}\x1b[22m", text)
    }

    /// Wrap text in italic.
    pub fn italic(&self, text: &str) -> String {
        format!("\x1b[3m{}\x1b[23m", text)
    }

    /// Wrap text in underline.
    pub fn underline(&self, text: &str) -> String {
        format!("\x1b[4m{}\x1b[24m", text)
    }

    // -----------------------------------------------------------------------
    // Model brand helpers
    // -----------------------------------------------------------------------

    /// Get the model brand for a provider + label.
    pub fn model_brand(&self, provider: &str, model_label: Option<&str>) -> ModelBrand {
        model_brand_for(provider, model_label)
    }

    /// Get the model glyph for the terminal's glyph tier.
    pub fn model_glyph(&self, provider: &str, model_label: Option<&str>) -> String {
        let brand = self.model_brand(provider, model_label);
        brand.unicode.to_string()
    }

    /// Brand accent color for a model provider + label.
    /// Returns an ANSI foreground escape.
    pub fn model_color(&self, provider: &str, model_label: Option<&str>) -> String {
        if !self.model_adaptive {
            return "\x1b[39m".to_string();
        }
        let brand = model_brand_for(provider, model_label);
        let (r, g, b) = hex_to_rgb(brand.color);
        format!("\x1b[38;2;{};{};{}m", r, g, b)
    }

    /// Returns the hex color (without ANSI wrapping) for a model's brand identity.
    pub fn model_hex_color(&self, provider: &str, model_label: Option<&str>) -> Option<String> {
        if !self.model_adaptive {
            return None;
        }
        let brand = model_brand_for(provider, model_label);
        Some(brand.color.to_string())
    }

    // -----------------------------------------------------------------------
    // Background blending
    // -----------------------------------------------------------------------

    /// Blend a model accent hex into a theme background to tint card surfaces
    /// with the model's brand color.
    ///
    /// The blend is ~12% model color into the base background so the tint is
    /// subtle and stays readable on both dark and light terminals.
    pub fn model_adaptive_bg(&self, accent_hex: &str, bg_hex: &str) -> String {
        let (ar, ag, ab) = hex_to_rgb(accent_hex);
        let (br, bg, bb) = hex_to_rgb(bg_hex);
        let ratio = 0.12;
        let r = (br as f64 * (1.0 - ratio) + ar as f64 * ratio).round() as u8;
        let g = (bg as f64 * (1.0 - ratio) + ag as f64 * ratio).round() as u8;
        let b = (bb as f64 * (1.0 - ratio) + ab as f64 * ratio).round() as u8;
        rgb_to_hex(r, g, b)
    }

    /// Returns a background function that tints the card surface with the
    /// model's brand accent color. Falls back to the plain bg when
    /// modelAdaptive is off or no accent is provided.
    pub fn model_adaptive_bg_fn(
        &self,
        accent_hex: Option<&str>,
        bg_key: ThemeBg,
    ) -> Option<ColorFn> {
        if !self.model_adaptive || accent_hex.is_none() {
            return self.bg_colors.get(bg_key).cloned();
        }
        let accent = accent_hex.unwrap();
        let base_hex = self.get_bg_hex(bg_key)?;
        let blended = self.model_adaptive_bg(accent, base_hex);
        Some(hex_to_ansi_fn(&blended, true))
    }

    // -----------------------------------------------------------------------
    // Thinking / editor border colors
    // -----------------------------------------------------------------------

    /// Get thinking border color function for a thinking level.
    pub fn get_thinking_border_color(&self, level: &str) -> ColorFn {
        let color_key = match level {
            "off" => "thinkingOff",
            "minimal" => "thinkingMinimal",
            "low" => "thinkingLow",
            "medium" => "thinkingMedium",
            "high" => "thinkingHigh",
            "xhigh" => "thinkingXhigh",
            _ => "thinkingOff",
        };
        self.fg_colors
            .get(color_key)
            .cloned()
            .unwrap_or_else(identity_fn)
    }

    /// Get bash mode border color function.
    pub fn get_bash_mode_border_color(&self) -> ColorFn {
        self.fg_colors
            .get("bashMode")
            .cloned()
            .unwrap_or_else(identity_fn)
    }

    /// Editor border color derived from model brand hex × thinking brightness.
    /// Returns None when modelAdaptive is false — callers should fall
    /// back to get_thinking_border_color() in that case.
    pub fn get_model_editor_border_color(
        &self,
        provider: &str,
        model_id: Option<&str>,
        thinking_level: Option<&str>,
    ) -> Option<ColorFn> {
        if !self.model_adaptive {
            return None;
        }
        let hex = self.model_hex_color(provider, model_id)?;
        let (r, g, b) = hex_to_rgb(&hex);

        // Brightness multipliers per thinking level
        let mult = match thinking_level {
            Some("xhigh") => 1.0,
            Some("high") => 0.85,
            Some("medium") => 0.65,
            Some("low") => 0.45,
            _ => 0.3,
        };

        let r = (r as f64 * mult).round() as u8;
        let g = (g as f64 * mult).round() as u8;
        let b = (b as f64 * mult).round() as u8;

        Some(hex_to_ansi_fn(&rgb_to_hex(r, g, b), false))
    }
}

// ============================================================================
// Global Theme Instance
// ============================================================================

use std::sync::RwLock;

static GLOBAL_THEME: RwLock<Option<Theme>> = RwLock::new(None);

/// Initialize the global theme instance.
pub fn init_theme(theme: Theme) {
    let mut guard = GLOBAL_THEME.write().unwrap();
    *guard = Some(theme);
}

/// Get a reference to the global theme.
/// Panics if the theme hasn't been initialized.
pub fn get_theme() -> &'static Theme {
    // Since we can't return a reference from RwLock, we use a lazy static approach.
    // In the real app, init_theme() is called at startup.
    static DEFAULT: std::sync::OnceLock<Theme> = std::sync::OnceLock::new();
    DEFAULT.get_or_init(|| Theme::default_theme())
}

/// Legacy static reference to the global theme.
/// Exists for backward compatibility with existing component stubs.
/// In production, use `get_theme()`.
pub fn theme() -> &'static Theme {
    static DEFAULT: std::sync::LazyLock<Theme> =
        std::sync::LazyLock::new(|| Theme::default_theme());
    &DEFAULT
}

// ============================================================================
// Markdown and SelectList themes
// ============================================================================

use crate::modes::interactive::components::tui_shim::MarkdownTheme;

/// Get the markdown theme.
pub fn get_markdown_theme() -> MarkdownTheme {
    MarkdownTheme::default()
}

// ============================================================================
// Syntax highlighting helpers
// ============================================================================

/// Try to detect a syntax-highlighting language from a file path.
pub fn get_language_from_path(file_path: &str) -> Option<String> {
    let ext = file_path.rsplit('.').next()?.to_lowercase();

    let lang = match ext.as_str() {
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "rb" => "ruby",
        "rs" => "rust",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "cs" => "csharp",
        "php" => "php",
        "sh" | "bash" | "zsh" => "bash",
        "fish" => "fish",
        "ps1" => "powershell",
        "sql" => "sql",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "md" | "markdown" => "markdown",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "cmake" => "cmake",
        "lua" => "lua",
        "perl" => "perl",
        "r" => "r",
        "scala" => "scala",
        "clj" => "clojure",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "hs" => "haskell",
        "ml" => "ocaml",
        "vim" => "vim",
        "graphql" | "gql" => "graphql",
        "proto" => "protobuf",
        "tf" | "hcl" => "hcl",
        _ => return None,
    };

    Some(lang.to_string())
}

/// Highlight code with a given language. Returns vector of styled lines.
/// Stub: in the full implementation this would use syntect or tree-sitter.
pub fn highlight_code(code: &str, _lang: &str) -> Vec<String> {
    code.lines().map(|l| l.to_string()).collect()
}

// ============================================================================
// Backward-compatible re-exports for existing stubs that use THEME / cards
// ============================================================================

/// Backward-compatible: get a reference to the global theme.
/// Aliased as THEME for existing stubs.
pub fn get_available_theme_names() -> Vec<String> {
    vec!["hamr".to_string(), "dark".to_string(), "light".to_string()]
}

/// Global theme instance for component use.
pub static THEME: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(|| Theme::default_theme());

/// Legacy accessor for card config from the global theme.
pub fn cards() -> CardConfig {
    get_theme().cards.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_brand_claude() {
        let brand = model_brand_for("anthropic", Some("claude-sonnet"));
        assert_eq!(brand.color, "#d08030");
    }

    #[test]
    fn test_model_brand_gpt() {
        let brand = model_brand_for("openai", Some("gpt-4"));
        assert_eq!(brand.color, "#cccccc");
    }

    #[test]
    fn test_model_brand_fallback() {
        let brand = model_brand_for("unknown", None);
        assert_eq!(brand.color, "#61afef");
    }

    #[test]
    fn test_model_adaptive_bg() {
        let theme = Theme::default_theme();
        let result = theme.model_adaptive_bg("#ff0000", "#000000");
        // 12% red blended into black → dark red
        assert!(result.starts_with("#"));
        let (r, g, b) = hex_to_rgb(&result);
        assert!(r > 0);
        assert!(r < 64);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_hex_to_rgb() {
        assert_eq!(hex_to_rgb("#ff0000"), (255, 0, 0));
        assert_eq!(hex_to_rgb("#00ff00"), (0, 255, 0));
        assert_eq!(hex_to_rgb("#0000ff"), (0, 0, 255));
    }

    #[test]
    fn test_rgb_to_hex() {
        assert_eq!(rgb_to_hex(255, 0, 0), "#ff0000");
        assert_eq!(rgb_to_hex(0, 255, 0), "#00ff00");
        assert_eq!(rgb_to_hex(0, 0, 255), "#0000ff");
    }

    #[test]
    fn test_get_language_from_path() {
        assert_eq!(get_language_from_path("test.rs"), Some("rust".to_string()));
        assert_eq!(
            get_language_from_path("test.ts"),
            Some("typescript".to_string())
        );
        assert_eq!(
            get_language_from_path("test.py"),
            Some("python".to_string())
        );
        assert_eq!(get_language_from_path("test.foo"), None);
    }

    #[test]
    fn test_default_theme_fg() {
        let theme = Theme::default_theme();
        let result = theme.fg("accent", "hello");
        assert!(result.contains("hello"));
        assert!(result.contains("\x1b["));
    }
}

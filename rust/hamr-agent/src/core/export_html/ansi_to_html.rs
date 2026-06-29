//! Port of `packages/coding-agent/src/core/export-html/ansi-to-html.ts`.
//!
//! ANSI escape code to HTML converter.
//! Converts terminal ANSI color/style codes to HTML with inline styles.
//! Supports:
//! - Standard foreground colors (30-37) and bright variants (90-97)
//! - Standard background colors (40-47) and bright variants (100-107)
//! - 256-color palette (38;5;N and 48;5;N)
//! - RGB true color (38;2;R;G;B and 48;2;R;G;B)
//! - Text styles: bold (1), dim (2), italic (3), underline (4)
//! - Reset (0)

use regex::Regex;
use std::sync::LazyLock;

// Standard ANSI color palette (0-15)
const ANSI_COLORS: [&str; 16] = [
    "#000000", // 0: black
    "#800000", // 1: red
    "#008000", // 2: green
    "#808000", // 3: yellow
    "#000080", // 4: blue
    "#800080", // 5: magenta
    "#008080", // 6: cyan
    "#c0c0c0", // 7: white
    "#808080", // 8: bright black
    "#ff0000", // 9: bright red
    "#00ff00", // 10: bright green
    "#ffff00", // 11: bright yellow
    "#0000ff", // 12: bright blue
    "#ff00ff", // 13: bright magenta
    "#00ffff", // 14: bright cyan
    "#ffffff", // 15: bright white
];

/// Convert 256-color index to hex.
fn color256_to_hex(index: u32) -> String {
    // Standard colors (0-15)
    if index < 16 {
        return ANSI_COLORS[index as usize].to_string();
    }

    // Color cube (16-231): 6×6×6 = 216 colors
    if index < 232 {
        let cube_index = index - 16;
        let r = cube_index / 36;
        let g = (cube_index % 36) / 6;
        let b = cube_index % 6;
        let to_component = |n: u32| -> u32 { if n == 0 { 0 } else { 55 + n * 40 } };
        format!(
            "#{:02x}{:02x}{:02x}",
            to_component(r),
            to_component(g),
            to_component(b)
        )
    } else {
        // Grayscale (232-255): 24 shades
        let gray = 8 + (index - 232) * 10;
        format!("#{gray:02x}{gray:02x}{gray:02x}")
    }
}

/// Escape HTML special characters.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

#[derive(Debug, Clone, Default)]
struct TextStyle {
    fg: Option<String>,
    bg: Option<String>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
}

impl TextStyle {
    fn new() -> Self {
        Self::default()
    }

    fn to_inline_css(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ref fg) = self.fg {
            parts.push(format!("color:{fg}"));
        }
        if let Some(ref bg) = self.bg {
            parts.push(format!("background-color:{bg}"));
        }
        if self.bold {
            parts.push("font-weight:bold".to_string());
        }
        if self.dim {
            parts.push("opacity:0.6".to_string());
        }
        if self.italic {
            parts.push("font-style:italic".to_string());
        }
        if self.underline {
            parts.push("text-decoration:underline".to_string());
        }
        parts.join(";")
    }

    fn has_style(&self) -> bool {
        self.fg.is_some()
            || self.bg.is_some()
            || self.bold
            || self.dim
            || self.italic
            || self.underline
    }
}

/// Parse ANSI SGR (Select Graphic Rendition) codes and update style.
fn apply_sgr_code(params: &[u32], style: &mut TextStyle) {
    let mut i = 0usize;
    while i < params.len() {
        let code = params[i];

        match code {
            0 => {
                // Reset all
                *style = TextStyle::new();
            }
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            30..=37 => {
                style.fg = Some(ANSI_COLORS[(code - 30) as usize].to_string());
            }
            38 => {
                // Extended foreground color
                if i + 2 < params.len() && params[i + 1] == 5 {
                    // 256-color: 38;5;N
                    style.fg = Some(color256_to_hex(params[i + 2]));
                    i += 2;
                } else if i + 4 < params.len() && params[i + 1] == 2 {
                    // RGB: 38;2;R;G;B
                    let r = params[i + 2];
                    let g = params[i + 3];
                    let b = params[i + 4];
                    style.fg = Some(format!("rgb({r},{g},{b})"));
                    i += 4;
                }
            }
            39 => style.fg = None,
            40..=47 => {
                style.bg = Some(ANSI_COLORS[(code - 40) as usize].to_string());
            }
            48 => {
                // Extended background color
                if i + 2 < params.len() && params[i + 1] == 5 {
                    // 256-color: 48;5;N
                    style.bg = Some(color256_to_hex(params[i + 2]));
                    i += 2;
                } else if i + 4 < params.len() && params[i + 1] == 2 {
                    // RGB: 48;2;R;G;B
                    let r = params[i + 2];
                    let g = params[i + 3];
                    let b = params[i + 4];
                    style.bg = Some(format!("rgb({r},{g},{b})"));
                    i += 4;
                }
            }
            49 => style.bg = None,
            90..=97 => {
                style.fg = Some(ANSI_COLORS[(code - 90 + 8) as usize].to_string());
            }
            100..=107 => {
                style.bg = Some(ANSI_COLORS[(code - 100 + 8) as usize].to_string());
            }
            _ => {} // Ignore unrecognized codes
        }

        i += 1;
    }
}

// Match ANSI escape sequences: ESC[ followed by params and ending with 'm'
static ANSI_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[([\d;]*)m").unwrap());

/// Convert ANSI-escaped text to HTML with inline styles.
pub fn ansi_to_html(text: &str) -> String {
    let mut style = TextStyle::new();
    let mut result = String::new();
    let mut last_index = 0;
    let mut in_span = false;

    for caps in ANSI_REGEX.captures_iter(text) {
        let m = caps.get(0).unwrap();

        // Add text before this escape sequence
        let before_text = &text[last_index..m.start()];
        if !before_text.is_empty() {
            result.push_str(&escape_html(before_text));
        }

        // Parse SGR parameters
        let param_str = caps.get(1).map_or("", |p| p.as_str());
        let params: Vec<u32> = if param_str.is_empty() {
            vec![0]
        } else {
            param_str
                .split(';')
                .map(|p| p.parse::<u32>().unwrap_or(0))
                .collect()
        };

        // Close existing span if we have one
        if in_span {
            result.push_str("</span>");
            in_span = false;
        }

        // Apply the codes
        apply_sgr_code(&params, &mut style);

        // Open new span if we have any styling
        if style.has_style() {
            result.push_str(&format!("<span style=\"{}\">", style.to_inline_css()));
            in_span = true;
        }

        last_index = m.end();
    }

    // Add remaining text
    let remaining_text = &text[last_index..];
    if !remaining_text.is_empty() {
        result.push_str(&escape_html(remaining_text));
    }

    // Close any open span
    if in_span {
        result.push_str("</span>");
    }

    result
}

/// Convert array of ANSI-escaped lines to HTML.
/// Each line is wrapped in a div element.
pub fn ansi_lines_to_html(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| {
            let html = ansi_to_html(line);
            if html.is_empty() {
                r#"<div class="ansi-line">&nbsp;</div>"#.to_string()
            } else {
                format!(r#"<div class="ansi-line">{html}</div>"#)
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        assert_eq!(ansi_to_html("hello"), "hello");
    }

    #[test]
    fn test_bold_text() {
        assert_eq!(
            ansi_to_html("\x1b[1mbold\x1b[0m"),
            "<span style=\"font-weight:bold\">bold</span>"
        );
    }

    #[test]
    fn test_fg_color() {
        assert_eq!(
            ansi_to_html("\x1b[31mred\x1b[0m"),
            "<span style=\"color:#800000\">red</span>"
        );
    }

    #[test]
    fn test_bright_fg_color() {
        assert_eq!(
            ansi_to_html("\x1b[91mbright red\x1b[0m"),
            "<span style=\"color:#ff0000\">bright red</span>"
        );
    }

    #[test]
    fn test_bg_color() {
        assert_eq!(
            ansi_to_html("\x1b[42mgreen bg\x1b[0m"),
            "<span style=\"background-color:#008000\">green bg</span>"
        );
    }

    #[test]
    fn test_256_color() {
        // 38;5;196 = bright red in 256-color palette
        assert_eq!(
            ansi_to_html("\x1b[38;5;196m256 red\x1b[0m"),
            "<span style=\"color:#ff0000\">256 red</span>"
        );
    }

    #[test]
    fn test_rgb_color() {
        assert_eq!(
            ansi_to_html("\x1b[38;2;255;128;0mrgb orange\x1b[0m"),
            "<span style=\"color:rgb(255,128,0)\">rgb orange</span>"
        );
    }

    #[test]
    fn test_multiple_styles() {
        assert_eq!(
            ansi_to_html("\x1b[1;31mbold red\x1b[0m"),
            "<span style=\"color:#800000;font-weight:bold\">bold red</span>"
        );
    }

    #[test]
    fn test_html_escaping() {
        assert_eq!(ansi_to_html("<script>"), "&lt;script&gt;");
    }

    #[test]
    fn test_no_ansi_codes() {
        assert_eq!(ansi_to_html("just text"), "just text");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(ansi_to_html(""), "");
    }

    #[test]
    fn test_dim_and_italic() {
        assert_eq!(
            ansi_to_html("\x1b[2mdim\x1b[0m \x1b[3mitalic\x1b[0m"),
            "<span style=\"opacity:0.6\">dim</span> <span style=\"font-style:italic\">italic</span>"
        );
    }

    #[test]
    fn test_underline() {
        assert_eq!(
            ansi_to_html("\x1b[4munderlined\x1b[0m"),
            "<span style=\"text-decoration:underline\">underlined</span>"
        );
    }

    #[test]
    fn test_256_color_cube() {
        // 38;5;51 = cyan in cube (16+35 = 51, r=0 g=5 b=5 → 0, 255, 255)
        assert_eq!(
            ansi_to_html("\x1b[38;5;51mcube cyan\x1b[0m"),
            "<span style=\"color:#00ffff\">cube cyan</span>"
        );
    }

    #[test]
    fn test_256_color_grayscale() {
        // 38;5;232 = darkest gray
        let result = ansi_to_html("\x1b[38;5;232mgray\x1b[0m");
        assert!(result.contains("color:#080808"));
        assert!(result.contains("gray"));
    }

    #[test]
    fn test_ansi_lines_to_html() {
        let lines = vec!["\x1b[31mred\x1b[0m".to_string(), "plain".to_string()];
        let html = ansi_lines_to_html(&lines);
        assert!(html.contains(r#"<div class="ansi-line">"#));
        assert!(html.contains("red"));
        assert!(html.contains("plain"));
    }

    #[test]
    fn test_ansi_lines_empty_line() {
        let lines = vec!["".to_string()];
        let html = ansi_lines_to_html(&lines);
        assert!(html.contains("&nbsp;"));
    }

    #[test]
    fn test_ansi_lines_to_html_does_not_insert_source_whitespace_between_lines() {
        let lines = vec!["one".to_string(), "two".to_string()];
        assert_eq!(
            ansi_lines_to_html(&lines),
            r#"<div class="ansi-line">one</div><div class="ansi-line">two</div>"#
        );
    }
}

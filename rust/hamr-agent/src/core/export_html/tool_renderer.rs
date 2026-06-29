//! Port of `packages/coding-agent/src/core/export-html/tool-renderer.ts`.
//!
//! Tool HTML renderer for custom tools in HTML export.
//! Renders custom tool calls and results to HTML by invoking their TUI renderers
//! and converting the ANSI output to HTML.

use crate::core::export_html::ansi_to_html::ansi_lines_to_html;
use regex::Regex;
use std::sync::LazyLock;

// ANSI escape regex for blank-line detection
static ANSI_ESCAPE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[\d;]*m").unwrap());

/// Check if a rendered output line is blank (only ANSI escapes, no visible text).
fn is_blank_rendered_line(line: &str) -> bool {
    ANSI_ESCAPE_REGEX.replace_all(line, "").trim().is_empty()
}

/// Trim blank lines from start and end of rendered result.
fn trim_rendered_result_lines(lines: &[String]) -> Vec<String> {
    let mut start = 0i32;
    let mut end = lines.len() as i32;
    while start < end && is_blank_rendered_line(&lines[start as usize]) {
        start += 1;
    }
    while end > start && is_blank_rendered_line(&lines[(end - 1) as usize]) {
        end -= 1;
    }
    lines[start as usize..end as usize].to_vec()
}

/// Dependencies for creating a tool HTML renderer.
pub struct ToolHtmlRendererDeps {
    /// Function to look up tool definition by name.
    /// Returns Some(lines) if the tool has a custom renderer, None to fall back to default.
    pub get_tool_call_renderer:
        Option<Box<dyn Fn(&str, &serde_json::Value) -> Option<Vec<String>> + Send + Sync>>,
    /// Function to render tool results. Returns (collapsed_lines, expanded_lines) or None.
    pub get_tool_result_renderer: Option<
        Box<
            dyn Fn(&str, &serde_json::Value, bool) -> Option<(Vec<String>, Vec<String>)>
                + Send
                + Sync,
        >,
    >,
    /// Terminal width for rendering (default: 100).
    pub width: Option<usize>,
}

/// Tool HTML renderer for HTML export.
pub struct ToolHtmlRenderer {
    deps: ToolHtmlRendererDeps,
}

impl ToolHtmlRenderer {
    pub fn new(deps: ToolHtmlRendererDeps) -> Self {
        ToolHtmlRenderer { deps }
    }

    /// Render a tool call to HTML. Returns None if tool has no custom renderer.
    pub fn render_call(
        &self,
        _tool_call_id: &str,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Option<String> {
        let renderer = self.deps.get_tool_call_renderer.as_ref()?;
        let lines = renderer(tool_name, args)?;
        Some(ansi_lines_to_html(&lines))
    }

    /// Render a tool result to collapsed/expanded HTML.
    /// Returns None if tool has no custom renderer.
    pub fn render_result(
        &self,
        _tool_call_id: &str,
        tool_name: &str,
        result: &serde_json::Value,
        is_error: bool,
    ) -> Option<ToolHtmlResultRendering> {
        let renderer = self.deps.get_tool_result_renderer.as_ref()?;
        let (collapsed_lines, expanded_lines) = renderer(tool_name, result, is_error)?;

        let collapsed_html = ansi_lines_to_html(&trim_rendered_result_lines(&collapsed_lines));
        let expanded_html = ansi_lines_to_html(&trim_rendered_result_lines(&expanded_lines));

        let collapsed = if collapsed_html != expanded_html {
            Some(collapsed_html)
        } else {
            None
        };

        Some(ToolHtmlResultRendering {
            collapsed,
            expanded: expanded_html,
        })
    }
}

/// Rendered HTML for a tool result.
pub struct ToolHtmlResultRendering {
    /// Collapsed view HTML. None if same as expanded.
    pub collapsed: Option<String>,
    /// Expanded view HTML.
    pub expanded: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_result_trims_tui_spacing_lines_from_custom_tool_result_html() {
        let renderer = ToolHtmlRenderer::new(ToolHtmlRendererDeps {
            get_tool_call_renderer: None,
            get_tool_result_renderer: Some(Box::new(|tool_name, _result, _is_error| {
                assert_eq!(tool_name, "custom");
                Some((
                    vec![
                        "".to_string(),
                        "\x1b[31mone\x1b[0m".to_string(),
                        "two".to_string(),
                        "".to_string(),
                    ],
                    vec![
                        "".to_string(),
                        "\x1b[31mone\x1b[0m".to_string(),
                        "two".to_string(),
                        "".to_string(),
                    ],
                ))
            })),
            width: Some(100),
        });

        let rendered = renderer
            .render_result("id", "custom", &json!([]), false)
            .expect("custom result renderer should run");

        assert_eq!(rendered.collapsed, None);
        assert_eq!(
            rendered.expanded,
            r#"<div class="ansi-line"><span style="color:#800000">one</span></div><div class="ansi-line">two</div>"#
        );
    }

    #[test]
    fn test_render_result_keeps_internal_blank_lines() {
        let renderer = ToolHtmlRenderer::new(ToolHtmlRendererDeps {
            get_tool_call_renderer: None,
            get_tool_result_renderer: Some(Box::new(|_, _, _| {
                Some((
                    vec!["one".to_string(), "".to_string(), "two".to_string()],
                    vec!["one".to_string(), "".to_string(), "two".to_string()],
                ))
            })),
            width: None,
        });

        let rendered = renderer
            .render_result("id", "custom", &json!([]), false)
            .expect("custom result renderer should run");

        assert_eq!(
            rendered.expanded,
            r#"<div class="ansi-line">one</div><div class="ansi-line">&nbsp;</div><div class="ansi-line">two</div>"#
        );
    }

    #[test]
    fn test_render_call_returns_none_without_custom_renderer() {
        let renderer = ToolHtmlRenderer::new(ToolHtmlRendererDeps {
            get_tool_call_renderer: None,
            get_tool_result_renderer: None,
            width: None,
        });

        assert!(renderer.render_call("id", "custom", &json!({})).is_none());
    }
}

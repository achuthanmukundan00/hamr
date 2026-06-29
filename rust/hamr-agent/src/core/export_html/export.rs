//! Main HTML export function — session entries to self-contained HTML.
//!
//! Mirror of `packages/coding-agent/src/core/export-html/index.ts` `exportFromFile`.

use base64::Engine;
use serde::Serialize;

use crate::core::session_manager::{SessionEntry, SessionHeader};

/// Embed the template assets at compile time.
const TEMPLATE_HTML: &str = include_str!("template.html");
const TEMPLATE_CSS: &str = include_str!("template.css");
const TEMPLATE_JS: &str = include_str!("template.js");
const MARKED_JS: &str = include_str!("vendor/marked.min.js");
const HIGHLIGHT_JS: &str = include_str!("vendor/highlight.min.js");

/// Data passed to the HTML template as base64-encoded JSON.
#[derive(Debug, Clone, Serialize)]
struct SessionData {
    header: Option<SessionHeader>,
    entries: Vec<SessionEntry>,
    #[serde(rename = "leafId")]
    leaf_id: Option<String>,
    #[serde(rename = "systemPrompt", skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(rename = "renderedTools", skip_serializing_if = "Option::is_none")]
    rendered_tools: Option<serde_json::Value>,
}

/// Default theme variables (dark theme) — mirrors the TS `generateThemeVars` default.
fn default_theme_vars() -> String {
    // These mirror the default dark theme colors used in the TS codebase.
    let vars = [
        ("--text", "rgb(220, 220, 220)"),
        ("--dim", "rgb(100, 100, 100)"),
        ("--muted", "rgb(80, 80, 80)"),
        ("--accent", "rgb(100, 140, 220)"),
        ("--success", "rgb(100, 200, 100)"),
        ("--error", "rgb(220, 80, 80)"),
        ("--warning", "rgb(200, 180, 80)"),
        ("--border", "rgb(60, 60, 70)"),
        ("--borderAccent", "rgb(100, 140, 220)"),
        ("--selectedBg", "rgba(100, 140, 220, 0.15)"),
        ("--hover", "rgba(255, 255, 255, 0.05)"),
        ("--userMessageBg", "rgb(52, 53, 65)"),
        ("--userMessageText", "rgb(220, 220, 220)"),
        ("--toolPendingBg", "rgba(100, 100, 100, 0.1)"),
        ("--toolSuccessBg", "rgba(100, 200, 100, 0.05)"),
        ("--toolErrorBg", "rgba(220, 80, 80, 0.05)"),
        ("--toolOutput", "rgb(180, 180, 180)"),
        ("--toolDiffAdded", "rgb(100, 200, 100)"),
        ("--toolDiffRemoved", "rgb(220, 80, 80)"),
        ("--toolDiffContext", "rgb(150, 150, 150)"),
        ("--thinkingText", "rgb(150, 150, 170)"),
        ("--customMessageBg", "rgba(100, 140, 220, 0.08)"),
        ("--customMessageLabel", "rgb(120, 160, 240)"),
        ("--customMessageText", "rgb(200, 210, 230)"),
        ("--mdHeading", "rgb(200, 200, 220)"),
        ("--mdLink", "rgb(100, 140, 220)"),
        ("--mdCode", "rgb(200, 180, 100)"),
        ("--mdQuote", "rgb(150, 150, 150)"),
        ("--mdQuoteBorder", "rgb(80, 80, 100)"),
        ("--mdListBullet", "rgb(120, 140, 220)"),
        ("--mdHr", "rgb(60, 60, 70)"),
        ("--mdCodeBlockBorder", "rgb(60, 60, 70)"),
        ("--syntaxComment", "rgb(100, 130, 100)"),
        ("--syntaxKeyword", "rgb(180, 140, 220)"),
        ("--syntaxNumber", "rgb(200, 180, 100)"),
        ("--syntaxString", "rgb(100, 180, 100)"),
        ("--syntaxFunction", "rgb(140, 180, 240)"),
        ("--syntaxType", "rgb(220, 180, 120)"),
        ("--syntaxVariable", "rgb(200, 200, 220)"),
        ("--syntaxOperator", "rgb(180, 180, 200)"),
        ("--syntaxPunctuation", "rgb(180, 180, 200)"),
    ];

    let mut lines: Vec<String> = vars.iter().map(|(k, v)| format!("{k}: {v};")).collect();

    // Derive export colors from userMessageBg
    lines.push("--exportPageBg: rgb(24, 24, 30);".to_string());
    lines.push("--exportCardBg: rgb(30, 30, 36);".to_string());
    lines.push("--exportInfoBg: rgb(50, 55, 40);".to_string());

    lines.join("\n      ")
}

/// Generate a self-contained HTML file from session entries.
///
/// This mirrors the TS `generateHtml` function, embedding session data as
/// base64-encoded JSON and using client-side JS for rendering.
pub fn export_session_to_html(
    header: Option<&SessionHeader>,
    entries: &[SessionEntry],
    leaf_id: Option<&str>,
) -> String {
    let theme_vars = default_theme_vars();
    let body_bg = "rgb(24, 24, 30)";
    let container_bg = "rgb(30, 30, 36)";
    let info_bg = "rgb(50, 55, 40)";

    let session_data = SessionData {
        header: header.cloned(),
        entries: entries.to_vec(),
        leaf_id: leaf_id.map(|s| s.to_string()),
        system_prompt: None,
        tools: None,
        rendered_tools: None,
    };

    let json = serde_json::to_string(&session_data).unwrap_or_else(|_| "{}".to_string());
    let b64 = base64::engine::general_purpose::STANDARD.encode(json.as_bytes());

    // Inject theme vars into CSS
    let css = TEMPLATE_CSS
        .replace("{{THEME_VARS}}", &theme_vars)
        .replace("{{BODY_BG}}", body_bg)
        .replace("{{CONTAINER_BG}}", container_bg)
        .replace("{{INFO_BG}}", info_bg);

    TEMPLATE_HTML
        .replace("{{CSS}}", &css)
        .replace("{{JS}}", TEMPLATE_JS)
        .replace("{{SESSION_DATA}}", &b64)
        .replace("{{MARKED_JS}}", MARKED_JS)
        .replace("{{HIGHLIGHT_JS}}", HIGHLIGHT_JS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session_manager::{SessionEntry, SessionHeader, SessionMessageEntry};

    #[test]
    fn test_export_empty_session() {
        let header = SessionHeader {
            entry_type: "session".into(),
            version: Some(3),
            id: "test-id".into(),
            timestamp: "2025-01-01T00:00:00Z".into(),
            cwd: "/tmp".into(),
            parent_session: None,
        };
        let html = export_session_to_html(Some(&header), &[], Some("test-id"));
        assert!(html.contains("<!DOCTYPE html>"));
        // Session data is base64-encoded; verify the <script> container exists
        assert!(html.contains(r#"<script id="session-data""#));
        // Verify the rendered CSS contains expected theme vars
        assert!(html.contains("--text:"));
    }

    #[test]
    fn test_export_with_entries() {
        let header = SessionHeader {
            entry_type: "session".into(),
            version: Some(3),
            id: "sess-1".into(),
            timestamp: "2025-01-01T00:00:00Z".into(),
            cwd: "/tmp".into(),
            parent_session: None,
        };

        let entries = vec![
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "1".into(),
                parent_id: None,
                timestamp: "2025-01-01T00:00:01Z".into(),
                message: serde_json::json!({"role": "user", "content": "hello"}),
            }),
            SessionEntry::Message(SessionMessageEntry {
                entry_type: "message".into(),
                id: "2".into(),
                parent_id: Some("1".into()),
                timestamp: "2025-01-01T00:00:02Z".into(),
                message: serde_json::json!({"role": "assistant", "content": "hi there"}),
            }),
        ];

        let html = export_session_to_html(Some(&header), &entries, Some("2"));
        assert!(html.contains("<!DOCTYPE html>"));
        // Session data is base64-encoded; verify the template rendered properly
        assert!(html.contains(r#"<script id="session-data""#));
        // The JS template and CSS should be inlined
        assert!(html.contains("function buildTree"));
        assert!(html.contains("--body-bg:"));
    }

    #[test]
    fn test_export_no_header() {
        let html = export_session_to_html(None, &[], None);
        assert!(html.contains("<!DOCTYPE html>"));
    }
}

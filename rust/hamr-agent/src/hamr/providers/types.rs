//! Port of `packages/coding-agent/src/hamr/providers/types.ts`.
//!
//! Shared types for the providers module — `ParseWarning` and `ParsedModelOutput`.

use serde::{Deserialize, Serialize};

use super::parsers::types::ParsedToolCall;

/// A non-fatal parse warning produced during model output parsing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParseWarning {
    pub message: String,
    pub source: WarningSource,
}

/// Where a warning originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WarningSource {
    Parser,
    Reasoning,
    Repair,
}

/// The fully parsed output from a model response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedModelOutput {
    /// Assistant-visible text (content without tool-call blocks).
    pub assistant_text: String,
    /// Parsed tool calls extracted from the content or reasoning.
    pub tool_calls: Vec<ParsedToolCall>,
    /// Extracted reasoning/thinking content, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Non-fatal warnings from parsing and repair.
    #[serde(default)]
    pub warnings: Vec<ParseWarning>,
    /// Whether the parser succeeded (even if no calls were found).
    pub parser_ok: bool,
}

impl ParsedModelOutput {
    /// Create a successful output with no tool calls.
    pub fn empty(assistant_text: String) -> Self {
        Self {
            assistant_text,
            tool_calls: Vec::new(),
            reasoning: None,
            warnings: Vec::new(),
            parser_ok: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_warning_serde() {
        let warning = ParseWarning {
            message: "test".into(),
            source: WarningSource::Parser,
        };
        let json = serde_json::to_string(&warning).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("parser"));

        let parsed: ParseWarning = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.message, "test");
        assert_eq!(parsed.source, WarningSource::Parser);
    }

    #[test]
    fn test_parsed_model_output_empty() {
        let out = ParsedModelOutput::empty("hello".into());
        assert_eq!(out.assistant_text, "hello");
        assert!(out.tool_calls.is_empty());
        assert!(out.parser_ok);
    }

    #[test]
    fn test_parsed_model_output_serde() {
        let out = ParsedModelOutput {
            assistant_text: "text".into(),
            tool_calls: vec![],
            reasoning: Some("thinking".into()),
            warnings: vec![ParseWarning {
                message: "warn".into(),
                source: WarningSource::Repair,
            }],
            parser_ok: true,
        };
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains("assistantText"));
        assert!(json.contains("reasoning"));
        assert!(json.contains("warn"));

        let parsed: ParsedModelOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.reasoning, Some("thinking".into()));
        assert!(!parsed.warnings.is_empty());
    }
}

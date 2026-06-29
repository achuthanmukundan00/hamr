//! Port of `packages/coding-agent/src/hamr/providers/tool-calls.ts`.
//!
//! Core tool-call parsing pipeline:
//! 1. Extract reasoning content (from API field or embedded in content)
//! 2. Parse tool calls from content using the configured parser
//! 3. Repair cascade: try JSON repair → XML repair → fallback parsers
//! 4. Fallback to reasoning_content if no tool calls found in content
//! 5. Extract assistant-visible text (without tool-call markup)

use std::collections::HashSet;

use crate::hamr::providers::parsers::{
    ensure_parsers_registered, parse_with_tool_call_parser, sanitize_reasoning_tags,
};
use crate::hamr::providers::repair::json_repair::repair_json;
use crate::hamr::providers::repair::reasoning_sanitizer::SanitizeResult;
use crate::hamr::providers::repair::xml_repair::repair_xml;
use crate::hamr::providers::types::{ParseWarning, ParsedModelOutput, WarningSource};

/// Ensure the parser registry is initialized (idempotent).
fn ensure_parsers_registered_once() {
    ensure_parsers_registered();
}

/// Convenience re-export: strip `<think>`/`<thinking>` tags from inline content
/// without the full repair-aware extraction.
pub fn sanitize_reasoning(content: &str) -> String {
    sanitize_reasoning_tags(content)
}

/// Run the full reasoning sanitizer (provider-aware extraction).
/// Mirrors `repairSanitize()` in the TS source.
pub fn repair_sanitize_reasoning(content: &str) -> SanitizeResult {
    crate::hamr::providers::repair::reasoning_sanitizer::sanitize_reasoning(content)
}

// ─── Main parse function ─────────────────────────────────────────────────────

/// Parse raw model output into a typed `ParsedModelOutput`.
///
/// This is the core pipeline. Mirror of `parseModelOutput` in the TS source.
///
/// Parameters:
/// - `content`: the raw text output from the model
/// - `parser_id`: which parser to use (e.g. "qwen3_xml", "hermes", "generic")
/// - `reasoning_content`: optional reasoning from a separate API field
pub fn parse_model_output(
    content: &str,
    parser_id: &str,
    reasoning_content: Option<&str>,
) -> ParsedModelOutput {
    ensure_parsers_registered_once();

    let mut warnings: Vec<ParseWarning> = Vec::new();
    let reasoning = reasoning_content
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
        .map(|r| r.to_string());
    let mut cleaned_content = content.to_string();

    // Step 1: Extract reasoning from content if the provider embeds it inline
    // (Qwen models emit <think> blocks, some models leak thinking in various forms).
    // Use the dedicated reasoning sanitizer for provider-aware extraction.
    if reasoning.is_none() {
        let sanitize_result = repair_sanitize_reasoning(&cleaned_content);
        if sanitize_result.removed_reasoning {
            warnings.push(ParseWarning {
                message: "Extracted reasoning tags from model output".to_string(),
                source: WarningSource::Reasoning,
            });
        }
        cleaned_content = sanitize_result.content;
    } else {
        // If reasoning was provided via API field, strip inline tags from
        // content so they don't interfere with parsing.
        let stripped = sanitize_reasoning(&cleaned_content);
        if stripped != cleaned_content {
            warnings.push(ParseWarning {
                message: "Stripped inline reasoning tags from content".to_string(),
                source: WarningSource::Reasoning,
            });
            cleaned_content = stripped;
        }
    }

    // Step 2: Parse tool calls from cleaned content with repair fallback
    let parser_result = parse_with_tool_call_parser(parser_id, &cleaned_content);
    let mut parser_ok = parser_result.ok;
    let mut tool_calls = parser_result.calls;

    if !parser_ok || tool_calls.is_empty() {
        // Repair cascade: try JSON repair first, then XML repair.
        // Track attempted (parserId, content) pairs to avoid redundant re-parsing
        // when repair functions produce identical or unchanged output (#16).
        let mut attempted_repairs = HashSet::new();
        attempted_repairs.insert(format!("{}::{}", parser_id, cleaned_content));

        let mut try_parse_repaired =
            |pid: &str,
             text: &str|
             -> Option<crate::hamr::providers::parsers::types::ToolCallParseResult> {
                let key = format!("{}::{}", pid, text);
                if attempted_repairs.contains(&key) {
                    return None;
                }
                attempted_repairs.insert(key);
                Some(parse_with_tool_call_parser(pid, text))
            };

        // Try JSON repair
        if let Some(json_repaired) = repair_json(&cleaned_content) {
            if let Some(result) = try_parse_repaired(parser_id, &json_repaired.repaired) {
                if result.ok && !result.calls.is_empty() {
                    parser_ok = true;
                    warnings.push(ParseWarning {
                        message: "Recovered via JSON repair".to_string(),
                        source: WarningSource::Parser,
                    });
                    tool_calls.extend(result.calls);
                }
            }
        }

        // Try XML repair
        if tool_calls.is_empty() {
            if let Some(xml_repaired) = repair_xml(&cleaned_content) {
                if let Some(result) = try_parse_repaired(parser_id, &xml_repaired.repaired) {
                    if result.ok && !result.calls.is_empty() {
                        parser_ok = true;
                        warnings.push(ParseWarning {
                            message: "Recovered via XML repair".to_string(),
                            source: WarningSource::Parser,
                        });
                        tool_calls.extend(result.calls);
                    } else if parser_id != "qwen3_xml" {
                        // Fallback: try qwen3_xml parser on XML-repaired content
                        if let Some(qwen_result) =
                            try_parse_repaired("qwen3_xml", &xml_repaired.repaired)
                        {
                            if qwen_result.ok && !qwen_result.calls.is_empty() {
                                parser_ok = true;
                                warnings.push(ParseWarning {
                                    message: "Recovered via XML repair + qwen3_xml parser"
                                        .to_string(),
                                    source: WarningSource::Parser,
                                });
                                tool_calls.extend(qwen_result.calls);
                            }
                        }
                    }
                }
            }
        }

        if tool_calls.is_empty() && !parser_ok {
            warnings.push(ParseWarning {
                message: parser_result
                    .error
                    .unwrap_or_else(|| "parser error".to_string()),
                source: WarningSource::Parser,
            });
        }
    }

    // Step 3: If no tool calls found in content, try parsing reasoning_content.
    // Some providers route tool-call XML to reasoning_content instead of content.
    if tool_calls.is_empty() {
        if let Some(ref rc) = reasoning {
            let mut rc_result = parse_with_tool_call_parser(parser_id, rc);
            if (!rc_result.ok || rc_result.calls.is_empty()) && !rc.is_empty() {
                let cleaned_rc = sanitize_reasoning(rc);
                if !cleaned_rc.is_empty() {
                    rc_result = parse_with_tool_call_parser(parser_id, &cleaned_rc);
                }
            }
            if (!rc_result.ok || rc_result.calls.is_empty()) && parser_id != "qwen3_xml" {
                let xml_result = parse_with_tool_call_parser("qwen3_xml", rc);
                if xml_result.ok && !xml_result.calls.is_empty() {
                    rc_result = xml_result;
                } else {
                    let cleaned_rc = sanitize_reasoning(rc);
                    if !cleaned_rc.is_empty() {
                        let xml_result2 = parse_with_tool_call_parser("qwen3_xml", &cleaned_rc);
                        if xml_result2.ok && !xml_result2.calls.is_empty() {
                            rc_result = xml_result2;
                        }
                    }
                }
            }
            if rc_result.ok && !rc_result.calls.is_empty() {
                warnings.push(ParseWarning {
                    message: "Extracted tool calls from reasoning_content".to_string(),
                    source: WarningSource::Parser,
                });
                tool_calls.extend(rc_result.calls);
            }
        }
    }

    // Step 4: Extract assistant-visible text (content without tool-call blocks)
    let mut assistant_text = parser_result.content;
    if assistant_text.is_empty() {
        assistant_text = cleaned_content;
    }

    // Bug #114: When DeepSeek returns empty content but rich reasoning_content,
    // fall back to reasoning as the assistant-visible answer. Strip thinking/tool-call
    // tags from reasoning to produce clean prose.
    if assistant_text.is_empty() {
        if let Some(ref rc) = reasoning {
            if !rc.is_empty() {
                let sanitized = sanitize_reasoning(rc);
                // Strip tool-call markup that may have leaked into reasoning_content
                let tc_re = regex::Regex::new(r"(?i)<tool_call>[\s\S]*?</tool_call>").unwrap();
                let pipe_re = regex::Regex::new(r"(?i)<\|tool_call\|>[\s\S]*").unwrap();
                let visible = pipe_re
                    .replace(&tc_re.replace_all(&sanitized, " "), "")
                    .trim()
                    .to_string();
                if !visible.is_empty() {
                    assistant_text = visible;
                    warnings.push(ParseWarning {
                        message: "Used reasoningContent as fallback for empty content (bug #114)"
                            .to_string(),
                        source: WarningSource::Reasoning,
                    });
                }
            }
        }
    }

    ParsedModelOutput {
        assistant_text,
        tool_calls,
        reasoning,
        warnings,
        parser_ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_output_empty() {
        let result = parse_model_output("hello world", "generic", None);
        // Generic parser should succeed on plain text (no tool calls expected)
        assert!(result.parser_ok);
        assert!(!result.assistant_text.is_empty());
        assert!(result.tool_calls.is_empty());
        assert!(result.reasoning.is_none());
    }

    #[test]
    fn test_parse_model_output_with_reasoning() {
        let result = parse_model_output(
            "some text <think>thinking</think>",
            "generic",
            Some("  deep thinking...  "),
        );
        assert_eq!(result.reasoning, Some("deep thinking...".to_string()));
    }

    #[test]
    fn test_parse_model_output_empty_content_with_reasoning() {
        // Bug #114 fallback — when content is empty but reasoning has prose,
        // the assistant_text should fall back to the sanitized reasoning.
        let result = parse_model_output(
            "",
            "generic",
            Some("This is the actual response from reasoning"),
        );
        // With empty content and no tool calls, assistant_text should fall back
        // to the reasoning content (sanitized).
        assert!(!result.assistant_text.is_empty());
    }

    #[test]
    fn test_parse_model_output_reasoning_extraction() {
        // Without explicit reasoning, the repair sanitizer extracts <think> blocks
        let result = parse_model_output(
            "<think>Let me think about this</think>\nThe answer is 42",
            "generic",
            None,
        );
        // The reasoning should be None (extracted from content, not provided separately),
        // and the content should be cleaned.
        assert!(result.reasoning.is_none());
        assert!(result.assistant_text.contains("The answer is 42"));
    }

    #[test]
    fn test_sanitize_reasoning_passthrough() {
        let result = sanitize_reasoning("hello world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_sanitize_reasoning_strips_tags() {
        let result = sanitize_reasoning("hello <think>thinking</think> world");
        assert!(!result.contains("<think>"));
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
    }
}

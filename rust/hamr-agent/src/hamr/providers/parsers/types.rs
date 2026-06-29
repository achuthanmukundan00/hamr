//! Canonical types for Hamr tool-call parsers.
//!
//! Parsers convert model text output into normalized [ParsedToolCall] records.
//! Inspired by vLLM's tool-call-parser architecture but implemented natively
//! so Hamr does not depend on vLLM runtime normalization.

use serde::{Deserialize, Serialize};
use std::fmt;

fn re_test(pattern: &str, text: &str) -> bool {
    if let Ok(re) = regex::Regex::new(pattern) {
        re.is_match(text)
    } else {
        false
    }
}

/// Stable call id, either from model output or deterministically generated.
pub type CallId = String;
/// Function/tool name.
pub type ToolName = String;
/// Parsed arguments object.
pub type Arguments = serde_json::Value;
/// Raw source text span (the exact substring that was parsed).
pub type RawSource = String;
/// Parser id that produced this call.
pub type ParserId = String;

// ─── Canonical tool call ──────────────────────────────────

/// A single parsed tool call, normalised across all parsers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedToolCall {
    /// Stable call id.
    pub id: CallId,
    /// Function / tool name.
    pub name: ToolName,
    /// Parsed arguments object.
    pub arguments: Arguments,
    /// Raw source text span (the exact substring that was parsed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_source: Option<RawSource>,
    /// Parser id that produced this call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser_id: Option<ParserId>,
    /// Recoverable parse warnings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

impl ParsedToolCall {
    pub fn new(id: CallId, name: ToolName, arguments: Arguments) -> Self {
        Self {
            id,
            name,
            arguments,
            raw_source: None,
            parser_id: None,
            warnings: None,
        }
    }
}

// ─── Parser result ────────────────────────────────────────

/// The result of parsing a model response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallParseResult {
    /// Whether parsing itself succeeded (even if no calls were found).
    pub ok: bool,
    /// Parser id used.
    pub parser_id: ParserId,
    /// Parsed calls. Empty when no calls detected — that is not an error.
    pub calls: Vec<ParsedToolCall>,
    /// Non-call content that should remain in the assistant message.
    pub content: String,
    /// Errors when ok=false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Recoverable warnings (call-level warnings are on each [ParsedToolCall]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

impl ToolCallParseResult {
    pub fn ok(
        parser_id: impl Into<String>,
        calls: Vec<ParsedToolCall>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            ok: true,
            parser_id: parser_id.into(),
            calls,
            content: content.into(),
            error: None,
            warnings: None,
        }
    }

    pub fn err(
        parser_id: impl Into<String>,
        content: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            ok: false,
            parser_id: parser_id.into(),
            calls: Vec::new(),
            content: content.into(),
            error: Some(error.into()),
            warnings: None,
        }
    }
}

impl fmt::Display for ToolCallParseResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ParseResult(ok={}, calls={}, parser={})",
            self.ok,
            self.calls.len(),
            self.parser_id
        )?;
        if let Some(ref error) = self.error {
            write!(f, ", error={}", error)?;
        }
        Ok(())
    }
}

// ─── Parser interface ─────────────────────────────────────

/// A tool-call parser converts raw model text into normalized tool call records.
///
/// Parsers are stateless single-invocation functions. Streaming buffering
/// is handled by the caller (the provider client), which feeds complete
/// model output to the parser after the stream ends.
pub trait ToolCallParser {
    /// Unique parser id, matching vLLM's --tool-call-parser names where practical.
    fn id(&self) -> &str;

    /// Human-readable description for config docs.
    fn description(&self) -> &str;

    /// Model families this parser is designed for (for docs and auto-detection).
    fn model_families(&self) -> &[&str];

    /// Parse a complete model response text into canonical calls.
    fn parse(&self, content: &str) -> ToolCallParseResult;
}

// ─── Parser factory ───────────────────────────────────────

/// Factory function that creates a parser instance.
/// Matching vLLM's approach, parsers receive a tokenizer only when needed
/// (primarily for parsers that need to decode token IDs). For Hamr's
/// text-based parsing, most parsers use the default no-op factory.
pub type ToolCallParserFactory = fn() -> Box<dyn ToolCallParser>;

// ─── Registry ─────────────────────────────────────────────

/// Maps parser ids to parser implementations.
pub trait ToolCallParserRegistry {
    /// Register a parser factory under a given id.
    fn register(&mut self, id: &str, factory: ToolCallParserFactory);

    /// Get a parser by id. Returns None if not registered.
    fn get(&self, id: &str) -> Option<Box<dyn ToolCallParser>>;

    /// List all registered parser ids.
    fn list_ids(&self) -> Vec<String>;

    /// List all registered parsers with their descriptions.
    fn list_parsers(&self) -> Vec<ParserInfo>;

    /// Parse content using the parser registered under `id`.
    fn parse(&self, id: &str, content: &str) -> ToolCallParseResult;
}

/// Minimal info about a registered parser (without requiring instantiation).
#[derive(Debug, Clone, Serialize)]
pub struct ParserInfo {
    pub id: String,
    pub description: String,
    pub model_families: Vec<String>,
}

// ─── Auto-detection ───────────────────────────────────────

/// Conservative auto-detection. Inspects the model id string for substrings
/// matching known model families. Returns None when uncertain.
///
/// Config override always takes priority over auto-detection.
pub fn detect_parser_id(model_id: &str) -> Option<&'static str> {
    let lower = model_id.to_lowercase();

    // Order matters: more specific patterns first.
    if re_test(r"\bqwen3[-.]?coder\b", &lower) {
        return Some("qwen3_xml");
    }
    if re_test(r"\bqwen3\.6\b", &lower) {
        return Some("qwen3_xml");
    }
    if re_test(r"\bqwen3\.5\b", &lower) {
        return Some("qwen3_xml");
    }
    if re_test(r"\bqwen3\b", &lower) {
        return Some("qwen3_xml");
    }
    if re_test(r"\bqwen2\.5\b", &lower) {
        return Some("hermes");
    } // Qwen2.5 uses Hermes-style
    // No broad /\bqwen\b/i catch-all — Qwen3.X / Qwen3-Coder / Qwen3 are matched above.

    // Hermes family
    if re_test(r"\bhermes\b", &lower) {
        return Some("hermes");
    }
    if re_test(r"\bnous\b", &lower) {
        return Some("hermes");
    }
    if re_test(r"openhermes", &lower) {
        return Some("hermes");
    }

    // Llama 3/4 JSON
    if re_test(r"\bllama-?4\b", &lower) {
        return Some("llama4_pythonic");
    }
    if re_test(r"\bllama-?3\.?[23]\b", &lower) {
        return Some("llama3_json");
    }
    if re_test(r"\bllama-?3\b", &lower) {
        return Some("llama3_json");
    }
    if re_test(r"meta-llama", &lower) {
        return Some("llama3_json");
    }

    // DeepSeek
    if re_test(r"deepseek-?v3\.1", &lower) {
        return Some("deepseek_v31");
    }
    if re_test(r"deepseek-?v3", &lower) {
        return Some("deepseek_v3");
    }
    if re_test(r"deepseek-?r1", &lower) {
        return Some("deepseek_v3");
    }
    if re_test(r"deepseek", &lower) {
        return Some("deepseek_v3");
    }

    // Mistral
    if re_test(r"\bmistral\b", &lower) {
        return Some("mistral");
    }
    if re_test(r"mixtral", &lower) {
        return Some("mistral");
    }

    // xLAM
    if re_test(r"\bxlam\b", &lower) {
        return Some("xlam");
    }

    // Granite
    if re_test(r"\bgranite-?4\b", &lower) {
        return Some("granite4");
    }
    if re_test(r"\bgranite-?20b-fc\b", &lower) {
        return Some("granite-20b-fc");
    }
    if re_test(r"\bgranite\b", &lower) {
        return Some("granite");
    }

    // InternLM
    if re_test(r"\binternlm\b", &lower) {
        return Some("internlm");
    }

    // FunctionGemma / Gemma
    if re_test(r"\bfunctiongemma\b", &lower) {
        return Some("functiongemma");
    }
    if re_test(r"\bgemma-?2.*function\b", &lower) {
        return Some("functiongemma");
    }
    // Gemma 3/4 (including QAT/quantized variants). These models use the
    // OpenAI native tool_calls convention (role: 'tool', tool_call_id)
    // rather than XML-wrapped </s> user messages.
    if re_test(r"\bgemma-?[34]", &lower) {
        return Some("gemma_native");
    }
    if re_test(r"\bgemma\b", &lower) {
        return Some("gemma_native");
    }

    // OLMo3
    if re_test(r"\bolmo[e3]", &lower) {
        return Some("olmo3");
    }

    // GLM family
    if re_test(r"\bglm-?4\.7\b", &lower) {
        return Some("glm47");
    }
    if re_test(r"\bglm-?4\.5\b", &lower) {
        return Some("glm45");
    }
    if re_test(r"\bglm-?4\b", &lower) {
        return Some("glm45");
    }
    if re_test(r"\bglm\b", &lower) {
        return Some("glm45");
    }

    // Step family
    if re_test(r"\bstep-?3\.5\b", &lower) {
        return Some("step3p5");
    }
    if re_test(r"\bstep-?3\b", &lower) {
        return Some("step3");
    }
    // No broad /\bstep\b/i catch-all — "step" is a common word in unrelated model names.

    // Kimi
    if re_test(r"\bkimi[-_]?k2\b", &lower) {
        return Some("kimi_k2");
    }
    if re_test(r"\bkimi\b", &lower) {
        return Some("kimi_k2");
    }

    // Hunyuan
    if re_test(r"\bhunyuan[-_]?a13b\b", &lower) {
        return Some("hunyuan_a13b");
    }
    if re_test(r"\bhunyuan\b", &lower) {
        return Some("hunyuan_a13b");
    }

    // LongCat
    if re_test(r"\blongcat\b", &lower) {
        return Some("longcat");
    }

    // Jamba
    if re_test(r"\bjamba\b", &lower) {
        return Some("jamba");
    }

    // MiniMax
    if re_test(r"\bminimax\b", &lower) {
        return Some("minimax");
    }

    // GigaChat
    if re_test(r"\bgigachat\b", &lower) {
        return Some("gigachat3");
    }

    // Pythonic (Llama 4, etc.)
    if re_test(r"\bllama-?4\b", &lower) {
        return Some("llama4_pythonic");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_qwen3() {
        assert_eq!(detect_parser_id("Qwen3-Coder"), Some("qwen3_xml"));
        assert_eq!(detect_parser_id("qwen3.6"), Some("qwen3_xml"));
        assert_eq!(detect_parser_id("qwen3.5"), Some("qwen3_xml"));
        assert_eq!(detect_parser_id("qwen3"), Some("qwen3_xml"));
    }

    #[test]
    fn test_detect_hermes() {
        assert_eq!(detect_parser_id("Hermes-2-Pro"), Some("hermes"));
        assert_eq!(detect_parser_id("nous-hermes"), Some("hermes"));
        assert_eq!(detect_parser_id("OpenHermes"), Some("hermes"));
    }

    #[test]
    fn test_detect_llama() {
        assert_eq!(detect_parser_id("llama-4"), Some("llama4_pythonic"));
        assert_eq!(detect_parser_id("llama-3.2"), Some("llama3_json"));
        assert_eq!(detect_parser_id("llama-3"), Some("llama3_json"));
        assert_eq!(detect_parser_id("meta-llama-70b"), Some("llama3_json"));
    }

    #[test]
    fn test_detect_deepseek() {
        assert_eq!(detect_parser_id("deepseek-v3.1"), Some("deepseek_v31"));
        assert_eq!(detect_parser_id("deepseek-v3"), Some("deepseek_v3"));
        assert_eq!(detect_parser_id("deepseek-r1"), Some("deepseek_v3"));
        assert_eq!(detect_parser_id("deepseek"), Some("deepseek_v3"));
    }

    #[test]
    fn test_detect_mistral() {
        assert_eq!(detect_parser_id("mistral-small-latest"), Some("mistral"));
        assert_eq!(detect_parser_id("Mixtral-8x7B"), Some("mistral"));
    }

    #[test]
    fn test_detect_glm() {
        assert_eq!(detect_parser_id("glm-4.7"), Some("glm47"));
        assert_eq!(detect_parser_id("glm-4.5"), Some("glm45"));
        assert_eq!(detect_parser_id("glm-4"), Some("glm45"));
        assert_eq!(detect_parser_id("glm"), Some("glm45"));
    }

    #[test]
    fn test_detect_undefined() {
        assert_eq!(detect_parser_id("unknown-model"), None);
        assert_eq!(detect_parser_id(""), None);
    }
}

//! Tool-call parsers for non-native function-calling models.
//!
//! Mirror of `packages/coding-agent/src/hamr/providers/parsers/`.
//!
//! ## Usage
//!
//! Call [`ensure_parsers_registered`] once at startup to register all parsers
//! in the global registry, then use [`parse_with_tool_call_parser`] to parse
//! model output.
//!
//! ```rust,ignore
//! use hamr_agent::hamr::providers::parsers::{ensure_parsers_registered, parse_with_tool_call_parser};
//!
//! ensure_parsers_registered();
//! let result = parse_with_tool_call_parser("hermes", "<tool_call>{...}</tool_call>");
//! ```

pub mod deepseek;
pub mod generic;
pub mod glm_step;
pub mod hermes;
pub mod json_in_tags;
pub mod llama3_json;
pub mod mistral;
pub mod pythonic;
pub mod qwen3_xml;
pub mod registry;
pub mod types;
pub mod utils;
pub mod xlam;

use std::sync::atomic::{AtomicBool, Ordering};

// ─── Re-exports ───────────────────────────────────────────

pub use registry::{
    DefaultToolCallParserRegistry, get_tool_call_parser, list_tool_call_parser_ids,
    parse_with_tool_call_parser, register_tool_call_parser,
};
pub use types::{
    ParsedToolCall, ParserInfo, ToolCallParseResult, ToolCallParser, ToolCallParserFactory,
    ToolCallParserRegistry, detect_parser_id,
};
pub use utils::{
    CallOpts, DelimitedResult, ValueKind, coerce_value, extract_delimited_blocks,
    extract_non_tool_content, fast_json_parse, generate_call_id, make_call, parse_pythonic_args,
    reset_call_id_counter, safe_json_parse, sanitize_reasoning_tags,
};

// ─── One-shot registration ────────────────────────────────

static REGISTERED: AtomicBool = AtomicBool::new(false);

/// Register every built-in tool-call parser into the global registry.
///
/// Idempotent — subsequent calls are no-ops. Call once at startup
/// before any parsing is attempted.
pub fn ensure_parsers_registered() {
    if REGISTERED.swap(true, Ordering::AcqRel) {
        return; // already registered
    }

    // XML/tag-based parsers (highest priority for local models)
    register_tool_call_parser("qwen3_xml", qwen3_xml::create_qwen3_xml_parser);
    register_tool_call_parser("qwen3_coder", qwen3_xml::create_qwen3_xml_parser); // alias
    register_tool_call_parser("hermes", hermes::create_hermes_parser);
    register_tool_call_parser("step3", glm_step::create_step3_parser);
    register_tool_call_parser("step3p5", glm_step::create_step3p5_parser);
    register_tool_call_parser("functiongemma", json_in_tags::create_function_gemma_parser);
    register_tool_call_parser("gemma_native", generic::create_generic_parser);
    register_tool_call_parser("olmo3", json_in_tags::create_olmo3_parser);
    register_tool_call_parser("glm45", glm_step::create_glm45_parser);
    register_tool_call_parser("glm47", glm_step::create_glm47_parser);
    register_tool_call_parser("gigachat3", json_in_tags::create_gigachat3_parser);

    // JSON-based parsers
    register_tool_call_parser("llama3_json", llama3_json::create_llama3_json_parser);
    register_tool_call_parser("mistral", mistral::create_mistral_parser);
    register_tool_call_parser("xlam", xlam::create_xlam_parser);
    register_tool_call_parser("granite", json_in_tags::create_granite_parser);
    register_tool_call_parser("granite4", json_in_tags::create_granite4_parser);
    register_tool_call_parser("granite-20b-fc", json_in_tags::create_granite20b_fc_parser);
    register_tool_call_parser("internlm", json_in_tags::create_internlm_parser);
    register_tool_call_parser("jamba", json_in_tags::create_jamba_parser);
    register_tool_call_parser("minimax", json_in_tags::create_minimax_parser);
    register_tool_call_parser("kimi_k2", json_in_tags::create_kimi_k2_parser);
    register_tool_call_parser("hunyuan_a13b", json_in_tags::create_hunyuan_a13b_parser);
    register_tool_call_parser("longcat", json_in_tags::create_longcat_parser);
    register_tool_call_parser("openai", json_in_tags::create_openai_passthrough_parser);

    // Pythonic parsers
    register_tool_call_parser("pythonic", pythonic::create_pythonic_parser);
    register_tool_call_parser("llama4_pythonic", pythonic::create_llama4_pythonic_parser);

    // DeepSeek parsers
    register_tool_call_parser("deepseek_v3", deepseek::create_deepseek_v3_parser);
    register_tool_call_parser("deepseek_v31", deepseek::create_deepseek_v31_parser);

    // Generic fallback
    register_tool_call_parser("generic", generic::create_generic_parser);
}

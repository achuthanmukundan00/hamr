/**
 * Canonical types for Hamr tool-call parsers.
 *
 * Parsers convert model text output into normalized ParsedToolCall records.
 * Inspired by vLLM's tool-call-parser architecture but implemented natively
 * so Hamr does not depend on vLLM runtime normalization.
 */
// ─── Auto-detection ───────────────────────────────────────
/**
 * Conservative auto-detection. Inspects the model id string for substrings
 * matching known model families. Returns undefined when uncertain.
 *
 * Config override always takes priority over auto-detection.
 */
export function detectParserId(modelId) {
    const lower = modelId.toLowerCase();
    // Order matters: more specific patterns first.
    const patterns = [
        // XML/tag-based families
        { pattern: /\bqwen3[-.]?coder\b/i, parser: "qwen3_xml" },
        { pattern: /\bqwen3\.6\b/i, parser: "qwen3_xml" },
        { pattern: /\bqwen3\.5\b/i, parser: "qwen3_xml" },
        { pattern: /\bqwen3\b/i, parser: "qwen3_xml" },
        { pattern: /\bqwen2\.5\b/i, parser: "hermes" }, // Qwen2.5 uses Hermes-style
        // No broad /\bqwen\b/i catch-all — Qwen3.X / Qwen3-Coder / Qwen3 are matched above.
        // Hermes family
        { pattern: /\bhermes\b/i, parser: "hermes" },
        { pattern: /\bnous\b/i, parser: "hermes" },
        { pattern: /openhermes/i, parser: "hermes" },
        // Llama 3/4 JSON
        { pattern: /\bllama-?4\b/i, parser: "llama4_pythonic" },
        { pattern: /\bllama-?3\.?[23]\b/i, parser: "llama3_json" },
        { pattern: /\bllama-?3\b/i, parser: "llama3_json" },
        { pattern: /meta-llama/i, parser: "llama3_json" },
        // DeepSeek
        { pattern: /deepseek-?v3\.1/i, parser: "deepseek_v31" },
        { pattern: /deepseek-?v3/i, parser: "deepseek_v3" },
        { pattern: /deepseek-?r1/i, parser: "deepseek_v3" },
        { pattern: /deepseek/i, parser: "deepseek_v3" },
        // Mistral
        { pattern: /\bmistral\b/i, parser: "mistral" },
        { pattern: /mixtral/i, parser: "mistral" },
        // xLAM
        { pattern: /\bxlam\b/i, parser: "xlam" },
        // Granite
        { pattern: /\bgranite-?4\b/i, parser: "granite4" },
        { pattern: /\bgranite-?20b-fc\b/i, parser: "granite-20b-fc" },
        { pattern: /\bgranite\b/i, parser: "granite" },
        // InternLM
        { pattern: /\binternlm/i, parser: "internlm" },
        // FunctionGemma / Gemma
        { pattern: /\bfunctiongemma\b/i, parser: "functiongemma" },
        { pattern: /\bgemma-?2.*function/i, parser: "functiongemma" },
        // Gemma 3/4 (including QAT/quantized variants). These models use the
        // OpenAI native tool_calls convention (role: 'tool', tool_call_id)
        // rather than XML-wrapped <tool_response> user messages.
        { pattern: /\bgemma-?[34]/i, parser: "gemma_native" },
        { pattern: /\bgemma\b/i, parser: "gemma_native" },
        // OLMo3
        { pattern: /\bolmo[e3]/i, parser: "olmo3" },
        // GLM family
        { pattern: /\bglm-?4\.7\b/i, parser: "glm47" },
        { pattern: /\bglm-?4\.5\b/i, parser: "glm45" },
        { pattern: /\bglm-?4\b/i, parser: "glm45" },
        { pattern: /\bglm\b/i, parser: "glm45" },
        // Step family
        { pattern: /\bstep-?3\.5\b/i, parser: "step3p5" },
        { pattern: /\bstep-?3\b/i, parser: "step3" },
        // No broad /\bstep\b/i catch-all — "step" is a common word in unrelated model names.
        // Kimi
        { pattern: /\bkimi[-_]?k2\b/i, parser: "kimi_k2" },
        { pattern: /\bkimi\b/i, parser: "kimi_k2" },
        // Hunyuan
        { pattern: /\bhunyuan[-_]?a13b\b/i, parser: "hunyuan_a13b" },
        { pattern: /\bhunyuan\b/i, parser: "hunyuan_a13b" },
        // LongCat
        { pattern: /\blongcat\b/i, parser: "longcat" },
        // Jamba
        { pattern: /\bjamba\b/i, parser: "jamba" },
        // MiniMax
        { pattern: /\bminimax\b/i, parser: "minimax" },
        // GigaChat
        { pattern: /\bgigachat\b/i, parser: "gigachat3" },
        // Pythonic (Llama 4, etc.)
        { pattern: /\bllama-?4\b/i, parser: "llama4_pythonic" },
    ];
    for (const { pattern, parser } of patterns) {
        if (pattern.test(lower))
            return parser;
    }
    return undefined;
}
//# sourceMappingURL=types.js.map
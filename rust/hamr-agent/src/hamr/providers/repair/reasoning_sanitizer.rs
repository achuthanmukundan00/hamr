use regex::Regex;
use std::sync::OnceLock;

/// Result of the reasoning sanitization process.
pub struct SanitizeResult {
    /// The sanitized content.
    pub content: String,
    /// Flag indicating whether any reasoning/thinking text was removed.
    pub removed_reasoning: bool,
}

static THINK_RE: OnceLock<Regex> = OnceLock::new();
static THINKING_RE: OnceLock<Regex> = OnceLock::new();
static FENCED_RE: OnceLock<Regex> = OnceLock::new();
static LEAKED_RE: OnceLock<Regex> = OnceLock::new();
static TOOL_CALL_RE: OnceLock<Regex> = OnceLock::new();
static MARKERS_RE: OnceLock<Regex> = OnceLock::new();
static SELF_CLOSING_RE: OnceLock<Regex> = OnceLock::new();
static OPEN_THINK_RE: OnceLock<Regex> = OnceLock::new();
static CLOSE_THINK_RE: OnceLock<Regex> = OnceLock::new();
static AGGRESSIVE_RE: OnceLock<Regex> = OnceLock::new();
static STRAY_RE: OnceLock<Regex> = OnceLock::new();
static HORIZ_WS_RE: OnceLock<Regex> = OnceLock::new();
static NEWLINE_RE: OnceLock<Regex> = OnceLock::new();

/// Sanitize model output by removing reasoning/thinking text.
///
/// Handles several patterns common to local models (e.g., DeepSeek, Qwen):
/// 1. `<think>...</think>` blocks
/// 2. `<thinking>...</thinking>` blocks
/// 3. Fenced code blocks like ```response or ```thinking
/// 4. Leaked boundary markers and truncated tags
pub fn sanitize_reasoning(content: &str) -> SanitizeResult {
    let mut sanitized = content.to_string();
    let mut removed_reasoning = false;

    // Pattern 1: <think>...</think> blocks (case-insensitive, dot-matches-all)
    let think_re = THINK_RE.get_or_init(|| Regex::new(r"(?is)<think\b[^>]*>.*?<\/think>").unwrap());
    if think_re.is_match(&sanitized) {
        sanitized = think_re.replace_all(&sanitized, " ").to_string();
        removed_reasoning = true;
    }

    // Pattern 2: <thinking>...</thinking> blocks
    let thinking_re =
        THINKING_RE.get_or_init(|| Regex::new(r"(?is)<thinking\b[^>]*>.*?<\/thinking>").unwrap());
    if thinking_re.is_match(&sanitized) {
        sanitized = thinking_re.replace_all(&sanitized, " ").to_string();
        removed_reasoning = true;
    }

    // Pattern 3: DeepSeek-style fenced blocks (```response, ```thinking, etc.)
    let fenced_re = FENCED_RE.get_or_init(|| {
        Regex::new(r"(?is)```\s*(?:response|assistant_text|thinking|reasoning).*?```").unwrap()
    });
    if fenced_re.is_match(&sanitized) {
        sanitized = fenced_re.replace_all(&sanitized, " ").to_string();
        removed_reasoning = true;
    }

    // Pattern 3b: Bare boundary markers that leak through (e.g., </think> before tool calls)
    let leaked_re =
        LEAKED_RE.get_or_init(|| Regex::new(r"(?is)^.*?<\/(?:think|thinking)>").unwrap());
    let tool_call_re = TOOL_CALL_RE.get_or_init(|| Regex::new(r"(?i)<tool_call>").unwrap());
    if leaked_re.is_match(&sanitized) && tool_call_re.is_match(&sanitized) {
        let stripped = leaked_re.replace(&sanitized, "").trim().to_string();
        if !stripped.is_empty() && tool_call_re.is_match(&stripped) {
            sanitized = stripped;
            removed_reasoning = true;
        }
    }

    // Pattern 3c: Raw " response" / " answer" markers after thinking tags
    let markers_re = MARKERS_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:^|\n)\s*(?:response|answer|final|output)\s*(?:\n|$)").unwrap()
    });
    if markers_re.is_match(&sanitized) && tool_call_re.is_match(&sanitized) {
        let stripped = markers_re
            .replace_all(&sanitized, "\n")
            .to_string()
            .trim()
            .to_string();
        if !stripped.is_empty() {
            sanitized = stripped;
            removed_reasoning = true;
        }
    }

    // Pattern 4: Self-closing <think/> or <thinking/> tags
    let self_closing_re =
        SELF_CLOSING_RE.get_or_init(|| Regex::new(r"(?i)<think(?:ing)?\s*\/>").unwrap());
    if self_closing_re.is_match(&sanitized) {
        sanitized = self_closing_re.replace_all(&sanitized, " ").to_string();
        removed_reasoning = true;
    }

    // Pattern 5: Opening <think> without closing tag (truncated reasoning)
    let open_think_re = OPEN_THINK_RE.get_or_init(|| Regex::new(r"(?i)<think(?:ing)?\b").unwrap());
    if let Some(mat) = open_think_re.find(&sanitized) {
        let after_open = &sanitized[mat.start()..];
        let close_think_re =
            CLOSE_THINK_RE.get_or_init(|| Regex::new(r"(?i)<\/think(?:ing)?>").unwrap());
        if !close_think_re.is_match(after_open) {
            sanitized = sanitized[..mat.start()].to_string();
            removed_reasoning = true;
        } else {
            // Has close tag, but try more aggressive cleanup for robustness
            let aggressive_re = AGGRESSIVE_RE.get_or_init(|| {
                Regex::new(r"(?is)<think(?:ing)?\b[^>]*>.*?<\/think(?:ing)?>").unwrap()
            });
            if aggressive_re.is_match(&sanitized) {
                sanitized = aggressive_re.replace_all(&sanitized, " ").to_string();
                removed_reasoning = true;
            }
        }
    }

    // Pattern 6: Stray closing tags from truncated/streamed reasoning blocks
    let stray_re = STRAY_RE.get_or_init(|| Regex::new(r"(?i)<\/think(?:ing)?>").unwrap());
    if stray_re.is_match(&sanitized) {
        sanitized = stray_re.replace_all(&sanitized, " ").to_string();
        removed_reasoning = true;
    }

    // Pattern 7: Collapse horizontal whitespace and normalize excessive newlines
    let horiz_ws_re = HORIZ_WS_RE.get_or_init(|| Regex::new(r"[ \t]+").unwrap());
    let newline_re = NEWLINE_RE.get_or_init(|| Regex::new(r"\n{3,}").unwrap());
    sanitized = horiz_ws_re.replace_all(&sanitized, " ").to_string();
    sanitized = newline_re.replace_all(&sanitized, "\n\n").to_string();
    sanitized = sanitized.trim().to_string();

    SanitizeResult {
        content: sanitized,
        removed_reasoning,
    }
}

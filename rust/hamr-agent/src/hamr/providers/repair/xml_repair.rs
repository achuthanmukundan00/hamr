//! Port of `packages/coding-agent/src/hamr/providers/repair/xml-repair.ts`.
//!
//! XML repair — bounded auto-recovery for Qwen-style XML tool calls.
//!
//! Qwen-family models emit `<tool_call>` blocks in XML format. Local models
//! frequently produce:
//! - Unclosed `<tool_call>` tags
//! - Leaked `<thinking>` / `<think>` tags inside tool calls
//! - Mixed XML + text content where thinking bleeds into tool blocks
//! - Nested `<tool_call>` with missing closing function tags
//! - Bare function names without `<function=NAME>` wrapper
//!
//! Each repair is recorded in `fixes[]` for debugging. Returns `None` when
//! the input is unrepairable.

// ─── Public types ─────────────────────────────────────────────────────────────

/// Result of an XML repair attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlRepairResult {
    pub repaired: String,
    pub fixes: Vec<String>,
}

// ─── Main repair function ────────────────────────────────────────────────────

/// Attempt to repair malformed XML tool-call text from a local model.
///
/// Repairs applied in order:
/// 1. Strip leaked reasoning tags (`<think>`, `<thinking>`) inside tool-call blocks
/// 2. Balance `<tool_call>` ... `</tool_call>` pairs
/// 3. Balance `<function=...>` ... `</function>` pairs
/// 4. Balance `<parameter=...>` ... `</parameter>` pairs
/// 5. Extract only tool-call blocks if there's mixed content
/// 6. Wrap bare function names inside `<function=...>` tags
///
/// Returns `None` if the string is unrepairable (empty or contains no XML tags).
///
/// Mirror of `repairXml` in the TS source.
pub fn repair_xml(raw: &str) -> Option<XmlRepairResult> {
    let mut fixes: Vec<String> = Vec::new();
    let mut working = raw.trim().to_string();

    if working.is_empty() {
        return None;
    }
    if !contains_xml_tags(&working) {
        return None;
    }

    // Step 1: Strip leaked reasoning tags
    let (cleaned, had_reasoning) = strip_reasoning_from_xml(&working);
    if had_reasoning {
        fixes.push("stripped reasoning tags inside tool blocks".to_string());
        working = cleaned;
    }

    // Step 2: Balance <tool_call> ... </tool_call> pairs
    let tool_balanced = balance_tool_call_tags(&working);
    if tool_balanced != working {
        fixes.push("balanced <tool_call> tags".to_string());
        working = tool_balanced;
    }

    // Step 3: Balance <function=...> ... </function> pairs
    let func_balanced = balance_function_tags(&working);
    if func_balanced != working {
        fixes.push("balanced <function> tags".to_string());
        working = func_balanced;
    }

    // Step 4: Balance <parameter=...> ... </parameter> pairs
    let param_balanced = balance_parameter_tags(&working);
    if param_balanced != working {
        fixes.push("balanced <parameter> tags".to_string());
        working = param_balanced;
    }

    // Step 5: Extract only tool-call blocks if there's mixed content
    let extracted = extract_tool_call_blocks(&working);
    if extracted != working {
        fixes.push("extracted tool-call blocks from mixed content".to_string());
        working = extracted;
    }

    // Step 6: Wrap bare function names inside <tool_call> blocks
    let wrapped = wrap_bare_function_names(&working);
    if wrapped != working {
        fixes.push("wrapped bare function name in <function=...> tags".to_string());
        working = wrapped;
    }

    if working.is_empty() {
        return None;
    }

    Some(XmlRepairResult {
        repaired: working,
        fixes,
    })
}

// ─── Helper: contains XML tags ───────────────────────────────────────────────

/// Check if text contains any tool_call XML tags.
fn contains_xml_tags(text: &str) -> bool {
    let re = regex::Regex::new(r"(?i)<tool_call|</tool_call>").unwrap();
    re.is_match(text)
}

// ─── Helper: strip reasoning tags ────────────────────────────────────────────

/// Strip `<think>...</think>` and `<thinking>...</thinking>` blocks
/// that leaked into tool-call XML, plus DeepSeek-style ```response``` blocks.
///
/// Mirror of `stripReasoningFromXml` in the TS source.
fn strip_reasoning_from_xml(text: &str) -> (String, bool) {
    let mut had_reasoning = false;

    // Remove thinking blocks (case-insensitive, multiline via (?s) flag)
    let thinking_re = regex::Regex::new(r"(?si)<think\b[^>]*>.*?</think>").unwrap();
    let thinking2_re = regex::Regex::new(r"(?si)<thinking\b[^>]*>.*?</thinking>").unwrap();

    let mut cleaned = text.to_string();
    if thinking_re.is_match(&cleaned) || thinking2_re.is_match(&cleaned) {
        had_reasoning = true;
        cleaned = thinking_re.replace_all(&cleaned, "").to_string();
        cleaned = thinking2_re.replace_all(&cleaned, "").to_string();
    }

    // Also strip DeepSeek-style ```response``` blocks
    let response_re = regex::Regex::new(r"(?si)```\s*response\s*.*?```").unwrap();
    if response_re.is_match(&cleaned) {
        had_reasoning = true;
        cleaned = response_re.replace_all(&cleaned, "").to_string();
    }

    (cleaned.trim().to_string(), had_reasoning)
}

// ─── Helper: balance <tool_call> tags ────────────────────────────────────────

/// Balance `<tool_call> ... </tool_call>` pairs.
/// Adds missing closing tags or strips unclosed opening tags.
///
/// Mirror of `balanceToolCallTags` in the TS source.
fn balance_tool_call_tags(text: &str) -> String {
    let open_re = regex::Regex::new(r"(?i)<tool_call>").unwrap();
    let close_re = regex::Regex::new(r"(?i)</tool_call>").unwrap();

    let opens = open_re.find_iter(text).count();
    let closes = close_re.find_iter(text).count();

    if opens == closes {
        return text.to_string();
    }

    if opens > closes {
        // Add missing closing tags
        let missing = opens - closes;
        format!("{}{}", text, "</tool_call>\n".repeat(missing))
    } else {
        // More closing than opening — strip extra closing tags from the end
        let extra = closes - opens;
        let mut result = text.to_string();
        for _ in 0..extra {
            if let Some(last_close) = result.rfind("</tool_call>") {
                let after = result[last_close + "</tool_call>".len()..]
                    .trim()
                    .to_string();
                result = format!(
                    "{}{}",
                    &result[..last_close],
                    if after.is_empty() { "" } else { &after }
                );
            } else {
                break;
            }
        }
        result
    }
}

// ─── Helper: balance <function> tags ─────────────────────────────────────────

/// Balance `<function=NAME> ... </function>` pairs.
/// Inserts missing closing tags before the enclosing `</tool_call>` when possible.
///
/// Mirror of `balanceFunctionTags` in the TS source.
fn balance_function_tags(text: &str) -> String {
    let open_re = regex::Regex::new(r"(?i)<function=[^>]+>").unwrap();
    let close_re = regex::Regex::new(r"(?i)</function>").unwrap();

    let opens = open_re.find_iter(text).count();
    let closes = close_re.find_iter(text).count();

    if opens == closes {
        return text.to_string();
    }

    if opens > closes {
        // Add missing closing tags inside the last tool_call block
        let missing = opens - closes;
        let mut result = text.to_string();
        if let Some(tool_close_idx) = result.rfind("</tool_call>") {
            let before = &result[..tool_close_idx];
            let after = &result[tool_close_idx..];
            result = format!("{}\n{}\n{}", before, "</function>".repeat(missing), after);
        } else {
            result.push_str(&"\n</function>".repeat(missing));
        }
        result
    } else {
        // More closing than opening — strip extra closing tags
        let extra = closes - opens;
        let mut result = text.to_string();
        for _ in 0..extra {
            if let Some(last_close) = result.rfind("</function>") {
                let after = result[last_close + "</function>".len()..]
                    .trim()
                    .to_string();
                result = format!(
                    "{}{}",
                    &result[..last_close],
                    if after.is_empty() { "" } else { &after }
                );
            } else {
                break;
            }
        }
        result
    }
}

// ─── Helper: balance <parameter> tags ────────────────────────────────────────

/// Balance `<parameter=NAME> ... </parameter>` pairs.
///
/// Mirror of `balanceParameterTags` in the TS source.
fn balance_parameter_tags(text: &str) -> String {
    let open_re = regex::Regex::new(r"(?i)<parameter=[^>]+>").unwrap();
    let close_re = regex::Regex::new(r"(?i)</parameter>").unwrap();

    let opens = open_re.find_iter(text).count();
    let closes = close_re.find_iter(text).count();

    if opens == closes {
        return text.to_string();
    }

    if opens > closes {
        // Add missing closing tags
        let missing = opens - closes;
        let mut result = text.to_string();
        let func_close_idx = result.rfind("</function>");
        let tool_close_idx = result.rfind("</tool_call>");
        let insert_idx = func_close_idx.or(tool_close_idx).unwrap_or(result.len());

        if insert_idx < result.len() {
            let before = &result[..insert_idx];
            let after = &result[insert_idx..];
            result = format!("{}\n{}\n{}", before, "</parameter>".repeat(missing), after);
        } else {
            result.push_str(&"\n</parameter>".repeat(missing));
        }
        result
    } else {
        // More closing than opening — strip extras
        let extra = closes - opens;
        let mut result = text.to_string();
        for _ in 0..extra {
            if let Some(last_close) = result.rfind("</parameter>") {
                let after = result[last_close + "</parameter>".len()..]
                    .trim()
                    .to_string();
                result = format!(
                    "{}{}",
                    &result[..last_close],
                    if after.is_empty() { "" } else { &after }
                );
            } else {
                break;
            }
        }
        result
    }
}

// ─── Helper: extract tool-call blocks ────────────────────────────────────────

/// Extract only the tool-call blocks from text that may have mixed content.
/// Returns consecutive `<tool_call>...</tool_call>` blocks.
///
/// Mirror of `extractToolCallBlocks` in the TS source.
fn extract_tool_call_blocks(text: &str) -> String {
    let re = regex::Regex::new(r"(?si)<tool_call>(.*?)</tool_call>").unwrap();

    let blocks: Vec<String> = re
        .captures_iter(text)
        .filter_map(|cap| {
            let inner = cap.get(1)?.as_str().trim();
            if inner.is_empty() {
                None
            } else {
                Some(format!("<tool_call>\n{}\n</tool_call>", inner))
            }
        })
        .collect();

    if blocks.is_empty() {
        text.to_string()
    } else {
        blocks.join("\n")
    }
}

// ─── Helper: wrap bare function names ────────────────────────────────────────

/// Wrap bare function names that appear at the start of `<tool_call>` blocks
/// without the required `<function=NAME>` wrapper.
///
/// Example input:
///   `<tool_call>\nread\n<parameter=path>foo</parameter></tool_call>`
///
/// Example output:
///   `<tool_call>\n<function=read>\n<parameter=path>foo</parameter>\n</function>\n</tool_call>`
///
/// Mirror of `wrapBareFunctionNames` in the TS source.
fn wrap_bare_function_names(text: &str) -> String {
    if !contains_xml_tags(text) {
        return text.to_string();
    }

    let has_function_re = regex::Regex::new(r"(?i)<function=[^>]+>").unwrap();
    let block_re = regex::Regex::new(r"(?si)<tool_call>(.*?)</tool_call>").unwrap();

    // Quick scan: any tool_call block that lacks <function=...>
    let mut has_block_needing_fix = false;
    for cap in block_re.captures_iter(text) {
        if let Some(inner) = cap.get(1) {
            if inner.as_str().trim().is_empty() || !has_function_re.is_match(inner.as_str()) {
                has_block_needing_fix = true;
                break;
            }
        }
    }
    if !has_block_needing_fix {
        return text.to_string();
    }

    // Rebuild: wrap bare function names
    let mut result = String::new();
    let mut last_index = 0;

    for cap in block_re.captures_iter(text) {
        let full_match = cap.get(0).unwrap();
        let match_start = full_match.start();
        let match_end = full_match.end();

        // Append text before this match
        result.push_str(&text[last_index..match_start]);
        last_index = match_end;

        let inner = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if inner.is_empty() || has_function_re.is_match(inner) {
            // Already has <function= or empty — leave unchanged
            result.push_str(full_match.as_str());
            continue;
        }

        // Try to find a bare function name at the start of the inner content
        let trimmed = inner.trim_start();
        let fn_re = regex::Regex::new(r"^([a-zA-Z_][\w.]*)").unwrap();
        if let Some(fn_cap) = fn_re.captures(trimmed) {
            let fn_name = fn_cap.get(1).unwrap().as_str();
            let after_name = &trimmed[fn_name.len()..];

            result.push_str(&format!(
                "<tool_call>\n<function={}>{}\n</function>\n</tool_call>",
                fn_name, after_name
            ));
        } else {
            // Can't find a function name — leave unchanged
            result.push_str(full_match.as_str());
        }
    }

    // Append any trailing text after the last block
    result.push_str(&text[last_index..]);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── contains_xml_tags ──────────────────────────────────────

    #[test]
    fn test_contains_xml_tags_true() {
        assert!(contains_xml_tags("<tool_call>"));
        assert!(contains_xml_tags("</tool_call>"));
        assert!(contains_xml_tags("<TOOL_CALL>"));
    }

    #[test]
    fn test_contains_xml_tags_false() {
        assert!(!contains_xml_tags("plain text"));
        assert!(!contains_xml_tags(""));
    }

    // ─── repair_xml ─────────────────────────────────────────────

    #[test]
    fn test_repair_xml_empty() {
        assert_eq!(repair_xml(""), None);
        assert_eq!(repair_xml("  "), None);
    }

    #[test]
    fn test_repair_xml_no_tags() {
        assert_eq!(repair_xml("hello world"), None);
    }

    #[test]
    fn test_repair_xml_unclosed_tool_call() {
        let result = repair_xml(
            "<tool_call>\n<function=read>\n<parameter=path>/tmp\n</parameter>\n</function>",
        );
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.fixes.iter().any(|f| f.contains("balanced <tool_call>")));
        assert!(r.repaired.contains("</tool_call>"));
    }

    #[test]
    fn test_repair_xml_strips_thinking() {
        let result = repair_xml(
            "<tool_call>\n<thinking>some thoughts</thinking>\n<function=read>\n<parameter=path>/tmp\n</parameter>\n</function>\n</tool_call>",
        );
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(!r.repaired.to_lowercase().contains("thinking"));
        assert!(r.fixes.iter().any(|f| f.contains("stripped reasoning")));
    }

    #[test]
    fn test_repair_xml_bare_function_name() {
        let result = repair_xml("<tool_call>\nread\n<parameter=path>foo</parameter>\n</tool_call>");
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.repaired.contains("<function=read>"));
        assert!(r.fixes.iter().any(|f| f.contains("wrapped bare function")));
    }

    #[test]
    fn test_repair_xml_valid_no_changes() {
        let input = "<tool_call>\n<function=read>\n<parameter=path>/tmp</parameter>\n</function>\n</tool_call>";
        let result = repair_xml(input).unwrap();
        // Should still return a result but with no fixes needed
        // (extract_tool_call_blocks may still trigger due to formatting)
        assert!(result.repaired.contains("<tool_call>"));
        assert!(result.repaired.contains("read"));
    }

    // ─── strip_reasoning_from_xml ───────────────────────────────

    #[test]
    fn test_strip_reasoning_think_tags() {
        let (cleaned, had) =
            strip_reasoning_from_xml("<think>some thought</think><tool_call>stuff</tool_call>");
        assert!(had);
        assert!(!cleaned.contains("<think>"));
    }

    #[test]
    fn test_strip_reasoning_no_tags() {
        let (cleaned, had) = strip_reasoning_from_xml("<tool_call>stuff</tool_call>");
        assert!(!had);
        assert_eq!(cleaned, "<tool_call>stuff</tool_call>");
    }

    // ─── balance functions ──────────────────────────────────────

    #[test]
    fn test_balance_tool_call_tags_missing_close() {
        let result = balance_tool_call_tags("<tool_call>A</tool_call>\n<tool_call>B");
        assert!(result.ends_with("</tool_call>\n"));
    }

    #[test]
    fn test_balance_tool_call_tags_extra_close() {
        let result = balance_tool_call_tags("<tool_call>A</tool_call>\n</tool_call>\n</tool_call>");
        // Should strip extra closing tags
        assert_eq!(result.matches("</tool_call>").count(), 1);
    }

    #[test]
    fn test_balance_tool_call_tags_balanced() {
        let input = "<tool_call>A</tool_call>";
        assert_eq!(balance_tool_call_tags(input), input);
    }

    // ─── extract_tool_call_blocks ───────────────────────────────

    #[test]
    fn test_extract_blocks_no_blocks_returns_original() {
        assert_eq!(extract_tool_call_blocks("plain text"), "plain text");
    }

    #[test]
    fn test_extract_blocks_extracts_correctly() {
        let input =
            "some text <tool_call>block1</tool_call> more text <tool_call>block2</tool_call>";
        let result = extract_tool_call_blocks(input);
        assert!(result.contains("block1"));
        assert!(result.contains("block2"));
        assert!(!result.contains("some text"));
        assert!(!result.contains("more text"));
    }
}

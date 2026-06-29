use serde_json;

/// Result of the JSON repair process.
pub struct RepairResult {
    /// The repaired JSON string.
    pub repaired: String,
    /// A list of fixes applied during the repair process.
    pub fixes: Vec<String>,
}

/// Attempt to repair malformed JSON produced by a local model.
///
/// Repairs are applied in increasing order of invasiveness:
/// 1. Trim whitespace and surrounding noise
/// 2. Fix trailing commas
/// 3. Heuristic inner-quote repair
/// 4. Balance braces/brackets
///
/// Returns `None` if the string is unrepairable.
pub fn repair_json(raw: &str) -> Option<RepairResult> {
    let mut fixes = Vec::new();
    let mut working = raw.trim().to_string();

    if working.is_empty() {
        return None;
    }

    // Step 0: Extract the probable JSON region from surrounding text
    let extracted = extract_json_region(&working);
    if extracted != working {
        fixes.push("extracted JSON from surrounding text".to_string());
        working = extracted;
    }

    if working.is_empty() {
        return None;
    }

    // Step 1: Fix trailing commas
    let trailing_fixed = fix_trailing_commas(&working);
    if trailing_fixed != working {
        fixes.push("removed trailing commas".to_string());
        working = trailing_fixed;
    }

    // Step 2: Fix unescaped inner quotes (heuristic, conservative)
    let quotes_fixed = fix_inner_quotes(&working);
    if quotes_fixed != working {
        fixes.push("escaped inner quotes".to_string());
        working = quotes_fixed;
    }

    // Step 3: Balance braces and brackets
    let balanced = balance_braces(&working);
    if balanced != working {
        fixes.push("balanced braces/brackets".to_string());
        working = balanced;
    }

    // Step 4: Try to parse. If it works, return the repaired text.
    if is_valid_json(&working) {
        return Some(RepairResult {
            repaired: working,
            fixes,
        });
    }

    // Step 5: As a last resort, try trimming from the end (for truncated output)
    if let Some(truncated) = fix_truncated_object(&working) {
        if truncated != working {
            fixes.push("recovered truncated object".to_string());
            if is_valid_json(&truncated) {
                return Some(RepairResult {
                    repaired: truncated,
                    fixes,
                });
            }
        }
    }

    None
}

// --- Internal helpers ---

/// Extract a JSON object or array from surrounding text.
/// Looks for the most likely JSON region.
fn extract_json_region(text: &str) -> String {
    if text.starts_with('{') || text.starts_with('[') {
        return text.to_string();
    }

    let obj_start = text.find('{');
    let arr_start = text.find('[');

    let start = match (obj_start, arr_start) {
        (Some(o), Some(a)) => std::cmp::min(o, a),
        (Some(o), None) => o,
        (None, Some(a)) => a,
        (None, None) => return text.to_string(),
    };

    let opener = text.as_bytes()[start] as char;
    let closer = if opener == '{' { '}' } else { ']' };
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;

    let chars: Vec<char> = text.chars().collect();
    for i in start..chars.len() {
        let ch = chars[i];
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == opener {
            depth += 1;
        } else if ch == closer {
            depth -= 1;
            if depth == 0 {
                return chars[start..=i].iter().collect();
            }
        }
    }

    chars[start..].iter().collect()
}

/// Fix trailing commas before } or ].
fn fix_trailing_commas(json: &str) -> String {
    // Simple heuristic: remove comma if followed by whitespace and a closer
    // In TS it uses /,\s*([}\]])/g
    let mut result = String::with_capacity(json.len());
    let chars: Vec<char> = json.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                // Skip the comma and whitespace
                i = j;
                result.push(chars[i]);
                i += 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Heuristic: fix unescaped inner double-quotes inside string values.
fn fix_inner_quotes(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let mut in_string = false;
    let mut in_key = false;
    let mut escape = false;
    let mut colon_seen = false;

    let chars: Vec<char> = json.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];

        if escape {
            escape = false;
            result.push(ch);
            i += 1;
            continue;
        }

        if ch == '\\' {
            escape = true;
            result.push(ch);
            i += 1;
            continue;
        }

        if ch == '"' {
            if !in_string {
                in_string = true;
                in_key = !colon_seen;
                colon_seen = false;
                result.push(ch);
            } else {
                // Closing a string - peek ahead
                let rest = &json[i + 1..];
                let rest_trimmed = rest.trim_start();

                if !in_key
                    && !rest_trimmed.is_empty()
                    && !rest_trimmed.starts_with(',')
                    && !rest_trimmed.starts_with('}')
                    && !rest_trimmed.starts_with(']')
                    && !rest_trimmed.starts_with(':')
                    && !rest_trimmed.starts_with('\n')
                    && !rest_trimmed.starts_with('\r')
                {
                    result.push('\\');
                }
                in_string = false;
                in_key = false;
                result.push(ch);
            }
            i += 1;
            continue;
        }

        if !in_string {
            if ch == ':' {
                colon_seen = true;
            } else if ch == ',' || ch == '{' || ch == '[' {
                colon_seen = false;
            } else if !ch.is_whitespace() {
                colon_seen = false;
            }
        }

        result.push(ch);
        i += 1;
    }

    result
}

/// Balance unclosed braces and brackets using a stack.
fn balance_braces(json: &str) -> String {
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escape = false;

    for ch in json.chars() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }

        if ch == '{' {
            stack.push('}');
        } else if ch == '[' {
            stack.push(']');
        } else if ch == '}' || ch == ']' {
            if let Some(&last) = stack.last() {
                if last == ch {
                    stack.pop();
                }
            }
        }
    }

    if stack.is_empty() || stack.len() > 20 {
        return json.to_string();
    }

    let mut result = json.to_string();
    while let Some(closer) = stack.pop() {
        result.push(closer);
    }
    result
}

/// Attempt to recover a truncated JSON object.
fn fix_truncated_object(json: &str) -> Option<String> {
    let balanced = balance_braces(json);
    if balanced != json {
        return Some(balanced);
    }

    if json.ends_with(',') {
        let cleaned = format!("{}{}", json, '}');
        if is_valid_json(&cleaned) {
            return Some(cleaned);
        }

        let alt = format!("{}{}", &json[..json.len() - 1], '}');
        if is_valid_json(&alt) {
            return Some(alt);
        }
    }

    // Try appending synthetic closing
    let mut synthetic = json.trim_end().to_string();
    if let Some(last_char) = synthetic.pop() {
        if last_char != '}' && last_char != ']' {
            // This is simplified version of .replace(/[^}\]]$/, "").trimEnd()
            // we just popped the last char.
        } else {
            synthetic.push(last_char);
        }
    }

    let braces = count_unclosed(&synthetic);
    if braces > 0 && braces <= 5 {
        let closed = format!("{}{}", synthetic, "}".repeat(braces));
        if is_valid_json(&closed) {
            return Some(closed);
        }
    }

    None
}

fn count_unclosed(json: &str) -> usize {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;

    for ch in json.chars() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            if depth > 0 {
                depth -= 1;
            }
        }
    }
    depth
}

fn is_valid_json(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text).is_ok()
}

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

// =============================================================================
// ANSI Code Extraction
// =============================================================================

/// Extract an ANSI escape sequence starting at `pos` in `str`.
/// Returns (code, length) or None if no sequence starts at pos.
pub fn extract_ansi_code(str: &str, pos: usize) -> Option<(&str, usize)> {
    let bytes = str.as_bytes();
    if pos >= bytes.len() || bytes[pos] != 0x1b {
        return None;
    }

    let next = bytes.get(pos + 1)?;

    match next {
        // CSI: ESC [ ... m/G/K/H/J
        b'[' => {
            let mut j = pos + 2;
            while j < bytes.len() {
                let b = bytes[j];
                if matches!(b, b'm' | b'G' | b'K' | b'H' | b'J') {
                    return Some((&str[pos..=j], j + 1 - pos));
                }
                j += 1;
            }
            None
        }

        // OSC: ESC ] ... BEL or ESC ] ... ST
        b']' => {
            let mut j = pos + 2;
            while j < bytes.len() {
                if bytes[j] == 0x07 {
                    return Some((&str[pos..=j], j + 1 - pos));
                }
                if bytes[j] == 0x1b && bytes.get(j + 1) == Some(&b'\\') {
                    return Some((&str[pos..=j + 1], j + 2 - pos));
                }
                j += 1;
            }
            None
        }

        // APC: ESC _ ... BEL or ESC _ ... ST
        b'_' => {
            let mut j = pos + 2;
            while j < bytes.len() {
                if bytes[j] == 0x07 {
                    return Some((&str[pos..=j], j + 1 - pos));
                }
                if bytes[j] == 0x1b && bytes.get(j + 1) == Some(&b'\\') {
                    return Some((&str[pos..=j + 1], j + 2 - pos));
                }
                j += 1;
            }
            None
        }

        _ => None,
    }
}

// =============================================================================
// ANSI Code Tracker
// =============================================================================

/// Track active ANSI SGR codes to preserve styling across line breaks.
#[derive(Debug, Clone)]
pub struct AnsiCodeTracker {
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    inverse: bool,
    hidden: bool,
    strikethrough: bool,
    fg_color: Option<String>,
    bg_color: Option<String>,
    active_hyperlink: Option<String>, // OSC 8 hyperlink open sequence
}

impl AnsiCodeTracker {
    pub fn new() -> Self {
        AnsiCodeTracker {
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            blink: false,
            inverse: false,
            hidden: false,
            strikethrough: false,
            fg_color: None,
            bg_color: None,
            active_hyperlink: None,
        }
    }

    /// Process an ANSI code and update tracker state.
    pub fn process(&mut self, code: &str) {
        // OSC 8 hyperlink
        if code.starts_with("\x1b]8;") {
            let body = &code[4..];
            let terminator_len = if code.ends_with("\x1b\\") {
                2
            } else if code.ends_with('\x07') {
                1
            } else {
                0
            };
            let body = &body[..body.len() - terminator_len];
            if let Some(sep_idx) = body.find(';') {
                let url = &body[sep_idx + 1..];
                if url.is_empty() {
                    self.active_hyperlink = None;
                } else {
                    self.active_hyperlink = Some(code.to_string());
                }
            }
            return;
        }

        if !code.ends_with('m') {
            return;
        }

        let params = &code[2..code.len() - 1]; // strip "\x1b[" and "m"
        if params.is_empty() || params == "0" {
            // Full reset
            *self = AnsiCodeTracker::new();
            return;
        }

        for param in params.split(';') {
            let mut parts = param.split(':');
            let num: u8 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            match num {
                0 => *self = AnsiCodeTracker::new(),
                1 => self.bold = true,
                2 => self.dim = true,
                3 => self.italic = true,
                4 => self.underline = true,
                5 => self.blink = true,
                7 => self.inverse = true,
                8 => self.hidden = true,
                9 => self.strikethrough = true,
                21..=22 => {
                    self.bold = false;
                    self.dim = false;
                }
                23 => self.italic = false,
                24 => self.underline = false,
                25 => self.blink = false,
                27 => self.inverse = false,
                28 => self.hidden = false,
                29 => self.strikethrough = false,
                // Foreground colors
                30..=37 => self.fg_color = Some(num.to_string()),
                38 => self.fg_color = Some(format!("38;{}", parts.next().unwrap_or(""))),
                39 => self.fg_color = None,
                // Background colors
                40..=47 => self.bg_color = Some(num.to_string()),
                48 => self.bg_color = Some(format!("48;{}", parts.next().unwrap_or(""))),
                49 => self.bg_color = None,
                // Bright foreground
                90..=97 => self.fg_color = Some(num.to_string()),
                // Bright background
                100..=107 => self.bg_color = Some(num.to_string()),
                _ => {}
            }
        }
    }

    /// Get the currently active ANSI codes as a string.
    pub fn get_active_codes(&self) -> String {
        let mut codes = String::new();

        // Reopen hyperlink if active
        if let Some(ref hl) = self.active_hyperlink {
            codes.push_str(hl);
        }

        let mut sgr: Vec<String> = Vec::new();
        if self.bold {
            sgr.push("1".into());
        }
        if self.dim {
            sgr.push("2".into());
        }
        if self.italic {
            sgr.push("3".into());
        }
        if self.underline {
            sgr.push("4".into());
        }
        if self.blink {
            sgr.push("5".into());
        }
        if self.inverse {
            sgr.push("7".into());
        }
        if self.hidden {
            sgr.push("8".into());
        }
        if self.strikethrough {
            sgr.push("9".into());
        }
        if let Some(ref fg) = self.fg_color {
            sgr.push(fg.clone());
        }
        if let Some(ref bg) = self.bg_color {
            sgr.push(bg.clone());
        }

        if !sgr.is_empty() {
            codes.push_str(&format!("\x1b[{}m", sgr.join(";")));
        }

        codes
    }

    /// Get a reset sequence for line endings (resets underline but preserves background).
    pub fn get_line_end_reset(&self) -> Option<String> {
        if self.underline {
            Some("\x1b[24m".into()) // Reset underline only
        } else {
            None
        }
    }
}

impl Default for AnsiCodeTracker {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Visible Width
// =============================================================================

/// Normalize Thai/Lao AM vowels for terminal output.
pub fn normalize_terminal_output(str: &str) -> String {
    str.replace('\u{0e33}', "\u{0e4d}\u{0e32}")
        .replace('\u{0eb3}', "\u{0ecd}\u{0eb2}")
}

/// Calculate the visible width of a string in terminal columns.
/// Strips ANSI escape codes before measuring.
pub fn visible_width(str: &str) -> usize {
    if str.is_empty() {
        return 0;
    }

    // Fast path: pure ASCII printable
    if str.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return str.len();
    }

    // Strip ANSI codes
    let clean = strip_ansi(str);

    // Calculate width using grapheme clusters
    let mut width = 0;
    for grapheme in clean.graphemes(true) {
        width += grapheme_width(grapheme);
    }

    width
}

fn strip_ansi(str: &str) -> String {
    if !str.contains('\x1b') {
        return str.to_string();
    }

    let mut result = String::with_capacity(str.len());
    let mut i = 0;

    while i < str.len() {
        if str.as_bytes()[i] == 0x1b {
            if let Some((_, len)) = extract_ansi_code(str, i) {
                i += len;
                continue;
            }
        }

        let ch = str[i..].chars().next().unwrap();
        result.push(ch);
        i += ch.len_utf8();
    }

    result
}

fn grapheme_width(grapheme: &str) -> usize {
    if grapheme == "\t" {
        return 3; // Tab width
    }

    // Emoji sequences, singleton regional indicators, and common streaming
    // emoji intermediates are rendered as two terminal cells.
    if could_be_emoji(grapheme) {
        return 2;
    }

    // Sum scalar widths inside non-emoji grapheme clusters. This preserves the
    // source behavior for Thai/Lao AM clusters such as "กำ" while combining
    // marks remain zero-width.
    let mut width = 0;
    for ch in grapheme.chars() {
        if is_zero_width(ch) {
            continue;
        }
        width += UnicodeWidthChar::width(ch).unwrap_or(1);
    }

    width
}

fn is_zero_width(c: char) -> bool {
    matches!(c,
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}' | // Zero-width
        '\u{0300}'..='\u{036F}' | // Combining diacritical marks
        '\u{0483}'..='\u{0489}' |
        '\u{0591}'..='\u{05BD}' |
        '\u{0610}'..='\u{061A}' |
        '\u{064B}'..='\u{065F}' |
        '\u{0670}' |
        '\u{06D6}'..='\u{06DC}' |
        '\u{06DF}'..='\u{06E4}' |
        '\u{06E7}'..='\u{06E8}' |
        '\u{06EA}'..='\u{06ED}' |
        '\u{0711}' |
        '\u{0730}'..='\u{074A}' |
        '\u{07A6}'..='\u{07B0}' |
        '\u{0900}'..='\u{0902}' |
        '\u{093A}'..='\u{093C}' |
        '\u{0941}'..='\u{0948}' |
        '\u{094D}' |
        '\u{0951}'..='\u{0957}' |
        '\u{0962}'..='\u{0963}' |
        '\u{0981}'..='\u{0983}' |
        '\u{09BC}' |
        '\u{09C1}'..='\u{09C4}' |
        '\u{09CD}' |
        '\u{09E2}'..='\u{09E3}' |
        '\u{0A01}'..='\u{0A03}' |
        '\u{0A3C}' |
        '\u{0A41}'..='\u{0A42}' | '\u{0A47}'..='\u{0A48}' | '\u{0A4B}'..='\u{0A4D}' |
        '\u{0A70}'..='\u{0A71}' |
        '\u{0A81}'..='\u{0A83}' |
        '\u{0ABC}' |
        '\u{0AC1}'..='\u{0AC5}' | '\u{0AC7}'..='\u{0AC8}' |
        '\u{0ACD}' |
        '\u{0AE2}'..='\u{0AE3}' |
        '\u{0B01}'..='\u{0B03}' |
        '\u{0B3C}' |
        '\u{0B3F}' |
        '\u{0B41}'..='\u{0B44}' |
        '\u{0B4D}' |
        '\u{0B56}' |
        '\u{0B82}' |
        '\u{0BC0}' |
        '\u{0BCD}' |
        '\u{0C3E}'..='\u{0C40}' |
        '\u{0C46}'..='\u{0C48}' | '\u{0C4A}'..='\u{0C4D}' |
        '\u{0C55}'..='\u{0C56}' |
        '\u{0CBC}' |
        '\u{0CBF}' |
        '\u{0CC6}' |
        '\u{0CCC}'..='\u{0CCD}' |
        '\u{0CE2}'..='\u{0CE3}' |
        '\u{0D41}'..='\u{0D44}' |
        '\u{0D4D}' |
        '\u{0DCA}' |
        '\u{0DD2}'..='\u{0DD4}' |
        '\u{0DD6}' |
        '\u{0E31}' |
        '\u{0E34}'..='\u{0E3A}' |
        '\u{0E47}'..='\u{0E4E}' |
        '\u{0EB1}' |
        '\u{0EB4}'..='\u{0EB9}' |
        '\u{0EBB}'..='\u{0EBC}' |
        '\u{0EC8}'..='\u{0ECD}' |
        '\u{0F18}'..='\u{0F19}' |
        '\u{0F35}' |
        '\u{0F37}' |
        '\u{0F39}' |
        '\u{0F71}'..='\u{0F7E}' |
        '\u{0F80}'..='\u{0F84}' |
        '\u{0F86}'..='\u{0F87}' |
        '\u{0F90}'..='\u{0F97}' |
        '\u{0F99}'..='\u{0FBC}' |
        '\u{0FC6}' |
        '\u{102D}'..='\u{1030}' |
        '\u{1032}'..='\u{1037}' |
        '\u{1039}'..='\u{103A}' |
        '\u{103D}'..='\u{103E}' |
        '\u{1058}'..='\u{1059}' |
        '\u{105E}'..='\u{1060}' |
        '\u{1071}'..='\u{1074}' |
        '\u{1082}' |
        '\u{1085}'..='\u{1086}' |
        '\u{108D}' |
        '\u{109D}' |
        '\u{1160}'..='\u{11FF}' | // Hangul Jungseong/Jongseong
        '\u{135D}'..='\u{135F}' |
        '\u{1712}'..='\u{1714}' |
        '\u{1732}'..='\u{1734}' |
        '\u{1752}'..='\u{1753}' |
        '\u{1772}'..='\u{1773}' |
        '\u{17B4}'..='\u{17B5}' |
        '\u{17B7}'..='\u{17BD}' |
        '\u{17C6}' |
        '\u{17C9}'..='\u{17D3}' |
        '\u{17DD}' |
        '\u{180B}'..='\u{180D}' |
        '\u{1885}'..='\u{1886}' |
        '\u{18A9}' |
        '\u{1920}'..='\u{1922}' |
        '\u{1927}'..='\u{1928}' |
        '\u{1932}' |
        '\u{1939}'..='\u{193B}' |
        '\u{1A17}'..='\u{1A18}' |
        '\u{1B00}'..='\u{1B03}' |
        '\u{1B34}' |
        '\u{1B36}'..='\u{1B3A}' |
        '\u{1B3C}' |
        '\u{1B42}' |
        '\u{1B6B}'..='\u{1B73}' |
        '\u{1DC0}'..='\u{1DFF}' |
        '\u{200E}'..='\u{200F}' |
        '\u{202A}'..='\u{202E}' |
        '\u{2060}'..='\u{2064}' |
        '\u{2066}'..='\u{206F}' |
        '\u{20D0}'..='\u{20F0}' |
        '\u{2CEF}'..='\u{2CF1}' |
        '\u{2D7F}' |
        '\u{2DE0}'..='\u{2DFF}' |
        '\u{A66F}'..='\u{A672}' |
        '\u{A674}'..='\u{A67D}' |
        '\u{A69E}'..='\u{A69F}' |
        '\u{A6F0}'..='\u{A6F1}' |
        '\u{A802}' |
        '\u{A806}' |
        '\u{A80B}' |
        '\u{A825}'..='\u{A826}' |
        '\u{A8C4}'..='\u{A8C5}' |
        '\u{A8E0}'..='\u{A8F1}' |
        '\u{A926}'..='\u{A92D}' |
        '\u{A947}'..='\u{A951}' |
        '\u{A980}'..='\u{A982}' |
        '\u{A9B3}' |
        '\u{A9B6}'..='\u{A9B9}' |
        '\u{A9BC}' |
        '\u{AA29}'..='\u{AA2E}' |
        '\u{AA31}'..='\u{AA32}' |
        '\u{AA35}'..='\u{AA36}' |
        '\u{AA43}' |
        '\u{AA4C}' |
        '\u{AAB0}' |
        '\u{AAB2}'..='\u{AAB4}' |
        '\u{AAB7}'..='\u{AAB8}' |
        '\u{AABE}'..='\u{AABF}' |
        '\u{AAC1}' |
        '\u{AAEC}'..='\u{AAED}' |
        '\u{AAF6}' |
        '\u{ABE5}' |
        '\u{ABE8}' |
        '\u{ABED}' |
        '\u{FB1E}' |
        '\u{FE00}'..='\u{FE0F}' |
        '\u{FE20}'..='\u{FE2F}' |
        '\u{101FD}' |
        '\u{10A01}'..='\u{10A03}' | '\u{10A05}'..='\u{10A06}' |
        '\u{10A0C}'..='\u{10A0F}' |
        '\u{10A38}'..='\u{10A3A}' |
        '\u{10A3F}' |
        '\u{11001}' |
        '\u{11038}'..='\u{11046}' |
        '\u{11080}'..='\u{11081}' |
        '\u{110B3}'..='\u{110B6}' |
        '\u{110B9}'..='\u{110BA}' |
        '\u{11100}'..='\u{11102}' |
        '\u{11127}'..='\u{1112B}' |
        '\u{1112D}'..='\u{11134}' |
        '\u{11180}'..='\u{11181}' |
        '\u{111B6}'..='\u{111BE}' |
        '\u{116AB}' |
        '\u{116AD}' |
        '\u{116B0}'..='\u{116B5}' |
        '\u{116B7}' |
        '\u{16F8F}'..='\u{16F92}' |
        '\u{1D167}'..='\u{1D169}' |
        '\u{1D173}'..='\u{1D17A}' |
        '\u{1D185}'..='\u{1D18B}' |
        '\u{1D1AA}'..='\u{1D1AD}' |
        '\u{1D242}'..='\u{1D244}' |
        '\u{E0001}' |
        '\u{E0020}'..='\u{E007F}' |
        '\u{E0100}'..='\u{E01EF}'
    )
}

fn could_be_emoji(grapheme: &str) -> bool {
    let cp = grapheme.chars().next().map(|c| c as u32).unwrap_or(0);
    // Broad heuristic matching the TS version
    (0x1f000..=0x1fbff).contains(&cp)  // Emoji and Pictographs
        || (0x2300..=0x23ff).contains(&cp) // Misc Technical
        || (0x2600..=0x27bf).contains(&cp) // Misc Symbols, Dingbats
        || (0x2b50..=0x2b55).contains(&cp) // Stars/Circles
        || grapheme.contains('\u{FE0F}') // Variation Selector-16
        || grapheme.graphemes(true).count() > 2 // Multi-codepoint sequences
}

// =============================================================================
// Truncate to Width
// =============================================================================

/// Truncate a string to fit within `max_width` visible columns.
/// Preserves ANSI escape codes. Adds ellipsis if truncated.
pub fn truncate_to_width(str: &str, max_width: usize, ellipsis: Option<&str>) -> String {
    let ellipsis = ellipsis.unwrap_or("…");
    let ellipsis_width = visible_width(ellipsis);

    if visible_width(str) <= max_width {
        return str.to_string();
    }

    if ellipsis_width > max_width {
        return String::new();
    }

    if max_width == ellipsis_width {
        if ellipsis.is_empty() {
            return "\x1b[0m".to_string();
        }
        return format!("\x1b[0m{}\x1b[0m", ellipsis);
    }

    let available = max_width.saturating_sub(ellipsis_width);

    // Build a mapping: for each grapheme in the clean (ANSI-stripped) text,
    // record its byte range in the clean text and its visible width.
    let clean = strip_ansi(str);
    let graphemes: Vec<(&str, usize, usize)> = {
        // (text, byte_start, byte_end)
        let mut result = Vec::new();
        for (start, g) in clean.grapheme_indices(true) {
            result.push((g, start, start + g.len()));
        }
        result
    };

    // Walk graphemes until we exceed available width, tracking which ones to include
    let mut visible = 0;
    let mut grapheme_idx = 0;
    for (i, (g, _, _)) in graphemes.iter().enumerate() {
        let w = grapheme_width(g);
        if visible + w > available {
            break;
        }
        visible += w;
        grapheme_idx = i + 1;
    }

    // Reconstruct the result by walking the original string, copying characters
    // (including ANSI codes) until we've consumed `grapheme_idx` graphemes.
    let mut result = String::new();
    let mut clean_g_count = 0usize;
    let mut i = 0;
    let original_bytes = str.as_bytes();

    while i < original_bytes.len() && clean_g_count < grapheme_idx {
        // Check for ANSI escape sequence
        if original_bytes[i] == 0x1b {
            if let Some((code, code_len)) = extract_ansi_code(str, i) {
                result.push_str(code);
                i += code_len;
                continue;
            }
        }

        // Find the next grapheme boundary in the original text
        let graphemes_here: Vec<(usize, &str)> =
            UnicodeSegmentation::grapheme_indices(&str[i..], true).collect();
        if let Some((_, g)) = graphemes_here.first() {
            result.push_str(g);
            i += g.len();
            clean_g_count += 1;
        } else {
            // Fallback: advance by one byte
            result.push(original_bytes[i] as char);
            i += 1;
            clean_g_count += 1;
        }
    }

    result.push_str("\x1b[0m"); // SGR reset
    result.push_str(ellipsis);
    result.push_str("\x1b[0m"); // reset after ellipsis

    result
}

// =============================================================================
// Word Wrap with ANSI
// =============================================================================

/// Word-wrap text to fit within `width` visible columns, preserving ANSI codes.
pub fn wrap_text_with_ansi(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let mut result: Vec<String> = Vec::new();
    let mut tracker = AnsiCodeTracker::new();

    for input_line in &lines {
        let prefix = if !result.is_empty() {
            tracker.get_active_codes()
        } else {
            String::new()
        };
        let wrapped = wrap_single_line(&format!("{}{}", prefix, input_line), width);
        for line in wrapped {
            result.push(line);
        }
        update_tracker_from_text(input_line, &mut tracker);
    }

    if result.is_empty() {
        vec![String::new()]
    } else {
        result
    }
}

fn wrap_single_line(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }

    if visible_width(line) <= width {
        return vec![line.to_string()];
    }

    let mut wrapped: Vec<String> = Vec::new();
    let mut tracker = AnsiCodeTracker::new();
    let tokens = split_into_tokens_with_ansi(line);
    let mut current_line = String::new();
    let mut current_visible = 0;

    for token in &tokens {
        let token_visible = visible_width(token);
        let is_whitespace = token.trim().is_empty();

        // Token itself is too long — break it
        if token_visible > width && !is_whitespace {
            if !current_line.is_empty() {
                if let Some(ref reset) = tracker.get_line_end_reset() {
                    current_line.push_str(reset);
                }
                wrapped.push(current_line.clone());
                current_line.clear();
                current_visible = 0;
            }
            let broken = break_long_word(token, width, &tracker);
            let len = broken.len();
            for (idx, line) in broken.into_iter().enumerate() {
                if idx < len - 1 {
                    wrapped.push(line);
                } else {
                    current_line = line;
                    current_visible = visible_width(&current_line);
                }
            }
            continue;
        }

        let total_needed = current_visible + token_visible;

        if total_needed > width && current_visible > 0 {
            let trimmed = current_line.trim_end().to_string();
            if let Some(ref reset) = tracker.get_line_end_reset() {
                wrapped.push(format!("{}{}", trimmed, reset));
            } else {
                wrapped.push(trimmed);
            }
            if is_whitespace {
                current_line = tracker.get_active_codes();
                current_visible = 0;
            } else {
                current_line = format!("{}{}", tracker.get_active_codes(), token);
                current_visible = token_visible;
            }
        } else {
            current_line.push_str(token);
            current_visible += token_visible;
        }

        update_tracker_from_text(token, &mut tracker);
    }

    if !current_line.is_empty() {
        wrapped.push(current_line.trim_end().to_string());
    }

    if wrapped.is_empty() {
        vec![String::new()]
    } else {
        wrapped
    }
}

fn split_into_tokens_with_ansi(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    while i < chars.len() {
        let remaining: String = chars[i..].iter().collect();
        if let Some((code, _len)) = extract_ansi_code(&remaining, 0) {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push(code.to_string());
            i += code.chars().count();
            continue;
        }

        if chars[i].is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            let mut ws = String::new();
            while i < chars.len() && chars[i].is_whitespace() {
                let remaining: String = chars[i..].iter().collect();
                if extract_ansi_code(&remaining, 0).is_some() {
                    break;
                }
                ws.push(chars[i]);
                i += 1;
            }
            tokens.push(ws);
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn break_long_word(word: &str, width: usize, tracker: &AnsiCodeTracker) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = tracker.get_active_codes();
    let mut current_width = 0;

    // Walk graphemes, handling ANSI codes
    let clean = strip_ansi(word);
    let graphemes: Vec<&str> = clean.graphemes(true).collect();
    let _g_idx = 0;
    let _byte_pos = 0;

    for g in &graphemes {
        let w = grapheme_width(g);

        if current_width + w > width && current_width > 0 {
            if let Some(ref reset) = tracker.get_line_end_reset() {
                current_line.push_str(reset);
            }
            lines.push(current_line);
            current_line = tracker.get_active_codes();
            current_width = 0;
        }

        // Find this grapheme in the original (with ANSI) and copy it
        // Simplified: just push the grapheme chars
        current_line.push_str(g);
        current_width += w;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![word.to_string()]
    } else {
        lines
    }
}

fn update_tracker_from_text(text: &str, tracker: &mut AnsiCodeTracker) {
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    while i < chars.len() {
        let remaining: String = chars[i..].iter().collect();
        if let Some((code, _len)) = extract_ansi_code(&remaining, 0) {
            tracker.process(code);
            i += code.chars().count();
            continue;
        }
        i += 1;
    }
}

// =============================================================================
// Character Classification
// =============================================================================

pub const PUNCTUATION_CHARS: &str = "(){}[]<>.,;:'\"!?+-=*/\\|&%^$#@~`";

/// Check if the first character of a string is punctuation.
pub fn is_punctuation_char(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| PUNCTUATION_CHARS.contains(c))
}

/// Check if a string is only whitespace.
pub fn is_whitespace_str(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_whitespace())
}

/// Check if a single character is whitespace.
pub fn is_whitespace_char(c: char) -> bool {
    c.is_whitespace()
}

// =============================================================================
// Column Slicing (pi-identical port from TS src/utils.ts:1057-1115)
// =============================================================================

/// Extract a range of visible columns from a line. Handles ANSI codes and wide chars.
/// @param strict - If true, exclude wide chars at boundary that would extend past the range
pub fn slice_by_column(line: &str, start_col: usize, length: usize, strict: bool) -> String {
    slice_with_width(line, start_col, length, strict).0
}

/// Like slice_by_column but also returns the actual visible width of the result.
pub fn slice_with_width(
    line: &str,
    start_col: usize,
    length: usize,
    strict: bool,
) -> (String, usize) {
    if length == 0 {
        return (String::new(), 0);
    }
    let end_col = start_col + length;
    let mut result = String::new();
    let mut result_width = 0usize;
    let mut current_col = 0usize;
    let mut i = 0;
    let mut pending_ansi = String::new();

    while i < line.len() {
        if let Some((code, code_len)) = extract_ansi_code(line, i) {
            if current_col >= start_col && current_col < end_col {
                result.push_str(code);
            } else if current_col < start_col {
                pending_ansi.push_str(code);
            }
            i += code_len;
            continue;
        }

        // Find text run until next ANSI code
        let mut text_end = i;
        while text_end < line.len() && extract_ansi_code(line, text_end).is_none() {
            text_end += 1;
        }

        for g in UnicodeSegmentation::graphemes(&line[i..text_end], true) {
            let w = visible_width(g);
            let in_range = current_col >= start_col && current_col < end_col;
            let fits = !strict || current_col + w <= end_col;
            if in_range && fits {
                if !pending_ansi.is_empty() {
                    result.push_str(&pending_ansi);
                    pending_ansi.clear();
                }
                result.push_str(g);
                result_width += w;
            }
            current_col += w;
            if current_col >= end_col {
                break;
            }
        }
        i = text_end;
        if current_col >= end_col {
            break;
        }
    }
    (result, result_width)
}

// =============================================================================
// Segment Extraction for Overlay Compositing (pi-identical from TS:1117-1187)
// =============================================================================

/// Result of extract_segments — before and after regions of a composited line.
pub struct SegmentsResult {
    pub before: String,
    pub before_width: usize,
    pub after: String,
    pub after_width: usize,
}

/// Extract "before" and "after" segments from a line in a single pass.
/// Used for overlay compositing where we need content before and after the overlay region.
/// Preserves styling from before the overlay that should affect content after it.
pub fn extract_segments(
    line: &str,
    before_end: usize,
    after_start: usize,
    after_len: usize,
    strict_after: bool,
) -> SegmentsResult {
    let mut before = String::new();
    let mut before_width = 0usize;
    let mut after = String::new();
    let mut after_width = 0usize;
    let mut current_col = 0usize;
    let mut i = 0;
    let mut pending_ansi_before = String::new();
    let mut after_started = false;
    let after_end = after_start + after_len;

    // Track styling state so "after" inherits styling from before the overlay
    let mut style_tracker = AnsiCodeTracker::new();

    while i < line.len() {
        if let Some((code, code_len)) = extract_ansi_code(line, i) {
            // Track all SGR codes to know styling state at afterStart
            style_tracker.process(code);
            // Include ANSI codes in their respective segments
            if current_col < before_end {
                pending_ansi_before.push_str(code);
            } else if current_col >= after_start && current_col < after_end && after_started {
                // Only include after we've started "after" (styling already prepended)
                after.push_str(code);
            }
            i += code_len;
            continue;
        }

        let mut text_end = i;
        while text_end < line.len() && extract_ansi_code(line, text_end).is_none() {
            text_end += 1;
        }

        for g in UnicodeSegmentation::graphemes(&line[i..text_end], true) {
            let w = visible_width(g);

            if current_col < before_end && current_col + w <= before_end {
                if !pending_ansi_before.is_empty() {
                    before.push_str(&pending_ansi_before);
                    pending_ansi_before.clear();
                }
                before.push_str(g);
                before_width += w;
            } else if current_col >= after_start && current_col < after_end {
                let fits = !strict_after || current_col + w <= after_end;
                if fits {
                    // On first "after" grapheme, prepend inherited styling from before overlay
                    if !after_started {
                        let active = style_tracker.get_active_codes();
                        if !active.is_empty() {
                            after.push_str(&active);
                        }
                        after_started = true;
                    }
                    after.push_str(g);
                    after_width += w;
                }
            }

            current_col += w;
            // Early exit
            if after_len == 0 {
                if current_col >= before_end {
                    break;
                }
            } else if current_col >= after_end {
                break;
            }
        }
        i = text_end;
        if after_len == 0 {
            if current_col >= before_end {
                break;
            }
        } else if current_col >= after_end {
            break;
        }
    }

    SegmentsResult {
        before,
        before_width,
        after,
        after_width,
    }
}

// =============================================================================
// Background Application (pi-identical from TS src/utils.ts:893-901)
// =============================================================================

/// Apply background color to a line, padding to full width.
pub fn apply_background_to_line<F: Fn(&str) -> String>(
    line: &str,
    width: usize,
    bg_fn: &F,
) -> String {
    let visible_len = visible_width(line);
    let padding_needed = width.saturating_sub(visible_len);
    let padding = " ".repeat(padding_needed);
    let with_padding = format!("{}{}", line, padding);
    bg_fn(&with_padding)
}

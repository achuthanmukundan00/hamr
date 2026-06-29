use crate::tui::Component;

// ── Text widget ──────────────────────────────────────────────────────────────

/// Text widget — displays text with word wrapping and padding.
pub struct Text {
    content: String,
    padding_x: u16,
    padding_y: u16,
    bg_fn: Option<Box<dyn Fn(&str) -> String>>,
    /// Optional shimmer animation state.
    shimmer: Option<Shimmer>,
}

impl Text {
    pub fn new(
        content: &str,
        padding_x: u16,
        padding_y: u16,
        bg_fn: Option<Box<dyn Fn(&str) -> String>>,
    ) -> Self {
        Text {
            content: content.to_string(),
            padding_x,
            padding_y,
            bg_fn,
            shimmer: None,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.content = text.to_string();
    }

    /// Enable character-by-character shimmer animation.
    ///
    /// ```rust,ignore
    /// Text::new("Working…", 1, 0, None)
    ///     .with_shimmer()
    ///     .render(80);
    /// ```
    pub fn with_shimmer(mut self) -> Self {
        self.shimmer = Some(Shimmer::default());
        self
    }

    /// Set shimmer speed (characters per frame tick).
    pub fn with_shimmer_speed(mut self, chars_per_frame: usize) -> Self {
        if let Some(ref mut s) = self.shimmer {
            s.speed = chars_per_frame;
        } else {
            self.shimmer = Some(Shimmer {
                offset: 0,
                speed: chars_per_frame,
                highlight_width: 3,
            });
        }
        self
    }

    /// Advance the shimmer animation by one tick.
    pub fn tick_shimmer(&mut self) {
        if let Some(ref mut s) = self.shimmer {
            let clean_len = crate::utils::visible_width(&self.content);
            s.offset = (s.offset + s.speed) % (clean_len.max(1) + s.highlight_width);
        }
    }

    /// Apply shimmer effect to a string.
    fn apply_shimmer(&self, text: &str) -> String {
        let shimmer = match &self.shimmer {
            Some(s) => s,
            None => return text.to_string(),
        };

        // Strip ANSI for character-level processing
        let clean = strip_all_ansi(text);
        let chars: Vec<char> = clean.chars().collect();
        let _total = chars.len();

        let mut result = String::new();
        for (i, ch) in chars.iter().enumerate() {
            if i >= shimmer.offset && i < shimmer.offset + shimmer.highlight_width {
                // Highlighted character: bright white bold
                result.push_str(&format!("\x1b[1;97m{}\x1b[0m", *ch));
            } else if i >= shimmer.offset.saturating_sub(1) && i < shimmer.offset {
                // Leading edge: dim
                result.push_str(&format!("\x1b[2m{}\x1b[22m", *ch));
            } else {
                // Normal character
                result.push(*ch);
            }
        }
        result
    }
}

/// Shimmer animation state.
#[derive(Debug, Clone)]
pub struct Shimmer {
    /// Current position offset (in characters).
    offset: usize,
    /// Characters advanced per frame tick.
    speed: usize,
    /// Number of highlighted characters in the pulse wave.
    highlight_width: usize,
}

impl Default for Shimmer {
    fn default() -> Self {
        Shimmer {
            offset: 0,
            speed: 1,
            highlight_width: 3,
        }
    }
}

impl Component for Text {
    fn render(&self, width: u16) -> Vec<String> {
        let inner = width.saturating_sub(self.padding_x * 2);
        let spacer = " ".repeat(self.padding_x as usize);
        let mut lines = vec!["".to_string(); self.padding_y as usize];

        let display_text = self.apply_shimmer(&self.content);

        for line in crate::utils::wrap_text_with_ansi(&display_text, inner as usize) {
            let padded = format!("{}{}", spacer, line);
            lines.push(if let Some(ref bg) = self.bg_fn {
                bg(&padded)
            } else {
                padded
            });
        }
        lines.extend(vec!["".to_string(); self.padding_y as usize]);
        lines
    }

    fn invalidate(&mut self) {}
}

fn strip_all_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut i = 0;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            if let Some((_, len)) = crate::utils::extract_ansi_code(text, i) {
                i += len;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

// ── TruncatedText widget ─────────────────────────────────────────────────────

/// TruncatedText widget — single-line text that truncates to fit width.
pub struct TruncatedText {
    content: String,
    padding_x: u16,
    padding_y: u16,
}

impl TruncatedText {
    pub fn new(content: &str, padding_x: u16, padding_y: u16) -> Self {
        TruncatedText {
            content: content.to_string(),
            padding_x,
            padding_y,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.content = text.to_string();
    }
}

impl Component for TruncatedText {
    fn render(&self, width: u16) -> Vec<String> {
        let inner = width.saturating_sub(self.padding_x * 2) as usize;
        let truncated = crate::utils::truncate_to_width(&self.content, inner, None);
        let mut lines = vec!["".to_string(); self.padding_y as usize];
        lines.push(format!(
            "{}{}",
            " ".repeat(self.padding_x as usize),
            truncated
        ));
        lines.extend(vec!["".to_string(); self.padding_y as usize]);
        lines
    }
    fn invalidate(&mut self) {}
}

// ── Spacer widget ────────────────────────────────────────────────────────────

/// Spacer widget — empty vertical space.
pub struct Spacer {
    lines: u16,
}

impl Spacer {
    pub fn new(lines: u16) -> Self {
        Spacer { lines }
    }
}

impl Component for Spacer {
    fn render(&self, _width: u16) -> Vec<String> {
        vec!["".to_string(); self.lines as usize]
    }
    fn invalidate(&mut self) {}
}

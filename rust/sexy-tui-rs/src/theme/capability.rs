/// Progressive enhancement capability tiers.

/// Terminal capability tiers for progressive enhancement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityTier {
    /// 16 ANSI colors, thin borders, ASCII icons, bold/dim/underline
    Baseline = 0,
    /// Full 24-bit palette, hex color tokens resolved accurately
    TrueColor = 1,
    /// Nerd Font icons, box-drawing heavy/double, braille spinners
    NerdFont = 2,
    /// Kitty graphics images, sync output (CSI 2026), full key protocol
    KittyProtocol = 3,
    /// Undercurl, cursor color (DECSCUSR), OSC 9 progress, OSC 777 notify
    GpuTerminal = 4,
}

/// Detect the current terminal's capability tier.
pub fn detect_tier() -> CapabilityTier {
    // Check for GPU terminal features
    if let Ok(term) = std::env::var("TERM") {
        let term_lower = term.to_lowercase();
        if term_lower.contains("kitty")
            || term_lower.contains("ghostty")
            || term_lower.contains("wezterm")
        {
            return CapabilityTier::GpuTerminal;
        }
        if term_lower.contains("xterm-kitty") {
            return CapabilityTier::KittyProtocol;
        }
    }

    // Check for Kitty protocol support
    if let Ok(program) = std::env::var("TERM_PROGRAM") {
        if program == "iTerm.app" || program == "WezTerm" {
            return CapabilityTier::KittyProtocol;
        }
    }

    // Check for true color
    if let Ok(color) = std::env::var("COLORTERM") {
        if color == "truecolor" || color == "24bit" {
            return CapabilityTier::TrueColor;
        }
    }

    CapabilityTier::Baseline
}

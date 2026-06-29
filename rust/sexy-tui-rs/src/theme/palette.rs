/// Color palette utilities — hex → ANSI color resolution.

/// Convert a hex color string to an ANSI true color foreground escape.
pub fn apply_fg(color: &str, text: &str) -> String {
    let rgb = hex_to_rgb(color);
    format!("\x1b[38;2;{};{};{}m{}\x1b[39m", rgb.0, rgb.1, rgb.2, text)
}

/// Convert a hex color string to an ANSI true color background escape.
pub fn apply_bg(color: &str, text: &str) -> String {
    let rgb = hex_to_rgb(color);
    format!("\x1b[48;2;{};{};{}m{}\x1b[49m", rgb.0, rgb.1, rgb.2, text)
}

/// Convert a hex color string to RGB values.
fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
        (r, g, b)
    } else {
        (255, 255, 255) // fallback to white
    }
}

/// Generate a palette of ANSI colors from a base hex color.
pub fn generate_palette(base_hex: &str) -> Vec<String> {
    let (r, g, b) = hex_to_rgb(base_hex);
    let mut palette = Vec::with_capacity(8);

    // Generate shades: lighter and darker variants
    for i in 0..8 {
        let factor = 0.5 + (i as f64 * 0.125);
        let sr = ((r as f64 * factor).min(255.0)) as u8;
        let sg = ((g as f64 * factor).min(255.0)) as u8;
        let sb = ((b as f64 * factor).min(255.0)) as u8;
        palette.push(format!("#{:02x}{:02x}{:02x}", sr, sg, sb));
    }
    palette
}

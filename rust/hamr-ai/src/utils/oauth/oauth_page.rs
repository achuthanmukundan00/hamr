//! Port of `packages/ai/src/utils/oauth/oauth_page.ts`
//!
//! HTML responses shown by the local OAuth callback server.
//! Carries the Hamr brand mark and wordmark.

/// HTML shown on successful OAuth completion.
pub fn oauth_success_html(message: &str) -> String {
    render_page(
        "Authentication successful",
        "Authentication successful",
        message,
        None,
    )
}

/// HTML shown on OAuth error.
pub fn oauth_error_html(message: &str) -> String {
    render_page(
        "Authentication failed",
        "Authentication failed",
        message,
        None,
    )
}

/// HTML shown on OAuth error with details.
pub fn oauth_error_html_with_details(message: &str, details: &str) -> String {
    render_page(
        "Authentication failed",
        "Authentication failed",
        message,
        Some(details),
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_page(title: &str, heading: &str, message: &str, details: Option<&str>) -> String {
    let title = escape_html(title);
    let heading = escape_html(heading);
    let message = escape_html(message);
    let details = details.map(escape_html);

    let details_html = details.map_or_else(String::new, |d| {
        format!(r#"    <div class="details">{}</div>"#, d)
    });

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <style>
    :root {{
      --text: #fafafa;
      --text-dim: #a1a1aa;
      --page-bg: #0a0a0e;
      --accent: #8abeb7;
      --font-sans: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";
      --font-mono: "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    }}
    * {{ box-sizing: border-box; }}
    html {{ color-scheme: dark; }}
    body {{
      margin: 0;
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 24px;
      background: var(--page-bg);
      color: var(--text);
      font-family: var(--font-sans);
      text-align: center;
    }}
    main {{
      width: 100%;
      max-width: 560px;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
    }}
    .brand {{
      display: inline-flex;
      align-items: center;
      gap: 10px;
      margin-bottom: 28px;
      font-family: var(--font-mono);
      font-weight: 700;
      font-size: 18px;
      color: var(--text);
    }}
    .brand-mark {{
      color: var(--accent);
      font-size: 22px;
      line-height: 1;
    }}
    h1 {{
      margin: 0 0 10px;
      font-size: 28px;
      line-height: 1.15;
      font-weight: 650;
      color: var(--text);
    }}
    p {{
      margin: 0;
      line-height: 1.7;
      color: var(--text-dim);
      font-size: 15px;
    }}
    .details {{
      margin-top: 16px;
      font-family: var(--font-mono);
      font-size: 13px;
      color: var(--text-dim);
      white-space: pre-wrap;
      word-break: break-word;
    }}
  </style>
</head>
<body>
  <main>
    <div class="brand"><span class="brand-mark">&#x2692;</span><span>hamr</span></div>
    <h1>{heading}</h1>
    <p>{message}</p>
    {details_html}
  </main>
</body>
</html>"#,
        title = title,
        heading = heading,
        message = message,
        details_html = details_html,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The inherited Pi logo SVG path fragment that must not appear in Hamr pages.
    const PI_LOGO_PATH_FRAGMENT: &str = "M165.29 165.29";

    fn pages() -> Vec<(&'static str, String)> {
        vec![
            ("success", oauth_success_html("You can close this window.")),
            ("error", oauth_error_html("Something went wrong.")),
        ]
    }

    #[test]
    fn carries_hamr_brand_mark() {
        for (_name, html) in &pages() {
            assert!(html.contains("hamr"), "expected 'hamr' in {_name} page");
            assert!(
                html.contains("&#x2692;"),
                "expected brand glyph in {_name} page"
            );
        }
    }

    #[test]
    fn does_not_embed_inherited_pi_logo() {
        for (_name, html) in &pages() {
            assert!(
                !html.contains(PI_LOGO_PATH_FRAGMENT),
                "{_name} page contains Pi logo"
            );
            assert!(!html.contains("<svg"), "{_name} page contains SVG element");
        }
    }

    #[test]
    fn does_not_name_pi_as_product() {
        for (_name, html) in &pages() {
            assert!(
                !html.contains(" Pi ") && !html.contains(" pi "),
                "{_name} page names Pi as product"
            );
        }
    }

    #[test]
    fn escape_html_entities() {
        let html = oauth_success_html("Error: <script>alert('xss')</script> & \"quotes\"");
        assert!(html.contains("&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"));
        assert!(html.contains("&amp;"));
        assert!(html.contains("&quot;quotes&quot;"));
    }
}

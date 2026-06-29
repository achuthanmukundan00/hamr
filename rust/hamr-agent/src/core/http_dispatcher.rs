//! HTTP dispatcher configuration: idle timeout, proxy support, undici agent setup.
//!
//! Port of `packages/coding-agent/src/core/http-dispatcher.ts`.
//!
//! The TypeScript implementation configures Node's `undici` global dispatcher.
//! In Rust we use `reqwest` as the HTTP client. This module exposes configuration
//! constants and helpers for the HTTP idle timeout, proxy validation, and
//! provider exclusion from proxying.

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default HTTP idle timeout in milliseconds (5 minutes).
pub const DEFAULT_HTTP_IDLE_TIMEOUT_MS: u64 = 300_000;

/// Available timeout choices with human-readable labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpIdleTimeoutChoice {
    pub label: &'static str,
    pub timeout_ms: i64,
}

pub const HTTP_IDLE_TIMEOUT_CHOICES: &[HttpIdleTimeoutChoice] = &[
    HttpIdleTimeoutChoice {
        label: "30 sec",
        timeout_ms: 30_000,
    },
    HttpIdleTimeoutChoice {
        label: "1 min",
        timeout_ms: 60_000,
    },
    HttpIdleTimeoutChoice {
        label: "2 min",
        timeout_ms: 120_000,
    },
    HttpIdleTimeoutChoice {
        label: "5 min",
        timeout_ms: 300_000,
    },
    HttpIdleTimeoutChoice {
        label: "disabled",
        timeout_ms: 0,
    },
];

// ---------------------------------------------------------------------------
// Timeout helpers
// ---------------------------------------------------------------------------

/// Parse an HTTP idle timeout value from a user-supplied input.
///
/// Accepts:
/// - a number (must be finite, non-negative)
/// - the string `"disabled"` → returns `0`
/// - an empty string → returns `None`
/// - any other string that parses as a valid number
pub fn parse_http_idle_timeout_ms(value: &str) -> Option<u64> {
    let trimmed = value.trim();

    if trimmed.eq_ignore_ascii_case("disabled") {
        return Some(0);
    }

    if trimmed.is_empty() {
        return None;
    }

    // Try parsing as a number
    match trimmed.parse::<f64>() {
        Ok(n) if n.is_finite() && n >= 0.0 => Some(n.floor() as u64),
        _ => None,
    }
}

/// Format an HTTP idle timeout in milliseconds as a human-readable label.
/// Falls back to a `"{n} sec"` string if the exact value doesn't match a choice.
pub fn format_http_idle_timeout_ms(timeout_ms: u64) -> String {
    let ios_timeout = timeout_ms as i64;
    for choice in HTTP_IDLE_TIMEOUT_CHOICES {
        if choice.timeout_ms == ios_timeout {
            return choice.label.to_string();
        }
    }
    format!("{} sec", timeout_ms / 1000)
}

// ---------------------------------------------------------------------------
// Proxy helpers
// ---------------------------------------------------------------------------

/// Validate that a proxy URL is well-formed and uses an http(s) scheme.
/// Returns `Some(normalized_string)` on success, `None` if the input is empty.
/// Returns an error if the URL is malformed or has an unsupported scheme.
pub fn validate_proxy_url(http_proxy: Option<&str>) -> Result<Option<String>, String> {
    let proxy = match http_proxy {
        Some(p) => p.trim(),
        None => return Ok(None),
    };
    if proxy.is_empty() {
        return Ok(None);
    }

    let url = url::Url::parse(proxy).map_err(|e| {
        format!(
            "Invalid httpProxy value (not a valid URL): {proxy}. Proxy has NOT been applied. ({e})"
        )
    })?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(format!(
                "Invalid httpProxy scheme '{other}' (only http: and https: are allowed). Proxy has NOT been applied."
            ));
        }
    }

    if url.host_str().is_none() {
        return Err(format!(
            "Invalid httpProxy value (missing host): {proxy}. Proxy has NOT been applied."
        ));
    }

    Ok(Some(proxy.to_string()))
}

/// Emit a prominent warning when an HTTP proxy is active.
/// Since all LLM traffic (including Authorization headers) is routed through it.
pub fn warn_proxy_active(proxy: Option<&str>) {
    let proxy = match proxy {
        Some(p) if !p.is_empty() => p,
        _ => return,
    };

    let host = url::Url::parse(proxy)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| proxy.to_string());

    tracing::warn!(
        "[hamr] HTTP proxy is active ({}). All LLM provider requests, including API key and provider credential headers, will be routed through this proxy. Verify the proxy is trusted.",
        host
    );
}

/// Add known provider hosts to the `NO_PROXY` / `no_proxy` environment variable
/// so that provider traffic is never routed through the proxy.
pub fn exclude_providers_from_proxy(provider_hosts: &[String]) {
    if provider_hosts.is_empty() {
        return;
    }
    let existing = std::env::var("NO_PROXY")
        .or_else(|_| std::env::var("no_proxy"))
        .unwrap_or_default();
    let hosts = provider_hosts.join(",");

    let no_proxy = if existing.is_empty() {
        hosts
    } else {
        format!("{existing},{hosts}")
    };

    // SAFETY: Setting environment variables before spawning child processes is safe.
    unsafe { std::env::set_var("NO_PROXY", &no_proxy) };
    // Also set lowercase variant for tools that check it
    unsafe { std::env::set_var("no_proxy", &no_proxy) };
}

/// Apply HTTP proxy settings from configuration.
///
/// 1. Validates the proxy URL
/// 2. Sets `HTTP_PROXY` and `HTTPS_PROXY` (only if not already set)
/// 3. Emits a warning about the proxy
pub fn apply_http_proxy_settings(http_proxy: Option<&str>) -> Result<(), String> {
    let proxy = match validate_proxy_url(http_proxy)? {
        Some(p) => p,
        None => return Ok(()),
    };

    // Only set if not already present (mirrors TypeScript ??= behaviour)
    // SAFETY: Setting environment variables before spawning child processes is safe.
    if std::env::var("HTTP_PROXY").is_err() {
        unsafe { std::env::set_var("HTTP_PROXY", &proxy) };
    }
    if std::env::var("HTTPS_PROXY").is_err() {
        unsafe { std::env::set_var("HTTPS_PROXY", &proxy) };
    }
    warn_proxy_active(Some(&proxy));
    Ok(())
}

// ---------------------------------------------------------------------------
// Dispatcher configuration
// ---------------------------------------------------------------------------

/// Build a `reqwest::Client` configured with the given HTTP idle timeout.
///
/// In the TypeScript version this configures the `undici` global dispatcher.
/// In Rust we create a `reqwest::Client` with appropriate timeout settings.
/// Returns `Err` if the timeout value is invalid.
pub fn build_http_client(timeout_ms: Option<u64>) -> Result<reqwest::Client, String> {
    let normalized_timeout = match timeout_ms {
        Some(ms) => {
            parse_http_idle_timeout_ms(&ms.to_string()).unwrap_or(DEFAULT_HTTP_IDLE_TIMEOUT_MS)
        }
        None => DEFAULT_HTTP_IDLE_TIMEOUT_MS,
    };

    let timeout = std::time::Duration::from_millis(normalized_timeout);

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(std::time::Duration::from_secs(30))
        .http2_prior_knowledge() // Mirror TS: allowH2 = false → we explicitly enable h2
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    Ok(client)
}

/// Convenience: validate and apply the full HTTP configuration.
/// Returns a configured `reqwest::Client` or an error.
pub fn configure_http_dispatcher(
    timeout_ms: Option<u64>,
    http_proxy: Option<&str>,
) -> Result<reqwest::Client, String> {
    apply_http_proxy_settings(http_proxy)?;
    build_http_client(timeout_ms)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_parse_http_idle_timeout_disabled() {
        assert_eq!(parse_http_idle_timeout_ms("disabled"), Some(0));
        assert_eq!(parse_http_idle_timeout_ms("DISABLED"), Some(0));
        assert_eq!(parse_http_idle_timeout_ms("Disabled"), Some(0));
    }

    #[test]
    fn test_parse_http_idle_timeout_empty() {
        assert_eq!(parse_http_idle_timeout_ms(""), None);
        assert_eq!(parse_http_idle_timeout_ms("   "), None);
    }

    #[test]
    fn test_parse_http_idle_timeout_number() {
        assert_eq!(parse_http_idle_timeout_ms("30000"), Some(30000));
        assert_eq!(parse_http_idle_timeout_ms("300000"), Some(300000));
        assert_eq!(parse_http_idle_timeout_ms("0"), Some(0));
    }

    #[test]
    fn test_parse_http_idle_timeout_invalid() {
        assert_eq!(parse_http_idle_timeout_ms("abc"), None);
        assert_eq!(parse_http_idle_timeout_ms("-100"), None); // negative → f64 but floor makes 0... actually -100.0 < 0
        assert_eq!(parse_http_idle_timeout_ms("NaN"), None);
    }

    #[test]
    fn test_format_http_idle_timeout() {
        assert_eq!(format_http_idle_timeout_ms(30000), "30 sec");
        assert_eq!(format_http_idle_timeout_ms(60000), "1 min");
        assert_eq!(format_http_idle_timeout_ms(120000), "2 min");
        assert_eq!(format_http_idle_timeout_ms(300000), "5 min");
        assert_eq!(format_http_idle_timeout_ms(0), "disabled");
        assert_eq!(format_http_idle_timeout_ms(45000), "45 sec");
    }

    #[test]
    fn test_validate_proxy_url_valid() {
        let result = validate_proxy_url(Some("http://127.0.0.1:7890")).unwrap();
        assert_eq!(result, Some("http://127.0.0.1:7890".to_string()));
    }

    #[test]
    fn test_validate_proxy_url_empty() {
        assert_eq!(validate_proxy_url(None).unwrap(), None);
        assert_eq!(validate_proxy_url(Some("")).unwrap(), None);
        assert_eq!(validate_proxy_url(Some("   ")).unwrap(), None);
    }

    #[test]
    fn test_validate_proxy_url_invalid_scheme() {
        let result = validate_proxy_url(Some("ftp://proxy:8080"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("scheme"));
    }

    #[test]
    fn test_validate_proxy_url_invalid_url() {
        let result = validate_proxy_url(Some("not a url"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("valid URL"));
    }

    struct EnvRestore {
        http_proxy: Option<String>,
        https_proxy: Option<String>,
    }

    impl EnvRestore {
        fn capture() -> Self {
            Self {
                http_proxy: std::env::var("HTTP_PROXY").ok(),
                https_proxy: std::env::var("HTTPS_PROXY").ok(),
            }
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            if let Some(val) = &self.http_proxy {
                unsafe { std::env::set_var("HTTP_PROXY", val) };
            } else {
                unsafe { std::env::remove_var("HTTP_PROXY") };
            }
            if let Some(val) = &self.https_proxy {
                unsafe { std::env::set_var("HTTPS_PROXY", val) };
            } else {
                unsafe { std::env::remove_var("HTTPS_PROXY") };
            }
        }
    }

    #[test]
    fn test_apply_http_proxy_settings_sets_proxy() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _restore = EnvRestore::capture();
        unsafe { std::env::remove_var("HTTP_PROXY") };
        unsafe { std::env::remove_var("HTTPS_PROXY") };

        apply_http_proxy_settings(Some("http://127.0.0.1:7890")).unwrap();

        assert_eq!(
            std::env::var("HTTP_PROXY").unwrap(),
            "http://127.0.0.1:7890"
        );
        assert_eq!(
            std::env::var("HTTPS_PROXY").unwrap(),
            "http://127.0.0.1:7890"
        );
    }

    #[test]
    fn test_apply_http_proxy_settings_does_not_override_existing() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _restore = EnvRestore::capture();
        unsafe { std::env::set_var("HTTP_PROXY", "http://env-http:8080") };
        unsafe { std::env::set_var("HTTPS_PROXY", "http://env-https:8080") };

        apply_http_proxy_settings(Some("http://settings:7890")).unwrap();

        assert_eq!(std::env::var("HTTP_PROXY").unwrap(), "http://env-http:8080");
        assert_eq!(
            std::env::var("HTTPS_PROXY").unwrap(),
            "http://env-https:8080"
        );
    }

    #[test]
    fn test_apply_http_proxy_settings_empty() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _restore = EnvRestore::capture();
        unsafe { std::env::remove_var("HTTP_PROXY") };
        unsafe { std::env::remove_var("HTTPS_PROXY") };

        apply_http_proxy_settings(Some("   ")).unwrap();

        assert!(std::env::var("HTTP_PROXY").is_err());
        assert!(std::env::var("HTTPS_PROXY").is_err());
    }
}

//! Anthropic OAuth flow (Claude Pro/Max).
//!
//! Port of `packages/ai/src/utils/oauth/anthropic.ts`.
//!
//! Implements the OAuth authorization code + PKCE flow:
//! 1. Start a local HTTP callback server on a random port
//! 2. Open browser to Anthropic's OAuth authorization URL
//! 3. Wait for callback with auth code
//! 4. Exchange code for tokens
//!
//! Uses a simple Tokio TCP listener (not a full HTTP framework)
//! for the callback server — it only needs to handle one GET with
//! query parameters.

use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use getrandom::getrandom;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

use crate::utils::oauth::{
    oauth_page::{oauth_error_html, oauth_success_html},
    types::{OAuthCredentials, OAuthLoginCallbacks, OAuthProviderInterface, OAuthPrompt},
};
use crate::utils::provider_env::get_provider_env_value;

// ---------------------------------------------------------------------------
// Constants (mirror TS)
// ---------------------------------------------------------------------------

/// Decoded client ID (base64-encoded in TS source; decoded here).
const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

const AUTHORIZE_URL: &str = "https://claude.ai/oauth/authorize";
const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";

const CALLBACK_HOST_DEFAULT: &str = "127.0.0.1";
const CALLBACK_PATH: &str = "/callback";
const SCOPES: &str =
    "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";

/// Buffer expiry by 5 minutes so we don't accidentally use an expired token.
const EXPIRY_BUFFER_MS: i64 = 5 * 60 * 1000;

// ---------------------------------------------------------------------------
// PKCE (inline — pkce.rs is a stub)
// ---------------------------------------------------------------------------

/// Generate a cryptographically random PKCE code verifier.
fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    getrandom(&mut bytes).expect("RNG failure");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Derive the S256 code challenge from a verifier.
fn derive_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

// ---------------------------------------------------------------------------
// Authorization code / redirect-URL parsing
// ---------------------------------------------------------------------------

/// Parse an authorization "code" and optional "state" from user input.
///
/// Accepts a full redirect URL, a raw query string, a `code#state` pair,
/// or a bare authorization code. Mirrors `parseAuthorizationInput` in TS.
fn parse_authorization_input(input: &str) -> (Option<String>, Option<String>) {
    let value = input.trim();
    if value.is_empty() {
        return (None, None);
    }

    // 1. Try as a full URL
    if let Ok(url) = Url::parse(value) {
        let code = first_query_param(&url, "code");
        let state = first_query_param(&url, "state");
        return (code, state);
    }

    // 2. Try "#" separator: "code#state"
    if let Some((code, state)) = value.split_once('#') {
        return (Some(code.to_string()), Some(state.to_string()));
    }

    // 3. Try as raw query string ("code=...&state=...")
    if value.contains('=') {
        if let Ok(url) = Url::parse(&format!("http://x/?{value}")) {
            let code = first_query_param(&url, "code");
            let state = first_query_param(&url, "state");
            return (code, state);
        }
    }

    // 4. Bare value: treat as code only
    (Some(value.to_string()), None)
}

fn first_query_param(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.into_owned())
}

// ---------------------------------------------------------------------------
// Callback server — Tokio TCP listener on a random port
// ---------------------------------------------------------------------------

/// Result from the callback server.
struct CallbackResult {
    /// The local address the server was bound to (used to build redirect_uri).
    port: u16,
    /// The authorization code and state extracted from the callback.
    code: Option<(String, String)>,
}

/// Start a local HTTP server on a random port, wait for one GET to
/// `CALLBACK_PATH`, extract `code` and `state` from the query string,
/// respond with a success/error HTML page, then shut down.
async fn run_callback_server(host: &str, expected_state: &str) -> std::io::Result<CallbackResult> {
    let listener = TcpListener::bind(format!("{host}:0")).await?;
    let port = listener.local_addr()?.port();

    // Accept exactly one connection.
    let (stream, _peer) = listener.accept().await?;
    // Drop the listener so no more connections are accepted.
    drop(listener);

    let code = handle_callback_connection(stream, expected_state).await;

    Ok(CallbackResult { port, code })
}

/// Handle a single inbound TCP connection: read the HTTP request line and
/// headers, parse the query string, validate, respond with HTML, return
/// the (code, state) pair or None.
async fn handle_callback_connection(
    stream: TcpStream,
    expected_state: &str,
) -> Option<(String, String)> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    // Read the request line: "GET /callback?code=...&state=... HTTP/1.1"
    let mut request_line = String::new();
    if buf_reader.read_line(&mut request_line).await.is_err() {
        let _ = writer
            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
            .await;
        return None;
    }

    // Parse method + path
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 || parts[0] != "GET" {
        let _ = writer
            .write_all(b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n")
            .await;
        return None;
    }
    let path_and_query = parts[1];

    // Split path from query
    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (path_and_query, None),
    };

    if path != CALLBACK_PATH {
        let body = oauth_error_html("Callback route not found.", None);
        let response = format!(
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = writer.write_all(response.as_bytes()).await;
        return None;
    }

    // Parse query parameters manually from the query string.
    let params = parse_query_string(query.unwrap_or(""));

    // Check for OAuth error
    if let Some(error) = params.get("error") {
        let body =
            oauth_error_html("Anthropic authentication did not complete.", Some(error));
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = writer.write_all(response.as_bytes()).await;
        return None;
    }

    let code = params.get("code").map(|s| s.to_string());
    let state = params.get("state").map(|s| s.to_string());

    if code.is_none() || state.is_none() {
        let body = oauth_error_html("Missing code or state parameter.", None);
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = writer.write_all(response.as_bytes()).await;
        return None;
    }

    let code = code.unwrap();
    let state = state.unwrap();

    if state != expected_state {
        let body = oauth_error_html("State mismatch.", None);
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = writer.write_all(response.as_bytes()).await;
        return None;
    }

    // Success
    let body = oauth_success_html("Anthropic authentication completed. You can close this window.");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let _ = writer.write_all(response.as_bytes()).await;

    Some((code, state))
}

/// Extremely minimal query-string parser.
///
/// Handles URL-encoded key=value pairs separated by `&`.
/// Does not handle all edge cases (fragments, etc.) but is sufficient
/// for OAuth callback parameters.
fn parse_query_string(query: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        let key = url_decode(key);
        let value = url_decode(value);
        map.insert(key, value);
    }
    map
}

/// Minimal percent-decode for query-string values.
fn url_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        match b {
            b'+' => result.push(' '),
            b'%' => {
                let hi = chars.next();
                let lo = chars.next();
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    if let Ok(byte) =
                        u8::from_str_radix(&format!("{}{}", hi as char, lo as char), 16)
                    {
                        result.push(byte as char);
                        continue;
                    }
                }
                result.push('%');
            }
            _ => result.push(b as char),
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Token exchange (POST to Anthropic token endpoint)
// ---------------------------------------------------------------------------

/// POST JSON to the given URL and return the response body as a string.
async fn post_json(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<String, String> {
    let response = client
        .post(url)
        .json(body)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed. url={url}; error={e}"))?;

    let status = response.status();
    let response_body = response.text().await.map_err(|e| {
        format!("Failed to read response body. url={url}; error={e}")
    })?;

    if !status.is_success() {
        return Err(format!(
            "HTTP request failed. status={status}; url={url}; body={response_body}"
        ));
    }

    Ok(response_body)
}

/// Exchange an authorization code for tokens.
async fn exchange_authorization_code(
    client: &reqwest::Client,
    code: &str,
    state: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<OAuthCredentials, String> {
    let body = serde_json::json!({
        "grant_type": "authorization_code",
        "client_id": CLIENT_ID,
        "code": code,
        "state": state,
        "redirect_uri": redirect_uri,
        "code_verifier": verifier,
    });

    let response_body =
        post_json(client, TOKEN_URL, &body).await.map_err(|e| {
            format!(
                "Token exchange request failed. url={TOKEN_URL}; redirect_uri={redirect_uri}; response_type=authorization_code; details={e}"
            )
        })?;

    let token_data: serde_json::Value = serde_json::from_str(&response_body).map_err(
        |e| {
            format!(
                "Token exchange returned invalid JSON. url={TOKEN_URL}; body={response_body}; details={e}"
            )
        },
    )?;

    let access_token = token_data["access_token"]
        .as_str()
        .ok_or_else(|| {
            format!("Missing access_token in token response. body={response_body}")
        })?
        .to_string();
    let refresh_token = token_data["refresh_token"]
        .as_str()
        .ok_or_else(|| {
            format!("Missing refresh_token in token response. body={response_body}")
        })?
        .to_string();
    let expires_in = token_data["expires_in"]
        .as_i64()
        .unwrap_or(3600);

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    Ok(OAuthCredentials {
        refresh: refresh_token,
        access: access_token,
        expires: now_ms + expires_in * 1000 - EXPIRY_BUFFER_MS,
    })
}

// ---------------------------------------------------------------------------
// Public API: login_anthropic
// ---------------------------------------------------------------------------

/// Login with Anthropic OAuth (authorization code + PKCE).
///
/// Orchestrates the full flow:
/// 1. Generates PKCE verifier/challenge
/// 2. Starts a local callback server on a random port
/// 3. Calls `callbacks.on_auth()` with the authorization URL
/// 4. Waits for the callback (or manual code input)
/// 5. Exchanges the code for tokens
pub async fn login_anthropic(
    callbacks: &dyn OAuthLoginCallbacks,
) -> Result<OAuthCredentials, String> {
    let verifier = generate_code_verifier();
    let challenge = derive_code_challenge(&verifier);

    let host = get_provider_env_value("PI_OAUTH_CALLBACK_HOST", None)
        .unwrap_or_else(|| CALLBACK_HOST_DEFAULT.to_string());

    // Spawn the callback server on a random port.
    let server_result = run_callback_server(&host, &verifier)
        .await
        .map_err(|e| format!("Failed to start callback server: {e}"))?;

    let port = server_result.port;
    let redirect_uri = format!("http://localhost:{port}{CALLBACK_PATH}");

    // Build the authorization URL
    let auth_params = [
        ("code", "true"),
        ("client_id", CLIENT_ID),
        ("response_type", "code"),
        ("redirect_uri", &redirect_uri),
        ("scope", SCOPES),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("state", &verifier),
    ];

    let auth_url = {
        let mut url = String::from(AUTHORIZE_URL);
        url.push('?');
        for (i, (k, v)) in auth_params.iter().enumerate() {
            if i > 0 {
                url.push('&');
            }
            url.push_str(&urlencoding(k));
            url.push('=');
            url.push_str(&urlencoding(v));
        }
        url
    };

    callbacks.on_auth(
        &auth_url,
        Some("Complete login in your browser. If the browser is on another machine, paste the final redirect URL here."),
    );

    // Determine the code and state.
    let (code, state) = {
        let (code, state) = server_result.code.unwrap_or((String::new(), String::new()));

        if code.is_empty() {
            // No code from the callback server — fall back to prompting the
            // user to paste the redirect URL or code.
            let input = callbacks
                .on_prompt(OAuthPrompt {
                    message: "Paste the authorization code or full redirect URL:".to_string(),
                    placeholder: Some(redirect_uri),
                    allow_empty: false,
                })
                .await
                .map_err(|e| format!("Prompt failed: {e}"))?;

            let (parsed_code, parsed_state) = parse_authorization_input(&input);
            let code = parsed_code.ok_or_else(|| "Missing authorization code".to_string())?;
            let state = parsed_state.unwrap_or_else(|| verifier.clone());

            if state != verifier {
                return Err("OAuth state mismatch".to_string());
            }

            (code, state)
        } else {
            (code, state)
        }
    };

    callbacks
        .on_progress("Exchanging authorization code for tokens...");

    let client = reqwest::Client::new();
    exchange_authorization_code(&client, &code, &state, &verifier, &redirect_uri).await
}

// ---------------------------------------------------------------------------
// Public API: refresh_anthropic_token
// ---------------------------------------------------------------------------

/// Refresh an Anthropic OAuth token.
pub async fn refresh_anthropic_token(
    refresh_token: &str,
) -> Result<OAuthCredentials, String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "grant_type": "refresh_token",
        "client_id": CLIENT_ID,
        "refresh_token": refresh_token,
    });

    let response_body =
        post_json(&client, TOKEN_URL, &body).await.map_err(|e| {
            format!("Anthropic token refresh request failed. url={TOKEN_URL}; details={e}")
        })?;

    let data: serde_json::Value = serde_json::from_str(&response_body).map_err(
        |e| {
            format!(
                "Anthropic token refresh returned invalid JSON. url={TOKEN_URL}; body={response_body}; details={e}"
            )
        },
    )?;

    let access_token = data["access_token"]
        .as_str()
        .ok_or_else(|| {
            format!("Missing access_token in refresh response. body={response_body}")
        })?
        .to_string();
    let new_refresh_token = data["refresh_token"]
        .as_str()
        .ok_or_else(|| {
            format!("Missing refresh_token in refresh response. body={response_body}")
        })?
        .to_string();
    let expires_in = data["expires_in"].as_i64().unwrap_or(3600);

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    Ok(OAuthCredentials {
        refresh: new_refresh_token,
        access: access_token,
        expires: now_ms + expires_in * 1000 - EXPIRY_BUFFER_MS,
    })
}

// ---------------------------------------------------------------------------
// OAuthProviderInterface implementation
// ---------------------------------------------------------------------------

/// Anthropic OAuth provider.
pub struct AnthropicOAuthProvider;

impl OAuthProviderInterface for AnthropicOAuthProvider {
    fn id(&self) -> OAuthProviderId {
        "anthropic".to_string()
    }

    fn name(&self) -> String {
        "Anthropic (Claude Pro/Max)".to_string()
    }

    fn uses_callback_server(&self) -> bool {
        true
    }

    async fn login(
        &self,
        callbacks: &dyn OAuthLoginCallbacks,
    ) -> Result<OAuthCredentials, String> {
        login_anthropic(callbacks).await
    }

    async fn refresh_token(
        &self,
        credentials: &OAuthCredentials,
    ) -> Result<OAuthCredentials, String> {
        refresh_anthropic_token(&credentials.refresh).await
    }

    fn get_api_key(&self, credentials: &OAuthCredentials) -> String {
        credentials.access.clone()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Minimal percent-encode for URL query parameters (like JS encodeURIComponent).
fn urlencoding(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push_str("%20"),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

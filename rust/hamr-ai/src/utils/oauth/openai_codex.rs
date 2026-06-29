//! Port of `packages/ai/src/utils/oauth/openai-codex.ts`
//!
//! OpenAI Codex (ChatGPT OAuth) flow with PKCE.
//!
//! Supports two login methods:
//! 1. **Browser login** — opens a local HTTP server on port 1455, launches the
//!    browser with the OAuth authorize URL, and receives the callback.
//! 2. **Device code login** — uses the Codex device-code flow for headless environments.
//!
//! Notes:
//! - Uses PKCE (Proof Key for Code Exchange) for security.
//! - Requires a local TCP listener for browser callback (uses `tokio::net::TcpListener`).
//! - Extracts `chatgpt_account_id` from the JWT access token for multi-account support.

use crate::utils::oauth::device_code::{
    poll_oauth_device_code_flow, DeviceCodePollOptions, DeviceCodePollResult,
};
use crate::utils::oauth::oauth_page::{oauth_error_html, oauth_success_html};
use crate::utils::oauth::pkce::generate_pkce;
use crate::utils::oauth::types::{
    OAuthAuthInfo, OAuthCredentials, OAuthDeviceCodeInfo, OAuthError, OAuthLoginCallbacks,
    OAuthPrompt, OAuthProviderId, OAuthProviderInterface, OAuthSelectOption, OAuthSelectPrompt,
};
use crate::utils::provider_env::get_provider_env_value;
use base64::Engine as _;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTH_BASE_URL: &str = "https://auth.openai.com";
const AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const DEVICE_USER_CODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";
const DEVICE_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const DEVICE_VERIFICATION_URI: &str = "https://auth.openai.com/codex/device";
const DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const DEVICE_CODE_TIMEOUT_SECONDS: u64 = 15 * 60; // 15 minutes
const SCOPE: &str = "openid profile email offline_access";
const JWT_CLAIM_PATH: &str = "https://api.openai.com/auth";

pub const OPENAI_CODEX_BROWSER_LOGIN_METHOD: &str = "browser";
pub const OPENAI_CODEX_DEVICE_CODE_LOGIN_METHOD: &str = "device_code";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a random hex string for OAuth state parameter.
fn create_state() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("getrandom must never fail");
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode a JWT and extract the payload as parsed JSON.
fn decode_jwt(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload_b64 = parts[1]?;
    // base64url decode (no padding)
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .ok()?;
    serde_json::from_slice(&payload_bytes).ok()
}

/// Extract the `chatgpt_account_id` from the JWT access token.
fn get_account_id(access_token: &str) -> Option<String> {
    let payload = decode_jwt(access_token)?;
    payload
        .get(JWT_CLAIM_PATH)?
        .get("chatgpt_account_id")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn get_callback_host() -> String {
    get_provider_env_value("PI_OAUTH_CALLBACK_HOST", None).unwrap_or_else(|| "127.0.0.1".to_string())
}

/// Parse authorization code input that could be a redirect URL, `code#state`,
/// `code=...&state=...`, or bare code.
fn parse_authorization_input(input: &str) -> (Option<String>, Option<String>) {
    let value = input.trim();
    if value.is_empty() {
        return (None, None);
    }

    // Try parsing as URL
    if let Ok(url) = url::Url::parse(value) {
        let code = url
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.to_string());
        let state = url
            .query_pairs()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.to_string());
        return (code, state);
    }

    // Try `code#state` format
    if value.contains('#') {
        let mut parts = value.splitn(2, '#');
        let code = parts.next().map(|s| s.to_string());
        let state = parts.next().map(|s| s.to_string());
        return (code, state);
    }

    // Try `code=...` format
    if value.contains("code=") {
        // Simple parse: extract code=value and state=value
        let code = value
            .split('&')
            .find(|p| p.starts_with("code="))
            .and_then(|p| p.strip_prefix("code="))
            .map(|s| s.to_string());
        let state = value
            .split('&')
            .find(|p| p.starts_with("state="))
            .and_then(|p| p.strip_prefix("state="))
            .map(|s| s.to_string());
        return (code, state);
    }

    // Bare code
    (Some(value.to_string()), None)
}

// ---------------------------------------------------------------------------
// Token helpers
// ---------------------------------------------------------------------------

struct OAuthToken {
    access: String,
    refresh: String,
    expires: u64,
}

/// Build credentials from an OAuth token, extracting the account ID.
fn credentials_from_token(token: &OAuthToken) -> Result<OAuthCredentials, OAuthError> {
    let account_id = get_account_id(&token.access).ok_or_else(|| {
        OAuthError::Failed("Failed to extract accountId from token".to_string())
    })?;

    let mut creds = OAuthCredentials::new(token.access.clone(), token.refresh.clone(), token.expires);
    creds
        .extra
        .insert("accountId".to_string(), serde_json::Value::String(account_id));
    Ok(creds)
}

async fn read_token_response(
    response: reqwest::Response,
    operation: &str,
) -> Result<OAuthToken, OAuthError> {
    let status = response.status();
    let text = response.text().await.map_err(|e| {
        OAuthError::Failed(format!(
            "OpenAI Codex token {} failed: {}",
            operation, e
        ))
    })?;

    if !status.is_success() {
        return Err(OAuthError::Failed(format!(
            "OpenAI Codex token {} failed ({}): {}",
            operation,
            status.as_u16(),
            text
        )));
    }

    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        OAuthError::Failed(format!(
            "OpenAI Codex token {} response missing fields: {}",
            operation, e
        ))
    })?;

    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            OAuthError::Failed(format!(
                "OpenAI Codex token {} response missing fields: {}",
                operation, text
            ))
        })?;
    let refresh_token = json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            OAuthError::Failed(format!(
                "OpenAI Codex token {} response missing fields: {}",
                operation, text
            ))
        })?;
    let expires_in = json
        .get("expires_in")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| {
            OAuthError::Failed(format!(
                "OpenAI Codex token {} response missing fields: {}",
                operation, text
            ))
        })? as u64;

    Ok(OAuthToken {
        access: access_token.to_string(),
        refresh: refresh_token.to_string(),
        expires: (chrono::Utc::now().timestamp_millis() as u64) + expires_in * 1000,
    })
}

async fn exchange_authorization_code(
    client: &reqwest::Client,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<OAuthToken, OAuthError> {
    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code),
        ("code_verifier", verifier),
        ("redirect_uri", redirect_uri),
    ];
    let body = serde_urlencoded::to_string(params)
        .map_err(|e| OAuthError::Failed(format!("Failed to encode token request: {}", e)))?;

    let response = client
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?;

    read_token_response(response, "exchange").await
}

async fn exchange_authorization_code_for_credentials(
    client: &reqwest::Client,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<OAuthCredentials, OAuthError> {
    let token = exchange_authorization_code(client, code, verifier, redirect_uri).await?;
    credentials_from_token(&token)
}

async fn refresh_access_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Result<OAuthToken, OAuthError> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", CLIENT_ID),
    ];
    let body = serde_urlencoded::to_string(params)
        .map_err(|e| OAuthError::Failed(format!("Failed to encode refresh request: {}", e)))?;

    let response = client
        .post(TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| OAuthError::Failed(format!("OpenAI Codex token refresh error: {}", e)))?;

    read_token_response(response, "refresh").await
}

// ---------------------------------------------------------------------------
// Device code flow
// ---------------------------------------------------------------------------

struct DeviceAuthInfo {
    device_auth_id: String,
    user_code: String,
    interval_seconds: u64,
}

struct DeviceTokenSuccess {
    authorization_code: String,
    code_verifier: String,
}

async fn start_openai_codex_device_auth(
    client: &reqwest::Client,
) -> Result<DeviceAuthInfo, OAuthError> {
    let body = serde_json::json!({"client_id": CLIENT_ID}).to_string();

    let response = client
        .post(DEVICE_USER_CODE_URL)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if status.as_u16() == 404 {
            return Err(OAuthError::Failed(
                "OpenAI Codex device code login is not enabled for this server. Use browser login or verify the server URL.".to_string(),
            ));
        }
        return Err(OAuthError::Failed(format!(
            "OpenAI Codex device code request failed with status {}{}",
            status.as_u16(),
            if text.is_empty() {
                String::new()
            } else {
                format!(": {}", text)
            }
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| OAuthError::Failed(format!("Failed to parse device code response: {}", e)))?;

    let device_auth_id = json
        .get("device_auth_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            OAuthError::Failed(format!(
                "Invalid OpenAI Codex device code response: {}",
                json
            ))
        })?
        .to_string();
    let user_code = json
        .get("user_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            OAuthError::Failed(format!(
                "Invalid OpenAI Codex device code response: {}",
                json
            ))
        })?
        .to_string();
    let interval_seconds = json
        .get("interval")
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                s.trim().parse::<u64>().ok()
            } else {
                v.as_u64()
            }
        })
        .filter(|&i| i > 0)
        .ok_or_else(|| {
            OAuthError::Failed(format!(
                "Invalid OpenAI Codex device code response: {}",
                json
            ))
        })?;

    Ok(DeviceAuthInfo {
        device_auth_id,
        user_code,
        interval_seconds,
    })
}

async fn poll_openai_codex_device_auth(
    client: &reqwest::Client,
    device: &DeviceAuthInfo,
) -> Result<DeviceTokenSuccess, OAuthError> {
    let device_auth_id = device.device_auth_id.clone();
    let user_code = device.user_code.clone();
    let poll_client = client.clone();

    poll_oauth_device_code_flow(DeviceCodePollOptions {
        interval_seconds: Some(device.interval_seconds),
        expires_in_seconds: Some(DEVICE_CODE_TIMEOUT_SECONDS),
        aborted: None,
        poll: move || {
            let client = poll_client.clone();
            let device_auth_id = device_auth_id.clone();
            let user_code = user_code.clone();
            async move {
                let body = serde_json::json!({
                    "device_auth_id": device_auth_id,
                    "user_code": user_code,
                })
                .to_string();

                let response = match client
                    .post(DEVICE_TOKEN_URL)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        return DeviceCodePollResult::Failed(format!("HTTP error: {}", e));
                    }
                };

                let status = response.status();

                if status.is_success() {
                    let text = match response.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            return DeviceCodePollResult::Failed(format!(
                                "Failed to read response: {}",
                                e
                            ));
                        }
                    };
                    let json: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(e) => {
                            return DeviceCodePollResult::Failed(format!(
                                "Invalid JSON: {} ({})",
                                e, text
                            ));
                        }
                    };
                    let authorization_code =
                        match json.get("authorization_code").and_then(|v| v.as_str()) {
                            Some(c) => c.to_string(),
                            None => {
                                return DeviceCodePollResult::Failed(format!(
                                    "Invalid OpenAI Codex device auth token response: {}",
                                    json
                                ));
                            }
                        };
                    let code_verifier =
                        match json.get("code_verifier").and_then(|v| v.as_str()) {
                            Some(v) => v.to_string(),
                            None => {
                                return DeviceCodePollResult::Failed(format!(
                                    "Invalid OpenAI Codex device auth token response: {}",
                                    json
                                ));
                            }
                        };
                    return DeviceCodePollResult::Complete(DeviceTokenSuccess {
                        authorization_code,
                        code_verifier,
                    });
                }

                // Not success — check for expected pending states
                if status.as_u16() == 403 || status.as_u16() == 404 {
                    return DeviceCodePollResult::Pending;
                }

                let text = response.text().await.unwrap_or_default();
                let error_code = serde_json::from_str::<serde_json::Value>(&text)
                    .ok()
                    .and_then(|v| {
                        v.get("error").and_then(|e| {
                            if let Some(obj) = e.as_object() {
                                obj.get("code").and_then(|c| c.as_str()).map(|s| s.to_string())
                            } else {
                                e.as_str().map(|s| s.to_string())
                            }
                        })
                    });

                match error_code.as_deref() {
                    Some("deviceauth_authorization_pending") => {
                        return DeviceCodePollResult::Pending;
                    }
                    Some("slow_down") => {
                        return DeviceCodePollResult::SlowDown;
                    }
                    _ => {
                        return DeviceCodePollResult::Failed(format!(
                            "OpenAI Codex device auth failed with status {}{}",
                            status.as_u16(),
                            if text.is_empty() {
                                String::new()
                            } else {
                                format!(": {}", text)
                            }
                        ));
                    }
                }
            }
        },
    })
    .await
}

/// Login with OpenAI Codex OAuth using the device-code flow.
async fn login_openai_codex_device_code(
    client: &reqwest::Client,
    callbacks: &OAuthLoginCallbacks,
) -> Result<OAuthCredentials, OAuthError> {
    let device = start_openai_codex_device_auth(client).await?;

    (callbacks.on_device_code)(OAuthDeviceCodeInfo {
        user_code: device.user_code.clone(),
        verification_uri: DEVICE_VERIFICATION_URI.to_string(),
        interval_seconds: Some(device.interval_seconds),
        expires_in_seconds: Some(DEVICE_CODE_TIMEOUT_SECONDS),
    })
    .await;

    let code = poll_openai_codex_device_auth(client, &device).await?;

    exchange_authorization_code_for_credentials(
        client,
        &code.authorization_code,
        &code.code_verifier,
        DEVICE_REDIRECT_URI,
    )
    .await
}

// ---------------------------------------------------------------------------
// Browser login (local HTTP server)
// ---------------------------------------------------------------------------

fn create_authorization_url(originator: &str) -> (String, String, String) {
    let pkce = generate_pkce();
    let state = create_state();

    let mut url = url::Url::parse(AUTHORIZE_URL).expect("AUTHORIZE_URL must be valid");
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("response_type", "code");
        query.append_pair("client_id", CLIENT_ID);
        query.append_pair("redirect_uri", REDIRECT_URI);
        query.append_pair("scope", SCOPE);
        query.append_pair("code_challenge", &pkce.challenge);
        query.append_pair("code_challenge_method", "S256");
        query.append_pair("state", &state);
        query.append_pair("id_token_add_organizations", "true");
        query.append_pair("codex_cli_simplified_flow", "true");
        query.append_pair("originator", originator);
    }

    (pkce.verifier, state, url.to_string())
}

struct OAuthServer {
    cancel_tx: tokio::sync::oneshot::Sender<()>,
    join_handle: tokio::task::JoinHandle<Result<Option<String>, OAuthError>>,
}

/// Start a local HTTP server to receive the OAuth callback.
async fn start_local_oauth_server(state: String) -> Result<OAuthServer, OAuthError> {
    let host = get_callback_host();
    let addr: SocketAddr = format!("{}:1455", host)
        .parse()
        .map_err(|e| OAuthError::Failed(format!("Failed to parse bind address: {}", e)))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| OAuthError::Failed(format!("Failed to bind OAuth server: {}", e)))?;

    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();

    let join_handle = tokio::spawn(async move {
        tokio::select! {
            result = oauth_server_loop(listener, state) => result,
            _ = cancel_rx => Ok(None),
        }
    });

    Ok(OAuthServer {
        cancel_tx,
        join_handle,
    })
}

async fn oauth_server_loop(
    listener: TcpListener,
    expected_state: String,
) -> Result<Option<String>, OAuthError> {
    loop {
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|e| OAuthError::Failed(format!("OAuth server accept error: {}", e)))?;

        // Read the HTTP request
        let mut buf = [0u8; 4096];
        let n = socket
            .read(&mut buf)
            .await
            .map_err(|e| OAuthError::Failed(format!("OAuth server read error: {}", e)))?;

        let request = String::from_utf8_lossy(&buf[..n]);
        let first_line = request.lines().next().unwrap_or("");

        // Parse request line: GET /auth/callback?code=...&state=... HTTP/1.1
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 {
            let response = oauth_error_html("Callback route not found.");
            let status_line = "HTTP/1.1 404 Not Found\r\n";
            let headers = "Content-Type: text/html; charset=utf-8\r\n";
            let _ = socket
                .write_all(format!("{}{}\r\n{}", status_line, headers, response).as_bytes())
                .await;
            continue;
        }

        let path_and_query = parts[1];

        // Extract path
        let path = path_and_query.split('?').next().unwrap_or("");

        if path != "/auth/callback" {
            let response = oauth_error_html("Callback route not found.");
            let status_line = "HTTP/1.1 404 Not Found\r\n";
            let headers = "Content-Type: text/html; charset=utf-8\r\n";
            let _ = socket
                .write_all(format!("{}{}\r\n{}", status_line, headers, response).as_bytes())
                .await;
            continue;
        }

        // Parse query string
        let query_str = path_and_query.split('?').nth(1).unwrap_or("");
        let query_pairs: std::collections::HashMap<String, String> = query_str
            .split('&')
            .filter(|p| !p.is_empty())
            .filter_map(|p| {
                let mut kv = p.splitn(2, '=');
                let key = urlencoding::decode(kv.next()?).ok()?;
                let val = urlencoding::decode(kv.next().unwrap_or("")).ok()?;
                Some((key.to_string(), val.to_string()))
            })
            .collect();

        // Validate state
        let state = query_pairs.get("state").cloned();
        if state.as_deref() != Some(&expected_state) {
            let response = oauth_error_html("State mismatch.");
            let status_line = "HTTP/1.1 400 Bad Request\r\n";
            let headers = "Content-Type: text/html; charset=utf-8\r\n";
            let _ = socket
                .write_all(format!("{}{}\r\n{}", status_line, headers, response).as_bytes())
                .await;
            continue;
        }

        let code = query_pairs.get("code").cloned();

        if let Some(ref code) = code {
            let response =
                oauth_success_html("OpenAI authentication completed. You can close this window.");
            let status_line = "HTTP/1.1 200 OK\r\n";
            let headers = "Content-Type: text/html; charset=utf-8\r\n";
            let _ = socket
                .write_all(format!("{}{}\r\n{}", status_line, headers, response).as_bytes())
                .await;
            return Ok(Some(code.clone()));
        }

        let response = oauth_error_html("Missing authorization code.");
        let status_line = "HTTP/1.1 400 Bad Request\r\n";
        let headers = "Content-Type: text/html; charset=utf-8\r\n";
        let _ = socket
            .write_all(format!("{}{}\r\n{}", status_line, headers, response).as_bytes())
            .await;
    }
}

// ---------------------------------------------------------------------------
// Browser login flow
// ---------------------------------------------------------------------------

/// Login with OpenAI Codex OAuth (browser flow).
async fn login_openai_codex_browser(
    client: &reqwest::Client,
    callbacks: &OAuthLoginCallbacks,
    originator: &str,
) -> Result<OAuthCredentials, OAuthError> {
    let (verifier, state, url) = create_authorization_url(originator);
    let server = start_local_oauth_server(state.clone()).await?;

    (callbacks.on_auth)(OAuthAuthInfo {
        url: url.clone(),
        instructions: Some(
            "A browser window should open. Complete login to finish.".to_string(),
        ),
    })
    .await;

    let code = if let Some(ref manual_input) = callbacks.on_manual_code_input {
        // Race between browser callback and manual input
        let manual_future = manual_input();
        let server_future = &server.join_handle;

        tokio::select! {
            server_result = server_future => {
                match server_result {
                    Ok(Ok(Some(code))) => code,
                    Ok(Ok(None)) => {
                        // Server was cancelled, try manual
                        let input = manual_future.await;
                        parse_authorization_input(&input).0
                            .ok_or_else(|| OAuthError::Failed("Missing authorization code".to_string()))?
                    }
                    Ok(Err(e)) => return Err(e),
                    Err(e) => return Err(OAuthError::Failed(format!("OAuth server error: {}", e))),
                }
            }
            input = manual_future => {
                // Manual input came first
                let _ = server.cancel_tx.send(());
                let parsed = parse_authorization_input(&input);
                if let (Some(input_state), Some(expected_state)) = (&parsed.1, Some(&state)) {
                    if input_state != expected_state {
                        return Err(OAuthError::Failed("State mismatch".to_string()));
                    }
                }
                parsed.0.ok_or_else(|| OAuthError::Failed("Missing authorization code".to_string()))?
            }
        }
    } else {
        // Wait for browser callback only, fallback to prompt
        let result = match server.join_handle.await {
            Ok(Ok(Some(code))) => Some(code),
            Ok(Ok(None)) => None,
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(OAuthError::Failed(format!("OAuth server error: {}", e))),
        };

        if let Some(c) = result {
            c
        } else {
            // Fallback to prompt
            let input = (callbacks.on_prompt)(OAuthPrompt {
                message: "Paste the authorization code (or full redirect URL):".to_string(),
                placeholder: None,
                allow_empty: false,
            })
            .await;

            let (code_opt, parsed_state) = parse_authorization_input(&input);
            if let Some(ps) = parsed_state {
                if ps != state {
                    return Err(OAuthError::Failed("State mismatch".to_string()));
                }
            }
            code_opt.ok_or_else(|| OAuthError::Failed("Missing authorization code".to_string()))?
        }
    };

    let _ = server.cancel_tx.send(());
    exchange_authorization_code_for_credentials(client, &code, &verifier, REDIRECT_URI).await
}

// ---------------------------------------------------------------------------
// Refresh token
// ---------------------------------------------------------------------------

/// Refresh OpenAI Codex OAuth token.
async fn refresh_openai_codex_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Result<OAuthCredentials, OAuthError> {
    let token = refresh_access_token(client, refresh_token).await?;
    credentials_from_token(&token)
}

// ---------------------------------------------------------------------------
// OAuthProviderInterface implementation
// ---------------------------------------------------------------------------

/// OpenAI Codex OAuth provider.
pub struct OpenAICodexOAuthProvider;

impl OpenAICodexOAuthProvider {
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl OAuthProviderInterface for OpenAICodexOAuthProvider {
    fn id(&self) -> OAuthProviderId {
        "openai-codex".to_string()
    }

    fn name(&self) -> &str {
        "ChatGPT Plus/Pro (Codex Subscription)"
    }

    fn uses_callback_server(&self) -> bool {
        true
    }

    async fn login(
        &self,
        callbacks: OAuthLoginCallbacks,
    ) -> Result<OAuthCredentials, OAuthError> {
        let client = reqwest::Client::new();

        let login_method = (callbacks.on_select)(OAuthSelectPrompt {
            message: "Select OpenAI Codex login method:".to_string(),
            options: vec![
                OAuthSelectOption {
                    id: OPENAI_CODEX_BROWSER_LOGIN_METHOD.to_string(),
                    label: "Browser login (default)".to_string(),
                },
                OAuthSelectOption {
                    id: OPENAI_CODEX_DEVICE_CODE_LOGIN_METHOD.to_string(),
                    label: "Device code login (headless)".to_string(),
                },
            ],
        })
        .await
        .ok_or_else(|| OAuthError::Failed("Login cancelled".to_string()))?;

        match login_method.as_str() {
            OPENAI_CODEX_DEVICE_CODE_LOGIN_METHOD => {
                login_openai_codex_device_code(&client, &callbacks).await
            }
            OPENAI_CODEX_BROWSER_LOGIN_METHOD => {
                login_openai_codex_browser(&client, &callbacks, "hamr").await
            }
            other => Err(OAuthError::Failed(format!(
                "Unknown OpenAI Codex login method: {}",
                other
            ))),
        }
    }

    async fn refresh_token(
        &self,
        credentials: &OAuthCredentials,
    ) -> Result<OAuthCredentials, OAuthError> {
        let client = reqwest::Client::new();
        refresh_openai_codex_token(&client, &credentials.refresh).await
    }

    fn get_api_key(&self, credentials: &OAuthCredentials) -> String {
        credentials.access.clone()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authorization_input_code_only() {
        let (code, state) = parse_authorization_input("abc123");
        assert_eq!(code.as_deref(), Some("abc123"));
        assert_eq!(state, None);
    }

    #[test]
    fn test_parse_authorization_input_hash_format() {
        let (code, state) = parse_authorization_input("abc123#state456");
        assert_eq!(code.as_deref(), Some("abc123"));
        assert_eq!(state.as_deref(), Some("state456"));
    }

    #[test]
    fn test_parse_authorization_input_query_string_format() {
        let (code, state) =
            parse_authorization_input("code=abc123&state=state456&other=value");
        assert_eq!(code.as_deref(), Some("abc123"));
        assert_eq!(state.as_deref(), Some("state456"));
    }

    #[test]
    fn test_parse_authorization_input_url() {
        let (code, state) = parse_authorization_input(
            "http://localhost:1455/auth/callback?code=abc123&state=state456",
        );
        assert_eq!(code.as_deref(), Some("abc123"));
        assert_eq!(state.as_deref(), Some("state456"));
    }

    #[test]
    fn test_parse_authorization_input_empty() {
        let (code, state) = parse_authorization_input("");
        assert_eq!(code, None);
        assert_eq!(state, None);
    }

    #[test]
    fn test_create_state_is_hex() {
        let state = create_state();
        assert_eq!(state.len(), 64); // 32 bytes → 64 hex chars
        assert!(state.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_create_state_unique() {
        let state1 = create_state();
        let state2 = create_state();
        assert_ne!(state1, state2);
    }

    #[test]
    fn test_provider_id_and_name() {
        let provider = OpenAICodexOAuthProvider::new();
        assert_eq!(provider.id(), "openai-codex");
        assert_eq!(provider.name(), "ChatGPT Plus/Pro (Codex Subscription)");
        assert!(provider.uses_callback_server());
    }

    #[test]
    fn test_get_api_key() {
        let provider = OpenAICodexOAuthProvider::new();
        let creds = OAuthCredentials::new(
            "refresh-token".to_string(),
            "access-token".to_string(),
            1234567890,
        );
        assert_eq!(provider.get_api_key(&creds), "access-token");
    }
}

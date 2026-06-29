//! Port of `packages/ai/src/utils/oauth/github-copilot.ts`
//!
//! GitHub Copilot device-code OAuth flow.
//!
//! Flow:
//! 1. Prompt for optional enterprise domain (blank = github.com).
//! 2. POST to /login/device/code to get device_code + user_code.
//! 3. Poll /login/oauth/access_token for the GitHub access token.
//! 4. Exchange GitHub access token for Copilot token via /copilot_internal/v2/token.
//! 5. Enable all GitHub Copilot models via /models/{id}/policy.

use crate::utils::oauth::device_code::{
    poll_oauth_device_code_flow, DeviceCodePollOptions, DeviceCodePollResult,
};
use crate::utils::oauth::types::{
    OAuthAuthInfo, OAuthCredentials, OAuthDeviceCodeInfo, OAuthError, OAuthLoginCallbacks,
    OAuthPrompt, OAuthProviderId, OAuthProviderInterface, OAuthSelectOption, OAuthSelectPrompt,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Base64-decoded: "Iv1.b507a08c87ecfe98"
const CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";

const COPILOT_HEADERS: &[(&str, &str)] = &[
    ("User-Agent", "GitHubCopilotChat/0.35.0"),
    ("Editor-Version", "vscode/1.107.0"),
    ("Editor-Plugin-Version", "copilot-chat/0.35.0"),
    ("Copilot-Integration-Id", "vscode-chat"),
];

const DEVICE_CODE_INTERVAL_SECONDS: u64 = 5;
const DEVICE_CODE_EXPIRES_SECONDS: u64 = 900; // 15 minutes

// ---------------------------------------------------------------------------
// Helper: normalize domain
// ---------------------------------------------------------------------------

/// Extract a valid hostname from user input. Returns `None` if the input is
/// not a recognizable domain or URL.
pub fn normalize_domain(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let url_str = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    };
    url::Url::parse(&url_str)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// Parse the `proxy-ep` field from a Copilot token and convert to API base URL.
///
/// Token format: `tid=...;exp=...;proxy-ep=proxy.individual.githubcopilot.com;...`
/// Returns API URL like `https://api.individual.githubcopilot.com`.
fn get_base_url_from_token(token: &str) -> Option<String> {
    let proxy_ep = token
        .split(';')
        .find_map(|part| {
            let trimmed = part.trim();
            trimmed
                .strip_prefix("proxy-ep=")
                .map(|v| v.to_string())
        })?;
    let api_host = proxy_ep.replacen("proxy.", "api.", 1);
    Some(format!("https://{}", api_host))
}

/// Compute the GitHub Copilot API base URL.
///
/// Priority:
/// 1. Extract from token's `proxy-ep` field.
/// 2. Enterprise domain → `https://copilot-api.{domain}`.
/// 3. Fallback to `https://api.individual.githubcopilot.com`.
pub fn get_git_hub_copilot_base_url(
    token: Option<&str>,
    enterprise_domain: Option<&str>,
) -> String {
    if let Some(t) = token {
        if let Some(url) = get_base_url_from_token(t) {
            return url;
        }
    }
    if let Some(domain) = enterprise_domain {
        return format!("https://copilot-api.{}", domain);
    }
    "https://api.individual.githubcopilot.com".to_string()
}

// ---------------------------------------------------------------------------
// URL builders
// ---------------------------------------------------------------------------

struct Urls {
    device_code_url: String,
    access_token_url: String,
    copilot_token_url: String,
}

fn get_urls(domain: &str) -> Urls {
    Urls {
        device_code_url: format!("https://{}/login/device/code", domain),
        access_token_url: format!("https://{}/login/oauth/access_token", domain),
        copilot_token_url: format!("https://api.{}/copilot_internal/v2/token", domain),
    }
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

async fn fetch_json(url: &str, client: &reqwest::Client, request: reqwest::Request) -> Result<serde_json::Value, OAuthError> {
    let response = client.execute(request).await?;
    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        return Err(OAuthError::Failed(format!(
            "{} {}: {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown"),
            text
        )));
    }

    serde_json::from_str(&text)
        .map_err(|e| OAuthError::Failed(format!("Invalid JSON response: {}", e)))
}

// ---------------------------------------------------------------------------
// Device code flow
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: Option<u64>,
    expires_in: u64,
}

async fn start_device_flow(
    client: &reqwest::Client,
    domain: &str,
) -> Result<DeviceCodeResponse, OAuthError> {
    let urls = get_urls(domain);

    let params = [("client_id", CLIENT_ID), ("scope", "read:user")];
    let body = serde_urlencoded::to_string(params)
        .map_err(|e| OAuthError::Failed(format!("Failed to encode form data: {}", e)))?;

    let request = client
        .post(&urls.device_code_url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", "GitHubCopilotChat/0.35.0")
        .body(body)
        .build()?;

    let data = fetch_json(&urls.device_code_url, client, request).await?;

    let device_code = data
        .get("device_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OAuthError::Failed("Invalid device code response".to_string()))?
        .to_string();
    let user_code = data
        .get("user_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OAuthError::Failed("Invalid device code response".to_string()))?
        .to_string();
    let verification_uri = data
        .get("verification_uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OAuthError::Failed("Invalid device code response".to_string()))?
        .to_string();
    let interval = data
        .get("interval")
        .and_then(|v| v.as_u64());
    let expires_in = data
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| OAuthError::Failed("Invalid device code response".to_string()))?;

    // Validate the verification URI is safe (https or http only)
    let parsed = url::Url::parse(&verification_uri)
        .map_err(|_| OAuthError::Failed("Untrusted verification_uri in device code response".to_string()))?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err(OAuthError::Failed(
            "Untrusted verification_uri in device code response".to_string(),
        ));
    }

    Ok(DeviceCodeResponse {
        device_code,
        user_code,
        verification_uri: parsed.to_string(),
        interval,
        expires_in,
    })
}

#[derive(Debug)]
struct DeviceTokenSuccess {
    access_token: String,
}

async fn poll_for_git_hub_access_token(
    client: &reqwest::Client,
    domain: &str,
    device: &DeviceCodeResponse,
    aborted: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<String, OAuthError> {
    let urls = get_urls(domain);

    let poll_client = client.clone();
    let device_code = device.device_code.clone();
    let access_token_url = urls.access_token_url.clone();

    poll_oauth_device_code_flow(DeviceCodePollOptions {
        interval_seconds: device.interval.or(Some(DEVICE_CODE_INTERVAL_SECONDS)),
        expires_in_seconds: Some(device.expires_in),
        aborted,
        poll: move || {
            let client = poll_client.clone();
            let url = access_token_url.clone();
            let device_code = device_code.clone();
            async move {
                let params = [
                    ("client_id", CLIENT_ID),
                    ("device_code", &device_code),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ];
                let body = match serde_urlencoded::to_string(params) {
                    Ok(b) => b,
                    Err(e) => {
                        return DeviceCodePollResult::Failed(format!(
                            "Failed to encode form data: {}",
                            e
                        ))
                    }
                };

                let request = match client
                    .post(&url)
                    .header("Accept", "application/json")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .header("User-Agent", "GitHubCopilotChat/0.35.0")
                    .body(body)
                    .build()
                {
                    Ok(r) => r,
                    Err(e) => {
                        return DeviceCodePollResult::Failed(format!("Failed to build request: {}", e))
                    }
                };

                match client.execute(request).await {
                    Ok(response) => {
                        let status = response.status();
                        let text = match response.text().await {
                            Ok(t) => t,
                            Err(e) => {
                                return DeviceCodePollResult::Failed(format!(
                                    "Failed to read response: {}",
                                    e
                                ))
                            }
                        };
                        let data: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(_) => {
                                return DeviceCodePollResult::Failed(format!(
                                    "Invalid JSON: {}",
                                    text
                                ))
                            }
                        };

                        if status.is_success() {
                            if let Some(access_token) =
                                data.get("access_token").and_then(|v| v.as_str())
                            {
                                return DeviceCodePollResult::Complete(
                                    access_token.to_string(),
                                );
                            }
                        }

                        if let Some(error) = data.get("error").and_then(|v| v.as_str()) {
                            match error {
                                "authorization_pending" => {
                                    return DeviceCodePollResult::Pending;
                                }
                                "slow_down" => {
                                    return DeviceCodePollResult::SlowDown;
                                }
                                other => {
                                    let description = data
                                        .get("error_description")
                                        .and_then(|v| v.as_str())
                                        .map(|d| format!(": {}", d))
                                        .unwrap_or_default();
                                    return DeviceCodePollResult::Failed(format!(
                                        "Device flow failed: {}{}",
                                        other, description
                                    ));
                                }
                            }
                        }

                        DeviceCodePollResult::Failed(
                            "Invalid device token response".to_string(),
                        )
                    }
                    Err(e) => DeviceCodePollResult::Failed(format!("HTTP error: {}", e)),
                }
            }
        },
    })
    .await
}

// ---------------------------------------------------------------------------
// Copilot token exchange & refresh
// ---------------------------------------------------------------------------

/// Exchange a GitHub access token for a Copilot token.
async fn refresh_git_hub_copilot_token(
    client: &reqwest::Client,
    github_access_token: &str,
    enterprise_domain: Option<&str>,
) -> Result<OAuthCredentials, OAuthError> {
    let domain = enterprise_domain.unwrap_or("github.com");
    let urls = get_urls(domain);

    let mut request_builder = client
        .get(&urls.copilot_token_url)
        .header("Accept", "application/json")
        .header("Authorization", format!("Bearer {}", github_access_token));

    for &(key, value) in COPILOT_HEADERS {
        request_builder = request_builder.header(key, value);
    }

    let request = request_builder.build()?;
    let data = fetch_json(&urls.copilot_token_url, client, request).await?;

    let token = data
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OAuthError::Failed("Invalid Copilot token response fields".to_string()))?
        .to_string();
    let expires_at = data
        .get("expires_at")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| OAuthError::Failed("Invalid Copilot token response fields".to_string()))?;

    // Subtract 5 minutes for a safety margin
    let expires = expires_at.saturating_mul(1000).saturating_sub(5 * 60 * 1000);

    let mut creds = OAuthCredentials::new(
        github_access_token.to_string(),
        token,
        expires,
    );

    if let Some(ed) = enterprise_domain {
        creds
            .extra
            .insert("enterpriseUrl".to_string(), serde_json::Value::String(ed.to_string()));
    }

    Ok(creds)
}

// ---------------------------------------------------------------------------
// Model policy enablement
// ---------------------------------------------------------------------------

/// Enable a single model for the user's GitHub Copilot account.
/// Required for some models (Claude, Grok) before they can be used.
async fn enable_git_hub_copilot_model(
    client: &reqwest::Client,
    token: &str,
    model_id: &str,
    enterprise_domain: Option<&str>,
) -> bool {
    let base_url = get_git_hub_copilot_base_url(Some(token), enterprise_domain);
    let url = format!("{}/models/{}/policy", base_url, model_id);

    let body = serde_json::json!({"state": "enabled"}).to_string();

    let mut request_builder = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .header("openai-intent", "chat-policy")
        .header("x-interaction-type", "chat-policy");

    for &(key, value) in COPILOT_HEADERS {
        request_builder = request_builder.header(key, value);
    }

    let request = match request_builder.body(body).build() {
        Ok(r) => r,
        Err(_) => return false,
    };

    match client.execute(request).await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// List of GitHub Copilot model IDs that may need policy acceptance.
const COPILOT_MODEL_IDS: &[&str] = &[
    "claude-3.5-sonnet",
    "claude-3.7-sonnet",
    "claude-sonnet-4",
    "claude-sonnet-4-5",
    "claude-opus-4",
    "claude-opus-4-1",
    "gpt-4o",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "grok-3",
    "grok-3-mini",
    "o3",
    "o3-mini",
    "o4-mini",
];

/// Enable all known GitHub Copilot models that may require policy acceptance.
async fn enable_all_git_hub_copilot_models(
    client: &reqwest::Client,
    token: &str,
    enterprise_domain: Option<&str>,
    on_progress: Option<
        std::sync::Arc<
            dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
                + Send
                + Sync,
        >,
    >,
) {
    let futures: Vec<_> = COPILOT_MODEL_IDS
        .iter()
        .map(|model_id| {
            let client = client.clone();
            let token = token.to_string();
            let domain = enterprise_domain.map(|s| s.to_string());
            let model_id = model_id.to_string();
            async move {
                let success = enable_git_hub_copilot_model(
                    &client,
                    &token,
                    &model_id,
                    domain.as_deref(),
                )
                .await;
                (model_id, success)
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    if let Some(ref progress_cb) = on_progress {
        for (model_id, _success) in results {
            progress_cb(format!("Enabled model: {}", model_id)).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

/// Login with GitHub Copilot OAuth (device code flow).
pub async fn login_git_hub_copilot(
    callbacks: &OAuthLoginCallbacks,
) -> Result<OAuthCredentials, OAuthError> {
    let client = reqwest::Client::new();

    // 1. Prompt for enterprise domain
    let input = (callbacks.on_prompt)(OAuthPrompt {
        message: "GitHub Enterprise URL/domain (blank for github.com)".to_string(),
        placeholder: Some("company.ghe.com".to_string()),
        allow_empty: true,
    })
    .await;

    let trimmed = input.trim();
    let enterprise_domain = normalize_domain(input.as_str());
    if !trimmed.is_empty() && enterprise_domain.is_none() {
        return Err(OAuthError::Failed(
            "Invalid GitHub Enterprise URL/domain".to_string(),
        ));
    }
    let domain = enterprise_domain.clone().unwrap_or_else(|| "github.com".to_string());

    // 2. Start device code flow
    let device = start_device_flow(&client, &domain).await?;

    (callbacks.on_device_code)(OAuthDeviceCodeInfo {
        user_code: device.user_code.clone(),
        verification_uri: device.verification_uri.clone(),
        interval_seconds: device.interval,
        expires_in_seconds: Some(device.expires_in),
    })
    .await;

    // 3. Poll for GitHub access token
    let github_access_token =
        poll_for_git_hub_access_token(&client, &domain, &device, None).await?;

    // 4. Exchange for Copilot token
    let credentials = refresh_git_hub_copilot_token(
        &client,
        &github_access_token,
        enterprise_domain.as_deref(),
    )
    .await?;

    // 5. Enable all models
    if let Some(ref progress_cb) = callbacks.on_progress {
        progress_cb("Enabling models...".to_string()).await;
    }
    enable_all_git_hub_copilot_models(
        &client,
        &credentials.access,
        enterprise_domain.as_deref(),
        callbacks.on_progress.clone(),
    )
    .await;

    Ok(credentials)
}

// ---------------------------------------------------------------------------
// OAuthProviderInterface implementation
// ---------------------------------------------------------------------------

/// GitHub Copilot OAuth provider.
pub struct GitHubCopilotOAuthProvider;

impl GitHubCopilotOAuthProvider {
    pub const fn new() -> Self {
        Self
    }

    /// Extract the enterprise domain from credentials' extra fields, if present.
    fn enterprise_domain_from_creds(&self, credentials: &OAuthCredentials) -> Option<String> {
        credentials
            .extra
            .get("enterpriseUrl")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

#[async_trait::async_trait]
impl OAuthProviderInterface for GitHubCopilotOAuthProvider {
    fn id(&self) -> OAuthProviderId {
        "github-copilot".to_string()
    }

    fn name(&self) -> &str {
        "GitHub Copilot"
    }

    async fn login(
        &self,
        callbacks: OAuthLoginCallbacks,
    ) -> Result<OAuthCredentials, OAuthError> {
        login_git_hub_copilot(&callbacks).await
    }

    async fn refresh_token(
        &self,
        credentials: &OAuthCredentials,
    ) -> Result<OAuthCredentials, OAuthError> {
        let client = reqwest::Client::new();
        let enterprise_domain = self.enterprise_domain_from_creds(credentials);
        refresh_git_hub_copilot_token(&client, &credentials.refresh, enterprise_domain.as_deref())
            .await
    }

    fn get_api_key(&self, credentials: &OAuthCredentials) -> String {
        credentials.access.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_domain() {
        assert_eq!(
            normalize_domain("github.com").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            normalize_domain("https://github.com").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            normalize_domain("company.ghe.com").as_deref(),
            Some("company.ghe.com")
        );
        assert_eq!(
            normalize_domain("https://company.ghe.com/extra").as_deref(),
            Some("company.ghe.com")
        );
        assert_eq!(normalize_domain(""), None);
        assert_eq!(normalize_domain("   "), None);
    }

    #[test]
    fn test_get_base_url_from_token() {
        let token = "tid=abc;exp=123;proxy-ep=proxy.individual.githubcopilot.com;sig=xyz";
        assert_eq!(
            get_base_url_from_token(token).as_deref(),
            Some("https://api.individual.githubcopilot.com")
        );
    }

    #[test]
    fn test_get_base_url_from_token_no_proxy_ep() {
        let token = "tid=abc;exp=123";
        assert_eq!(get_base_url_from_token(token), None);
    }

    #[test]
    fn test_get_git_hub_copilot_base_url_default() {
        assert_eq!(
            get_git_hub_copilot_base_url(None, None),
            "https://api.individual.githubcopilot.com"
        );
    }

    #[test]
    fn test_get_git_hub_copilot_base_url_enterprise() {
        assert_eq!(
            get_git_hub_copilot_base_url(None, Some("company.ghe.com")),
            "https://copilot-api.company.ghe.com"
        );
    }

    #[test]
    fn test_get_git_hub_copilot_base_url_from_token() {
        let token = "proxy-ep=proxy.custom.githubcopilot.com";
        assert_eq!(
            get_git_hub_copilot_base_url(Some(token), None),
            "https://api.custom.githubcopilot.com"
        );
    }

    #[test]
    fn test_provider_id_and_name() {
        let provider = GitHubCopilotOAuthProvider::new();
        assert_eq!(provider.id(), "github-copilot");
        assert_eq!(provider.name(), "GitHub Copilot");
    }

    #[test]
    fn test_get_api_key() {
        let provider = GitHubCopilotOAuthProvider::new();
        let creds = OAuthCredentials::new(
            "refresh-token".to_string(),
            "access-token".to_string(),
            1234567890,
        );
        assert_eq!(provider.get_api_key(&creds), "access-token");
    }
}

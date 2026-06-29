//! OAuth provider types — port of `packages/ai/src/utils/oauth/types.ts`.

use std::future::Future;
use std::pin::Pin;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// OAuth credentials (tokens + expiry).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OAuthCredentials {
    pub refresh: String,
    pub access: String,
    /// Expiry timestamp in milliseconds since Unix epoch.
    pub expires: i64,
}

/// OAuth provider identifier (e.g. "anthropic", "github-copilot").
pub type OAuthProviderId = String;

/// Prompt shown to the user (e.g. to paste a code).
#[derive(Debug, Clone)]
pub struct OAuthPrompt {
    pub message: String,
    pub placeholder: Option<String>,
    pub allow_empty: bool,
}

/// Information about the authorization URL to open.
#[derive(Debug, Clone)]
pub struct OAuthAuthInfo {
    pub url: String,
    pub instructions: Option<String>,
}

/// Device code information for device-code flows.
#[derive(Debug, Clone)]
pub struct OAuthDeviceCodeInfo {
    pub user_code: String,
    pub verification_uri: String,
    pub interval_seconds: Option<u64>,
    pub expires_in_seconds: Option<u64>,
}

/// An option in a selection prompt.
#[derive(Debug, Clone)]
pub struct OAuthSelectOption {
    pub id: String,
    pub label: String,
}

/// Selection prompt.
#[derive(Debug, Clone)]
pub struct OAuthSelectPrompt {
    pub message: String,
    pub options: Vec<OAuthSelectOption>,
}

/// OAuth operation error — used by device_code and other flows.
#[derive(Debug, Clone)]
pub enum OAuthError {
    Cancelled,
    Failed(String),
    Timeout,
    SlowDownTimeout,
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "OAuth flow cancelled"),
            Self::Failed(msg) => write!(f, "OAuth flow failed: {msg}"),
            Self::Timeout => write!(f, "OAuth flow timed out"),
            Self::SlowDownTimeout => {
                write!(f, "OAuth flow timed out after repeated slow-down")
            }
        }
    }
}

impl std::error::Error for OAuthError {}

// ---------------------------------------------------------------------------
// Callback trait — the async interface the provider uses to interact with
// the user / host application.
// ---------------------------------------------------------------------------

/// Trait for OAuth login callbacks.
///
/// This mirrors the TS `OAuthLoginCallbacks` interface. Implementors
/// provide the UI hooks (browser open, prompts, progress, etc.).
pub trait OAuthLoginCallbacks: Send + Sync {
    /// Notify the user to open the given URL for authorization.
    fn on_auth(&self, url: &str, instructions: Option<&str>);

    /// Notify about device code flow.
    fn on_device_code(&self, info: &OAuthDeviceCodeInfo);

    /// Prompt the user for input (e.g. paste a code).
    fn on_prompt(
        &self,
        prompt: OAuthPrompt,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + '_>>;

    /// Report progress to the user.
    fn on_progress(&self, message: &str);

    /// Show an interactive selector and return the selected option id,
    /// or None on cancel.
    fn on_select(
        &self,
        prompt: OAuthSelectPrompt,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, String>> + Send + '_>>;
}

// ---------------------------------------------------------------------------
// Provider interface
// ---------------------------------------------------------------------------

/// Interface for OAuth providers.
///
/// Each provider (Anthropic, GitHub Copilot, OpenAI Codex, etc.)
/// implements this trait.
pub trait OAuthProviderInterface: Send + Sync {
    /// Unique provider ID (e.g. "anthropic", "github-copilot").
    fn id(&self) -> OAuthProviderId;

    /// Human-readable provider name.
    fn name(&self) -> String;

    /// Whether this provider uses a local callback server and supports
    /// manual code input.
    fn uses_callback_server(&self) -> bool {
        false
    }

    /// Run the login flow and return credentials to persist.
    fn login(
        &self,
        callbacks: &dyn OAuthLoginCallbacks,
    ) -> Pin<Box<dyn Future<Output = Result<OAuthCredentials, String>> + Send + '_>>;

    /// Refresh expired credentials, return updated credentials.
    fn refresh_token(
        &self,
        credentials: &OAuthCredentials,
    ) -> Pin<Box<dyn Future<Output = Result<OAuthCredentials, String>> + Send + '_>>;

    /// Convert credentials to an API key string for the provider.
    fn get_api_key(&self, credentials: &OAuthCredentials) -> String;
}

/// Information about an available OAuth provider.
#[derive(Debug, Clone)]
pub struct OAuthProviderInfo {
    pub id: OAuthProviderId,
    pub name: String,
    pub available: bool,
}

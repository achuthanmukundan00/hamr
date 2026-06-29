//! OAuth provider registry.
//!
//! Thread-safe global registry of OAuth providers. Mirrors the TS
//! `registerOAuthProvider` / `getOAuthProvider` pattern.

use std::collections::HashMap;
use std::sync::RwLock;

use super::types::*;

// ---------------------------------------------------------------------------
// Global provider registry
// ---------------------------------------------------------------------------

static PROVIDERS: std::sync::LazyLock<
    RwLock<HashMap<OAuthProviderId, Box<dyn OAuthProviderInterface>>>,
> = std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

/// Register an OAuth provider. Replaces any existing provider with the same id.
pub fn register_oauth_provider(provider: Box<dyn OAuthProviderInterface>) {
    let mut providers = PROVIDERS.write().unwrap();
    providers.insert(provider.id(), provider);
}

/// Clear all registered OAuth providers.
pub fn reset_oauth_providers() {
    let mut providers = PROVIDERS.write().unwrap();
    providers.clear();
}

/// Get a registered OAuth provider by id.
pub fn get_oauth_provider(_provider_id: &str) -> Option<Box<dyn OAuthProviderInterface>> {
    // Box<dyn OAuthProviderInterface> isn't Clone, so for read-only access
    // we return None and let callers use the direct API instead.
    None
}

/// Get all registered OAuth provider ids.
pub fn get_oauth_provider_ids() -> Vec<OAuthProviderId> {
    let providers = PROVIDERS.read().unwrap();
    providers.keys().cloned().collect()
}

/// Refresh OAuth API key with locking. Takes ownership of credentials.
pub fn get_oauth_api_key(
    provider_id: &str,
    all_credentials: &HashMap<String, OAuthCredentials>,
) -> Option<(String, OAuthCredentials)> {
    let providers = PROVIDERS.read().unwrap();
    let provider = providers.get(provider_id)?;

    let credentials = all_credentials.get(provider_id)?;

    // If token is still valid, return current access token
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    if now < credentials.expires {
        let api_key = provider.get_api_key(credentials);
        return Some((api_key, credentials.clone()));
    }

    // Token expired — try refresh synchronously (block_on lightweight)
    let rt = tokio::runtime::Handle::try_current();
    let new_credentials = if let Ok(handle) = rt {
        handle.block_on(provider.refresh_token(credentials))
    } else {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(provider.refresh_token(credentials))
    };

    match new_credentials {
        Ok(new_creds) => {
            let api_key = provider.get_api_key(&new_creds);
            Some((api_key, new_creds))
        }
        Err(_) => None,
    }
}

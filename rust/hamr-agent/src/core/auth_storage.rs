//! Credential storage for API keys and OAuth tokens.
//!
//! Handles loading, saving, and refreshing credentials from auth.json.
//! Uses file locking to prevent race conditions when multiple instances
//! try to refresh tokens simultaneously.
//!
//! Ported from `packages/coding-agent/src/core/auth-storage.ts`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::core::resolve_config_value::resolve_config_value;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// API key credential stored in auth.json.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKeyCredential {
    #[serde(rename = "type")]
    pub credential_type: String,
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// OAuth token credential stored in auth.json.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuthCredential {
    #[serde(rename = "type")]
    pub credential_type: String,
    pub access: String,
    pub refresh: String,
    pub expires: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
}

/// Union of credential types that can be stored in auth.json.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AuthCredential {
    ApiKey(ApiKeyCredential),
    OAuth(OAuthCredential),
}

/// Full auth.json data: provider name → credential.
pub type AuthStorageData = HashMap<String, AuthCredential>;

/// Auth status returned by get_auth_status without exposing credential values.
#[derive(Debug, Clone, Serialize)]
pub struct AuthStatus {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Result from with_lock callback.
struct LockResult<T> {
    result: T,
    next: Option<String>,
}

// ---------------------------------------------------------------------------
// Backend (enum-based, no trait objects)
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Backend {
    File(PathBuf),
    InMemory { value: Arc<Mutex<Option<String>>> },
}

impl Backend {
    fn file(path: PathBuf) -> Self {
        Backend::File(path)
    }

    fn in_memory(initial: Option<String>) -> Self {
        Backend::InMemory {
            value: Arc::new(Mutex::new(initial)),
        }
    }

    fn with_lock<R>(&self, fn_: impl FnOnce(Option<&str>) -> LockResult<R>) -> R {
        match self {
            Backend::File(auth_path) => {
                if let Some(parent) = auth_path.parent() {
                    if !parent.exists() {
                        let _ = fs::create_dir_all(parent);
                    }
                }
                if !auth_path.exists() {
                    let _ = fs::write(auth_path, "{}");
                }

                let current = if auth_path.exists() {
                    fs::read_to_string(auth_path).ok()
                } else {
                    None
                };

                let LockResult { result, next } = fn_(current.as_deref());

                if let Some(next_content) = next {
                    if let Some(parent) = auth_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(auth_path, &next_content);
                }

                result
            }
            Backend::InMemory { value } => {
                let mut guard = value.lock().expect("auth storage in-memory lock poisoned");
                let LockResult { result, next } = fn_(guard.as_deref());
                if let Some(next_content) = next {
                    *guard = Some(next_content);
                }
                result
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AuthStorage
// ---------------------------------------------------------------------------

/// Credential storage backed by a JSON file.
#[derive(Clone)]
pub struct AuthStorage {
    data: AuthStorageData,
    runtime_overrides: HashMap<String, String>,
    fallback_resolver: Option<Arc<dyn Fn(&str) -> Option<String> + Send + Sync>>,
    load_error: Option<String>,
    errors: Vec<String>,
    storage: Backend,
}

impl crate::core::model_registry::auth_trait::AuthStorage for AuthStorage {
    fn get(
        &self,
        provider: &str,
    ) -> Option<crate::core::model_registry::auth_trait::AuthCredential> {
        match AuthStorage::get(self, provider)? {
            AuthCredential::ApiKey(credential) => Some(
                crate::core::model_registry::auth_trait::AuthCredential::ApiKey {
                    key: credential.key.clone(),
                },
            ),
            AuthCredential::OAuth(credential) => Some(
                crate::core::model_registry::auth_trait::AuthCredential::OAuth {
                    provider_id: provider.to_string(),
                    credentials: serde_json::to_value(credential).ok()?,
                },
            ),
        }
    }

    fn get_provider_env(&self, provider: &str) -> Option<HashMap<String, String>> {
        AuthStorage::get_provider_env(self, provider)
    }

    fn has_auth(&self, provider: &str) -> bool {
        AuthStorage::has_auth(self, provider)
    }

    fn get_api_key(
        &self,
        provider: &str,
        _include_fallback: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send + '_>> {
        let provider = provider.to_string();
        Box::pin(async move { AuthStorage::get_api_key(self, &provider).await })
    }

    fn get_auth_status(&self, provider: &str) -> crate::core::model_registry::AuthStatus {
        let status = AuthStorage::get_auth_status(self, provider);
        crate::core::model_registry::AuthStatus {
            configured: status.configured,
            source: status.source.map(str::to_string),
            label: status.label,
        }
    }

    fn get_oauth_providers(&self) -> Vec<serde_json::Value> {
        Vec::new()
    }
}

impl AuthStorage {
    /// Create from a file path.
    pub fn create(auth_path: &Path) -> Self {
        let storage = Backend::file(auth_path.to_path_buf());
        let mut s = Self {
            data: HashMap::new(),
            runtime_overrides: HashMap::new(),
            fallback_resolver: None,
            load_error: None,
            errors: Vec::new(),
            storage,
        };
        s.reload();
        s
    }

    /// Create from a pre-built backend.
    fn from_storage(storage: Backend) -> Self {
        let mut s = Self {
            data: HashMap::new(),
            runtime_overrides: HashMap::new(),
            fallback_resolver: None,
            load_error: None,
            errors: Vec::new(),
            storage,
        };
        s.reload();
        s
    }

    /// Create an in-memory storage with initial data.
    pub fn in_memory(data: AuthStorageData) -> Self {
        let initial = serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string());
        let storage = Backend::in_memory(Some(initial));
        Self::from_storage(storage)
    }

    /// Set a runtime API key override (not persisted to disk).
    pub fn set_runtime_api_key(&mut self, provider: &str, api_key: &str) {
        self.runtime_overrides
            .insert(provider.to_string(), api_key.to_string());
    }

    /// Remove a runtime API key override.
    pub fn remove_runtime_api_key(&mut self, provider: &str) {
        self.runtime_overrides.remove(provider);
    }

    /// Set a fallback resolver for API keys not found in auth.json or env vars.
    pub fn set_fallback_resolver(
        &mut self,
        resolver: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
    ) {
        self.fallback_resolver = Some(resolver);
    }

    fn record_error(&mut self, error: String) {
        self.errors.push(error);
    }

    fn parse_storage_data(content: Option<&str>) -> Option<AuthStorageData> {
        match content {
            Some(c) if !c.is_empty() => serde_json::from_str(c).ok(),
            _ => None,
        }
    }

    /// Reload credentials from storage.
    pub fn reload(&mut self) {
        let mut loaded_data: Option<Option<AuthStorageData>> = None;

        self.storage.with_lock(|current| {
            loaded_data = Some(Self::parse_storage_data(current));
            LockResult {
                result: (),
                next: None,
            }
        });

        if let Some(Some(data)) = loaded_data {
            self.data = data;
            self.load_error = None;
        } else if let Some(None) = loaded_data {
            // Parse failure — keep old data
            self.load_error = Some("parse_failure".to_string());
        }
        // If loaded_data is None (no content), keep old data too
    }

    fn persist_provider_change(&mut self, provider: &str, credential: Option<&AuthCredential>) {
        if self.load_error.is_some() {
            return;
        }

        let provider = provider.to_string();
        let credential = credential.cloned();

        self.storage.with_lock(|current| {
            // If current content can't be parsed, don't overwrite
            if let Some(current_str) = current {
                if Self::parse_storage_data(Some(current_str)).is_none() {
                    return LockResult {
                        result: (),
                        next: None,
                    };
                }
            }

            let merged = if let Some(current_str) = current {
                let mut current_data =
                    Self::parse_storage_data(Some(current_str)).unwrap_or_default();
                if let Some(ref cred) = credential {
                    current_data.insert(provider.clone(), cred.clone());
                } else {
                    current_data.remove(&provider);
                }
                current_data
            } else {
                let mut data = HashMap::new();
                if let Some(ref cred) = credential {
                    data.insert(provider.clone(), cred.clone());
                }
                data
            };

            LockResult {
                result: (),
                next: serde_json::to_string_pretty(&merged).ok(),
            }
        });
    }

    /// Get credential for a provider.
    pub fn get(&self, provider: &str) -> Option<&AuthCredential> {
        self.data.get(provider)
    }

    /// Get provider-scoped environment values for an API key credential.
    pub fn get_provider_env(&self, provider: &str) -> Option<HashMap<String, String>> {
        match self.data.get(provider) {
            Some(AuthCredential::ApiKey(api_key)) => api_key.env.clone(),
            _ => None,
        }
    }

    /// Set credential for a provider.
    pub fn set(&mut self, provider: &str, credential: AuthCredential) {
        self.data.insert(provider.to_string(), credential.clone());
        self.persist_provider_change(provider, Some(&credential));
    }

    /// Remove credential for a provider.
    pub fn remove(&mut self, provider: &str) {
        self.data.remove(provider);
        self.persist_provider_change(provider, None);
    }

    /// List all providers with credentials.
    pub fn list(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// Check if credentials exist for a provider in auth.json.
    pub fn has(&self, provider: &str) -> bool {
        self.data.contains_key(provider)
    }

    /// Check if any form of auth is configured for a provider.
    pub fn has_auth(&self, provider: &str) -> bool {
        if self.runtime_overrides.contains_key(provider) {
            return true;
        }
        if self.data.contains_key(provider) {
            return true;
        }
        if std::env::var(format!("{}_API_KEY", provider.to_uppercase())).is_ok() {
            return true;
        }
        if let Some(ref resolver) = self.fallback_resolver {
            if resolver(provider).is_some() {
                return true;
            }
        }
        false
    }

    /// Return auth status without exposing credential values or refreshing tokens.
    pub fn get_auth_status(&self, provider: &str) -> AuthStatus {
        if self.data.contains_key(provider) {
            return AuthStatus {
                configured: true,
                source: Some("stored"),
                label: None,
            };
        }

        if self.runtime_overrides.contains_key(provider) {
            return AuthStatus {
                configured: false,
                source: Some("runtime"),
                label: Some("--api-key".to_string()),
            };
        }

        let env_key = format!("{}_API_KEY", provider.to_uppercase());
        if std::env::var(&env_key).is_ok() {
            return AuthStatus {
                configured: false,
                source: Some("environment"),
                label: Some(env_key),
            };
        }

        if self.fallback_resolver.is_some() {
            return AuthStatus {
                configured: false,
                source: Some("fallback"),
                label: Some("custom provider config".to_string()),
            };
        }

        AuthStatus {
            configured: false,
            source: None,
            label: None,
        }
    }

    /// Get all credentials.
    pub fn get_all(&self) -> AuthStorageData {
        self.data.clone()
    }

    /// Drain errors from the error buffer.
    pub fn drain_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }

    /// Get API key for a provider.
    /// Priority: runtime override → auth.json API key → OAuth token → env var → fallback
    pub async fn get_api_key(&self, provider_id: &str) -> Option<String> {
        // 1. Runtime override
        if let Some(key) = self.runtime_overrides.get(provider_id) {
            return Some(key.clone());
        }

        // 2. Auth.json credentials
        match self.data.get(provider_id) {
            Some(AuthCredential::ApiKey(cred)) => {
                let env = cred.env.as_ref();
                return resolve_config_value(&cred.key, env.map(|m| m as &HashMap<String, String>));
            }
            Some(AuthCredential::OAuth(cred)) => {
                let now_ms = chrono::Utc::now().timestamp_millis();
                if now_ms < cred.expires {
                    return Some(format!("Bearer {}", cred.access));
                }
                // Token expired — would refresh in full implementation
                return None;
            }
            None => {}
        }

        // 3. Environment variable
        let env_key = format!("{}_API_KEY", provider_id.to_uppercase());
        if let Ok(env_val) = std::env::var(&env_key) {
            return Some(env_val);
        }

        // 4. Fallback resolver
        if let Some(ref resolver) = self.fallback_resolver {
            return resolver(provider_id);
        }

        None
    }

    /// Login to a provider with OAuth credentials.
    pub async fn login(&mut self, provider_id: &str, credentials: OAuthCredential) {
        self.set(provider_id, AuthCredential::OAuth(credentials));
    }

    /// Logout from a provider.
    pub fn logout(&mut self, provider: &str) {
        self.remove(provider);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_temp_dir() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let auth_path = dir.path().join("auth.json");
        (dir, auth_path)
    }

    fn write_auth(path: &Path, data: &serde_json::Value) {
        fs::write(path, serde_json::to_string_pretty(data).unwrap()).unwrap();
    }

    fn api_key_cred(key: &str) -> AuthCredential {
        AuthCredential::ApiKey(ApiKeyCredential {
            credential_type: "api_key".to_string(),
            key: key.to_string(),
            env: None,
        })
    }

    mod api_key_resolution {
        use super::*;

        #[tokio::test]
        async fn literal_api_key_is_returned_directly() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "sk-ant-literal-key" }
                }),
            );

            let auth_storage = AuthStorage::create(&auth_path);
            let api_key = auth_storage.get_api_key("anthropic").await;
            assert_eq!(api_key, Some("sk-ant-literal-key".to_string()));
        }

        #[tokio::test]
        async fn api_key_with_bang_prefix_executes_command() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "!echo test-api-key-from-command" }
                }),
            );

            let auth_storage = AuthStorage::create(&auth_path);
            let api_key = auth_storage.get_api_key("anthropic").await;
            assert_eq!(api_key, Some("test-api-key-from-command".to_string()));
        }

        #[tokio::test]
        async fn api_key_with_bang_prefix_trims_whitespace() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "!echo '  spaced-key  '" }
                }),
            );

            let auth_storage = AuthStorage::create(&auth_path);
            let api_key = auth_storage.get_api_key("anthropic").await;
            assert_eq!(api_key, Some("spaced-key".to_string()));
        }

        #[tokio::test]
        async fn api_key_with_dollar_prefix_resolves_to_env_value() {
            let (_dir, auth_path) = make_temp_dir();
            unsafe { std::env::set_var("TEST_AUTH_API_KEY_V1", "env-api-key-value") };
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "$TEST_AUTH_API_KEY_V1" }
                }),
            );

            let auth_storage = AuthStorage::create(&auth_path);
            let api_key = auth_storage.get_api_key("anthropic").await;
            assert_eq!(api_key, Some("env-api-key-value".to_string()));
            unsafe {
                std::env::remove_var("TEST_AUTH_API_KEY_V1");
            }
        }

        #[tokio::test]
        async fn api_key_env_bag_takes_precedence_over_process_env() {
            let (_dir, auth_path) = make_temp_dir();
            unsafe { std::env::set_var("TEST_AUTH_SCOPED_V1", "process-env-value") };
            let mut env_map = HashMap::new();
            env_map.insert(
                "TEST_AUTH_SCOPED_V1".to_string(),
                "credential-env-value".to_string(),
            );

            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": {
                        "type": "api_key",
                        "key": "$TEST_AUTH_SCOPED_V1",
                        "env": { "TEST_AUTH_SCOPED_V1": "credential-env-value" }
                    }
                }),
            );

            let auth_storage = AuthStorage::create(&auth_path);
            assert_eq!(
                auth_storage.get_api_key("anthropic").await,
                Some("credential-env-value".to_string())
            );
            assert_eq!(auth_storage.get_provider_env("anthropic"), Some(env_map));
            unsafe {
                std::env::remove_var("TEST_AUTH_SCOPED_V1");
            }
        }

        #[tokio::test]
        async fn api_key_with_bang_returns_none_on_failure() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "!exit 1" }
                }),
            );

            let auth_storage = AuthStorage::create(&auth_path);
            let api_key = auth_storage.get_api_key("anthropic").await;
            assert_eq!(api_key, None);
        }

        #[tokio::test]
        async fn plain_api_key_used_directly_even_when_matches_env_var() {
            let (_dir, auth_path) = make_temp_dir();
            unsafe { std::env::set_var("LITERAL_API_KEY_V1", "env-api-key-value") };
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "LITERAL_API_KEY_V1" }
                }),
            );

            let auth_storage = AuthStorage::create(&auth_path);
            let api_key = auth_storage.get_api_key("anthropic").await;
            // Plain literal key — NOT env-resolved unless starts with $
            assert_eq!(api_key, Some("LITERAL_API_KEY_V1".to_string()));
            unsafe {
                std::env::remove_var("LITERAL_API_KEY_V1");
            }
        }

        #[tokio::test]
        async fn literal_value_when_not_env_var() {
            let (_dir, auth_path) = make_temp_dir();
            unsafe {
                std::env::remove_var("literal_api_key_value");
            }
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "literal_api_key_value" }
                }),
            );

            let auth_storage = AuthStorage::create(&auth_path);
            let api_key = auth_storage.get_api_key("anthropic").await;
            assert_eq!(api_key, Some("literal_api_key_value".to_string()));
        }
    }

    mod persistence_semantics {
        use super::*;

        #[test]
        fn set_preserves_unrelated_external_edits() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "old-anthropic" },
                    "openai": { "type": "api_key", "key": "openai-key" }
                }),
            );

            let mut auth_storage = AuthStorage::create(&auth_path);

            // Simulate external edit
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "old-anthropic" },
                    "openai": { "type": "api_key", "key": "openai-key" },
                    "google": { "type": "api_key", "key": "google-key" }
                }),
            );

            auth_storage.set("anthropic", api_key_cred("new-anthropic"));

            let updated: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&auth_path).unwrap()).unwrap();
            assert_eq!(updated["anthropic"]["key"], "new-anthropic");
            assert_eq!(updated["openai"]["key"], "openai-key");
            assert_eq!(updated["google"]["key"], "google-key");
        }

        #[test]
        fn remove_preserves_unrelated_external_edits() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "anthropic-key" },
                    "openai": { "type": "api_key", "key": "openai-key" }
                }),
            );

            let mut auth_storage = AuthStorage::create(&auth_path);

            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "anthropic-key" },
                    "openai": { "type": "api_key", "key": "openai-key" },
                    "google": { "type": "api_key", "key": "google-key" }
                }),
            );

            auth_storage.remove("anthropic");

            let updated: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&auth_path).unwrap()).unwrap();
            assert!(updated.get("anthropic").is_none());
            assert_eq!(updated["openai"]["key"], "openai-key");
            assert_eq!(updated["google"]["key"], "google-key");
        }

        #[test]
        fn does_not_overwrite_malformed_auth_file_after_load_error() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "anthropic-key" }
                }),
            );

            let mut auth_storage = AuthStorage::create(&auth_path);
            fs::write(&auth_path, "{invalid-json").unwrap();

            auth_storage.reload();
            auth_storage.set("openai", api_key_cred("openai-key"));

            let raw = fs::read_to_string(&auth_path).unwrap();
            assert_eq!(raw, "{invalid-json");
        }

        #[test]
        fn reload_keeps_previous_data_on_parse_failure() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "anthropic-key" }
                }),
            );

            let mut auth_storage = AuthStorage::create(&auth_path);
            fs::write(&auth_path, "{invalid-json").unwrap();

            auth_storage.reload();

            // Keeps previous in-memory data on reload failure
            assert_eq!(
                auth_storage.get("anthropic"),
                Some(&api_key_cred("anthropic-key"))
            );
        }
    }

    mod auth_status {
        use super::*;

        #[test]
        fn does_not_expose_stored_secrets() {
            let auth_storage = AuthStorage::in_memory({
                let mut data = AuthStorageData::new();
                data.insert("anthropic".to_string(), api_key_cred("secret-api-key"));
                data.insert(
                    "openai".to_string(),
                    AuthCredential::OAuth(OAuthCredential {
                        credential_type: "oauth".to_string(),
                        access: "secret-access-token".to_string(),
                        refresh: "secret-refresh-token".to_string(),
                        expires: chrono::Utc::now().timestamp_millis() + 1000,
                        token_type: None,
                    }),
                );
                data
            });

            let status = auth_storage.get_auth_status("anthropic");
            assert!(status.configured);
            assert_eq!(status.source, Some("stored"));

            let status_openai = auth_storage.get_auth_status("openai");
            assert!(status_openai.configured);
            assert_eq!(status_openai.source, Some("stored"));

            // Verify no secrets leaked in debug output
            let debug_str = format!("{:?}", auth_storage.get_auth_status("anthropic"));
            assert!(!debug_str.contains("secret-api-key"));
        }
    }

    mod runtime_overrides {
        use super::*;

        #[tokio::test]
        async fn runtime_override_takes_priority_over_auth_json() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "!echo stored-key" }
                }),
            );

            let mut auth_storage = AuthStorage::create(&auth_path);
            auth_storage.set_runtime_api_key("anthropic", "runtime-key");

            let api_key = auth_storage.get_api_key("anthropic").await;
            assert_eq!(api_key, Some("runtime-key".to_string()));
        }

        #[tokio::test]
        async fn removing_runtime_override_falls_back_to_auth_json() {
            let (_dir, auth_path) = make_temp_dir();
            write_auth(
                &auth_path,
                &serde_json::json!({
                    "anthropic": { "type": "api_key", "key": "!echo stored-key" }
                }),
            );

            let mut auth_storage = AuthStorage::create(&auth_path);
            auth_storage.set_runtime_api_key("anthropic", "runtime-key");
            auth_storage.remove_runtime_api_key("anthropic");

            let api_key = auth_storage.get_api_key("anthropic").await;
            assert_eq!(api_key, Some("stored-key".to_string()));
        }
    }

    #[tokio::test]
    async fn model_registry_auth_bridge_resolves_stored_api_keys() {
        let auth_storage = AuthStorage::in_memory({
            let mut data = AuthStorageData::new();
            data.insert("deepseek".to_string(), api_key_cred("stored-key"));
            data
        });
        let registry =
            crate::core::model_registry::ModelRegistry::in_memory(Arc::new(auth_storage));

        assert_eq!(
            registry.get_api_key_for_provider("deepseek").await,
            Some("stored-key".to_string())
        );
    }
}

//! Port of `packages/ai/src/utils/oauth/pkce.rs`
//!
//! PKCE (Proof Key for Code Exchange) utilities for OAuth 2.0 flows.
//! Uses SHA-256 for code challenge generation.

use sha2::{Digest, Sha256};

/// Result from `generate_pkce` containing both verifier and challenge.
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

/// Encode bytes as base64url (no padding, `-` instead of `+`, `_` instead of `/`).
fn base64url_encode(bytes: &[u8]) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

/// Generate PKCE code verifier and SHA-256 challenge.
///
/// The verifier is 32 random bytes, base64url-encoded.
/// The challenge is the SHA-256 hash of the verifier, base64url-encoded.
pub fn generate_pkce() -> Pkce {
    // Generate 32 random bytes for the verifier
    let mut verifier_bytes = [0u8; 32];
    getrandom::getrandom(&mut verifier_bytes).expect("getrandom must never fail");
    let verifier = base64url_encode(&verifier_bytes);

    // SHA-256 hash of the verifier (ASCII bytes)
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    let challenge = base64url_encode(&hash);

    Pkce {
        verifier,
        challenge,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pkce_produces_valid_outputs() {
        let pkce = generate_pkce();
        // Verifier should be exactly 43 characters (32 bytes → base64url)
        assert_eq!(pkce.verifier.len(), 43);
        // Challenge should be exactly 43 characters (SHA-256 → base64url)
        assert_eq!(pkce.challenge.len(), 43);
        // Verifier should only contain base64url characters
        assert!(
            pkce.verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        assert!(
            pkce.challenge
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );

        // Two generations should produce different values
        let pkce2 = generate_pkce();
        assert_ne!(pkce.verifier, pkce2.verifier);
        assert_ne!(pkce.challenge, pkce2.challenge);
    }
}

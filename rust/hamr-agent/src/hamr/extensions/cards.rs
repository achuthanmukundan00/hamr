//! Port of `packages/coding-agent/src/hamr/extensions/cards.ts`.
//!
//! Cards extension — stub placeholder for future card-based UI components.
//! The TS counterpart is also a stub.

use std::sync::Arc;

use crate::core::extensions::types::{ExtensionAPI, ExtensionFactory};

/// Extension factory name constant. Used for feature-flag gating.
pub const EXTENSION_NAME: &str = "hamr-cards";

/// Creates the hamr cards extension (currently a stub).
///
/// Mirror of `hamrCardsExtension` in the TS source.
pub fn hamr_cards_extension() -> ExtensionFactory {
    Arc::new(|_pi: Arc<dyn ExtensionAPI>| Box::pin(std::future::ready(())))
}

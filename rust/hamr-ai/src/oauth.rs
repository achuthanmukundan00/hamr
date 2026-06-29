//! Port of `packages/ai/src/oauth.ts` (`export * from "./utils/oauth/index.ts"`).
//!
//! Re-exports the OAuth utilities so they are reachable from the crate root,
//! mirroring the TS barrel module.

pub use crate::utils::oauth::*;

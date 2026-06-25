//! Tool-call parsers for non-native function-calling models.
//!
//! Mirror of `packages/coding-agent/src/hamr/providers/parsers/`.

pub mod deepseek;
pub mod generic;
pub mod glm_step;
pub mod hermes;
pub mod json_in_tags;
pub mod llama3_json;
pub mod mistral;
pub mod pythonic;
pub mod qwen3_xml;
pub mod registry;
pub mod types;
pub mod utils;
pub mod xlam;

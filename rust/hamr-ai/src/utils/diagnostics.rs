//! Port of `packages/ai/src/utils/diagnostics.ts`.
//!
//! Redacted provider/runtime diagnostics attached to assistant messages on
//! failures and recoveries.

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// An error `code` from a thrown value — either a string or a number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum DiagnosticCode {
    Str(String),
    Num(i64),
}

/// Structured information extracted from a thrown error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticErrorInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<DiagnosticCode>,
}

/// A single diagnostic entry attached to an assistant message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessageDiagnostic {
    #[serde(rename = "type")]
    pub diagnostic_type: String,
    /// Milliseconds since the Unix epoch (mirrors JS `Date.now()`).
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DiagnosticErrorInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Format an arbitrary thrown value into a human-readable string.
///
/// Mirrors the TS branch order: `Error` → its message (falling back to its
/// name), any other value → its `Display`/`to_string` form.
pub fn format_thrown_value<E: std::error::Error + ?Sized>(value: &E) -> String {
    let message = value.to_string();
    if message.is_empty() {
        // JS `error.message || error.name` — Rust errors have no `name`, so the
        // type's Display output already stands in for both.
        std::any::type_name::<E>().to_string()
    } else {
        message
    }
}

/// Extract structured diagnostic info from a thrown error.
///
/// Rust errors do not carry a JS-style `name`/`stack`/`code`, so those are
/// derived where possible: `name` from the error's `Display`-less source chain
/// is unavailable and left `None`; `message` is the `Display` form.
pub fn extract_diagnostic_error<E: std::error::Error + ?Sized>(error: &E) -> DiagnosticErrorInfo {
    DiagnosticErrorInfo {
        name: None,
        message: error.to_string(),
        stack: None,
        code: None,
    }
}

/// Build a diagnostic info from a plain message (the "non-Error thrown value" path).
pub fn diagnostic_error_from_message(message: impl Into<String>) -> DiagnosticErrorInfo {
    DiagnosticErrorInfo {
        name: Some("ThrownValue".to_string()),
        message: message.into(),
        stack: None,
        code: None,
    }
}

/// Create an [`AssistantMessageDiagnostic`] from an error and optional details.
pub fn create_assistant_message_diagnostic<E: std::error::Error + ?Sized>(
    diagnostic_type: impl Into<String>,
    error: &E,
    details: Option<HashMap<String, serde_json::Value>>,
) -> AssistantMessageDiagnostic {
    AssistantMessageDiagnostic {
        diagnostic_type: diagnostic_type.into(),
        timestamp: Utc::now().timestamp_millis(),
        error: Some(extract_diagnostic_error(error)),
        details,
    }
}

/// Types that carry an optional list of diagnostics (mirrors the TS generic
/// constraint `T extends { diagnostics?: AssistantMessageDiagnostic[] }`).
pub trait WithDiagnostics {
    fn diagnostics_mut(&mut self) -> &mut Option<Vec<AssistantMessageDiagnostic>>;
}

/// Append a diagnostic to a message that carries diagnostics.
pub fn append_assistant_message_diagnostic<T: WithDiagnostics>(
    message: &mut T,
    diagnostic: AssistantMessageDiagnostic,
) {
    let slot = message.diagnostics_mut();
    let mut list = slot.take().unwrap_or_default();
    list.push(diagnostic);
    *slot = Some(list);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Holder {
        diagnostics: Option<Vec<AssistantMessageDiagnostic>>,
    }
    impl WithDiagnostics for Holder {
        fn diagnostics_mut(&mut self) -> &mut Option<Vec<AssistantMessageDiagnostic>> {
            &mut self.diagnostics
        }
    }

    #[test]
    fn extracts_message_from_error() {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "boom");
        let info = extract_diagnostic_error(&err);
        assert_eq!(info.message, "boom");
    }

    #[test]
    fn appends_diagnostics() {
        let mut holder = Holder::default();
        let err = std::io::Error::new(std::io::ErrorKind::Other, "x");
        append_assistant_message_diagnostic(
            &mut holder,
            create_assistant_message_diagnostic("retry", &err, None),
        );
        append_assistant_message_diagnostic(
            &mut holder,
            create_assistant_message_diagnostic("retry", &err, None),
        );
        assert_eq!(holder.diagnostics.unwrap().len(), 2);
    }

    #[test]
    fn code_serializes_untagged() {
        let s = serde_json::to_string(&DiagnosticCode::Num(429)).unwrap();
        assert_eq!(s, "429");
        let s = serde_json::to_string(&DiagnosticCode::Str("ETIMEDOUT".into())).unwrap();
        assert_eq!(s, "\"ETIMEDOUT\"");
    }
}

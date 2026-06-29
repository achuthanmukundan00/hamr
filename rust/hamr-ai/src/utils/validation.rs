//! Port of `packages/ai/src/utils/validation.ts`.
//!
//! JSON-Schema validation for tool-call arguments, using the `jsonschema` crate
//! in place of TypeBox's `Value.Check` / `Compile`.  Coercion is **not**
//! re-implemented (TypeBox `Value.Convert` and `coerceWithJsonSchema` are
//! deeply dynamic); raw-argument passthrough mirrors the TS fallback when the
//! validator isn't TypeBox-backed.

use jsonschema::Draft;
use serde_json::Value;

use crate::types::{Tool, ToolCall};

/// Compile a JSON Schema into a validator, returning `None` for invalid schemas.
fn compile_schema(parameters: &Value) -> Option<jsonschema::Validator> {
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .build(parameters)
        .ok()
}

/// Format a validation-error path into a human-readable dotted string.
/// Mirrors the TS `formatValidationPath`.
fn format_validation_path(instance_path: &str) -> String {
    let path = instance_path.trim_start_matches('/').replace('/', ".");
    if path.is_empty() {
        "root".to_string()
    } else {
        path
    }
}

/// Validate tool-call arguments against the tool's JSON Schema.
/// Mirrors the TS `validateToolArguments`.
pub fn validate_tool_arguments(tool: &Tool, tool_call: &ToolCall) -> Result<Value, String> {
    let args = &tool_call.arguments;

    let Some(validator) = compile_schema(&tool.parameters) else {
        return Ok(args.clone());
    };

    if validator.is_valid(args) {
        return Ok(args.clone());
    }

    let errors: Vec<String> = validator
        .iter_errors(args)
        .map(|err| {
            let path = format_validation_path(&err.instance_path.to_string());
            format!("  - {path}: {err}")
        })
        .collect();

    let error_message = if errors.is_empty() {
        format!(
            "Validation failed for tool \"{}\":\n  Unknown validation error\n\nReceived arguments:\n{}",
            tool_call.name,
            serde_json::to_string_pretty(args).unwrap_or_default(),
        )
    } else {
        format!(
            "Validation failed for tool \"{}\":\n{}\n\nReceived arguments:\n{}",
            tool_call.name,
            errors.join("\n"),
            serde_json::to_string_pretty(args).unwrap_or_default(),
        )
    };

    Err(error_message)
}

/// Find a tool by name and validate the tool-call arguments against its schema.
/// Mirrors the TS `validateToolCall`.
pub fn validate_tool_call(tools: &[Tool], tool_call: &ToolCall) -> Result<Value, String> {
    let tool = tools
        .iter()
        .find(|t| t.name == tool_call.name)
        .ok_or_else(|| format!("Tool \"{}\" not found", tool_call.name))?;
    validate_tool_arguments(tool, tool_call)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn simple_tool(name: &str, schema: Value) -> Tool {
        Tool {
            name: name.to_string(),
            description: "test".to_string(),
            parameters: schema,
        }
    }

    #[test]
    fn valid_args_pass() {
        let tool = simple_tool(
            "inc",
            json!({"type": "object", "properties": {"x": {"type": "number"}}, "required": ["x"]}),
        );
        let call = ToolCall {
            id: "c1".into(),
            name: "inc".into(),
            arguments: json!({"x": 1}),
            thought_signature: None,
        };
        assert!(validate_tool_arguments(&tool, &call).is_ok());
    }

    #[test]
    fn missing_required_field_fails() {
        let tool = simple_tool(
            "inc",
            json!({"type": "object", "properties": {"x": {"type": "number"}}, "required": ["x"]}),
        );
        let call = ToolCall {
            id: "c1".into(),
            name: "inc".into(),
            arguments: json!({}),
            thought_signature: None,
        };
        let err = validate_tool_arguments(&tool, &call).unwrap_err();
        assert!(err.contains("Validation failed"), "{err}");
    }

    #[test]
    fn unknown_tool_fails() {
        let call = ToolCall {
            id: "c1".into(),
            name: "missing".into(),
            arguments: json!({}),
            thought_signature: None,
        };
        let err = validate_tool_call(&[], &call).unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }

    #[test]
    fn empty_schema_accepts_args() {
        let tool = Tool {
            name: "opaque".into(),
            description: "".into(),
            parameters: json!({}),
        };
        let call = ToolCall {
            id: "c1".into(),
            name: "opaque".into(),
            arguments: json!({"anything": "goes"}),
            thought_signature: None,
        };
        assert!(validate_tool_arguments(&tool, &call).is_ok());
    }

    #[test]
    fn wrong_type_rejected() {
        let tool = simple_tool(
            "inc",
            json!({"type": "object", "properties": {"x": {"type": "number"}}, "required": ["x"]}),
        );
        let call = ToolCall {
            id: "c2".into(),
            name: "inc".into(),
            arguments: json!({"x": "not_a_number"}),
            thought_signature: None,
        };
        let err = validate_tool_arguments(&tool, &call).unwrap_err();
        assert!(err.contains("Validation failed"), "{err}");
    }

    #[test]
    fn extra_fields_accepted_by_open_schema() {
        let tool = simple_tool(
            "flex",
            json!({"type": "object", "properties": {"a": {"type": "string"}}}),
        );
        let call = ToolCall {
            id: "c3".into(),
            name: "flex".into(),
            arguments: json!({"a": "ok", "b": 42}),
            thought_signature: None,
        };
        // JSON Schema allows additional properties by default
        assert!(validate_tool_arguments(&tool, &call).is_ok());
    }

    #[test]
    fn invalid_schema_falls_through() {
        let tool = simple_tool("bad", json!({"type": "invalid_type_xyz"}));
        let call = ToolCall {
            id: "c4".into(),
            name: "bad".into(),
            arguments: json!({"a": 1}),
            thought_signature: None,
        };
        // When the schema itself is invalid, compile_schema returns None → args pass through
        assert!(validate_tool_arguments(&tool, &call).is_ok());
    }

    #[test]
    fn format_validation_path_root() {
        assert_eq!(format_validation_path(""), "root");
        assert_eq!(format_validation_path("/"), "root");
    }

    #[test]
    fn format_validation_path_nested() {
        assert_eq!(format_validation_path("/properties/foo"), "properties.foo");
    }
}

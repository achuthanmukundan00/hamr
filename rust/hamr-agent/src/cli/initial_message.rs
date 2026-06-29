//! Combine stdin content, @file text, and CLI messages into an initial prompt.
//!
//! Port of `packages/coding-agent/src/cli/initial-message.ts`.

use hamr_ai::types::ImageContent;

use super::args::Args;

/// Input for building the initial message.
pub struct InitialMessageInput<'a> {
    pub parsed: &'a mut Args,
    pub file_text: Option<&'a str>,
    pub file_images: Option<&'a [ImageContent]>,
    pub stdin_content: Option<&'a str>,
}

/// Result of building the initial message.
pub struct InitialMessageResult {
    pub initial_message: Option<String>,
    pub initial_images: Option<Vec<ImageContent>>,
}

/// Combine stdin content, @file text, and the first CLI message into a single
/// initial prompt for non-interactive mode.
pub fn build_initial_message(input: InitialMessageInput<'_>) -> InitialMessageResult {
    let InitialMessageInput {
        parsed,
        file_text,
        file_images,
        stdin_content,
    } = input;

    let mut parts: Vec<&str> = Vec::new();

    if let Some(content) = stdin_content {
        parts.push(content);
    }
    if let Some(text) = file_text {
        parts.push(text);
    }

    let mut message: Option<String> = None;
    if !parsed.messages.is_empty() {
        message = Some(parsed.messages.remove(0));
    }
    if let Some(ref msg) = message {
        parts.push(msg.as_str());
    }

    InitialMessageResult {
        initial_message: if parts.is_empty() {
            None
        } else {
            Some(parts.join(""))
        },
        initial_images: file_images
            .filter(|imgs| !imgs.is_empty())
            .map(|imgs| imgs.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::args::Args;
    use super::*;
    use std::collections::HashMap;

    fn create_args(messages: Vec<&str>) -> Args {
        Args {
            messages: messages.into_iter().map(String::from).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_merges_stdin_with_first_cli_message() {
        let mut parsed = create_args(vec!["Summarize the text given"]);
        let result = build_initial_message(InitialMessageInput {
            parsed: &mut parsed,
            stdin_content: Some("README contents\n"),
            file_text: None,
            file_images: None,
        });

        assert_eq!(
            result.initial_message.as_deref(),
            Some("README contents\nSummarize the text given")
        );
        assert!(parsed.messages.is_empty());
    }

    #[test]
    fn test_uses_stdin_when_no_cli_message() {
        let mut parsed = create_args(vec![]);
        let result = build_initial_message(InitialMessageInput {
            parsed: &mut parsed,
            stdin_content: Some("README contents"),
            file_text: None,
            file_images: None,
        });

        assert_eq!(result.initial_message.as_deref(), Some("README contents"));
        assert!(parsed.messages.is_empty());
    }

    #[test]
    fn test_combines_stdin_file_and_first_cli_message() {
        let mut parsed = create_args(vec!["Explain it", "Second message"]);
        let result = build_initial_message(InitialMessageInput {
            parsed: &mut parsed,
            stdin_content: Some("stdin\n"),
            file_text: Some("file\n"),
            file_images: None,
        });

        assert_eq!(
            result.initial_message.as_deref(),
            Some("stdin\nfile\nExplain it")
        );
        assert_eq!(parsed.messages, vec!["Second message"]);
    }
}

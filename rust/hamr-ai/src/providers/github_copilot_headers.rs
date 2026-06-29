//! Port of `packages/ai/src/providers/github-copilot-headers.ts`.
//!
//! GitHub Copilot expects dynamic request headers derived from the message
//! list: an initiator hint and a vision-request flag.

use std::collections::HashMap;

use crate::types::{Message, MessageContent};

/// Whether a message's content contains an inline image.
fn content_has_image(content: &[MessageContent]) -> bool {
    content
        .iter()
        .any(|c| matches!(c, MessageContent::Image(_)))
}

/// Indicate whether the request is user-initiated or agent-initiated (e.g. a
/// follow-up after assistant/tool messages).
pub fn infer_copilot_initiator(messages: &[Message]) -> &'static str {
    match messages.last() {
        Some(Message::User(_)) | None => "user",
        Some(_) => "agent",
    }
}

/// Whether any user or tool-result message carries an image.
pub fn has_copilot_vision_input(messages: &[Message]) -> bool {
    messages.iter().any(|msg| match msg {
        Message::User(m) => content_has_image(&m.content),
        Message::ToolResult(m) => content_has_image(&m.content),
        Message::Assistant(_) => false,
    })
}

/// Build the dynamic Copilot headers for a request.
pub fn build_copilot_dynamic_headers(
    messages: &[Message],
    has_images: bool,
) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert(
        "X-Initiator".to_string(),
        infer_copilot_initiator(messages).to_string(),
    );
    headers.insert(
        "Openai-Intent".to_string(),
        "conversation-edits".to_string(),
    );
    if has_images {
        headers.insert("Copilot-Vision-Request".to_string(), "true".to_string());
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ImageContent, MessageRole, TextContent, UserMessage};
    use chrono::Utc;

    fn user(content: Vec<MessageContent>) -> Message {
        Message::User(UserMessage {
            role: MessageRole::User,
            content,
            timestamp: Utc::now(),
        })
    }

    fn text(s: &str) -> MessageContent {
        MessageContent::Text(TextContent {
            text: s.into(),
            text_signature: None,
        })
    }

    fn image() -> MessageContent {
        MessageContent::Image(ImageContent {
            data: "AAAA".into(),
            mime_type: "image/png".into(),
        })
    }

    #[test]
    fn empty_is_user_initiated() {
        assert_eq!(infer_copilot_initiator(&[]), "user");
    }

    #[test]
    fn last_user_is_user_initiated() {
        assert_eq!(infer_copilot_initiator(&[user(vec![text("hi")])]), "user");
    }

    #[test]
    fn vision_detected_in_user_message() {
        assert!(has_copilot_vision_input(&[user(vec![
            text("look"),
            image()
        ])]));
        assert!(!has_copilot_vision_input(&[user(vec![text("look")])]));
    }

    #[test]
    fn dynamic_headers_include_vision_when_images() {
        let headers = build_copilot_dynamic_headers(&[user(vec![image()])], true);
        assert_eq!(headers.get("X-Initiator").map(String::as_str), Some("user"));
        assert_eq!(
            headers.get("Openai-Intent").map(String::as_str),
            Some("conversation-edits")
        );
        assert_eq!(
            headers.get("Copilot-Vision-Request").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn dynamic_headers_omit_vision_without_images() {
        let headers = build_copilot_dynamic_headers(&[user(vec![text("hi")])], false);
        assert!(!headers.contains_key("Copilot-Vision-Request"));
    }
}

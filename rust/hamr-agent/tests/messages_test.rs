//! Tests for `hamr-agent::core::messages`.
//! Ported from packages/coding-agent test files.

use hamr_agent::core::messages::{
    BRANCH_SUMMARY_PREFIX, BRANCH_SUMMARY_SUFFIX, BashExecutionMessage, BranchSummaryMessage,
    COMPACTION_SUMMARY_PREFIX, COMPACTION_SUMMARY_SUFFIX, CompactionSummaryMessage, CustomMessage,
    CustomMessageContent, bash_execution_to_text, convert_to_llm, create_branch_summary_message,
    create_compaction_summary_message, create_custom_message,
};

// ---------------------------------------------------------------------------
// Bash execution message tests
// ---------------------------------------------------------------------------

#[test]
fn test_bash_execution_to_text_basic() {
    let msg = BashExecutionMessage {
        command: "ls -la".to_string(),
        output: "total 42\ndrwxr-xr-x".to_string(),
        exit_code: Some(0),
        cancelled: false,
        truncated: false,
        full_output_path: None,
        timestamp: 1_000_000,
        exclude_from_context: false,
    };

    let text = bash_execution_to_text(&msg);
    assert!(text.contains("Ran `ls -la`"));
    assert!(text.contains("total 42"));
}

#[test]
fn test_bash_execution_to_text_no_output() {
    let msg = BashExecutionMessage {
        command: "echo".to_string(),
        output: "".to_string(),
        exit_code: Some(0),
        cancelled: false,
        truncated: false,
        full_output_path: None,
        timestamp: 1_000_000,
        exclude_from_context: false,
    };

    let text = bash_execution_to_text(&msg);
    assert!(text.contains("(no output)"));
}

#[test]
fn test_bash_execution_to_text_cancelled() {
    let msg = BashExecutionMessage {
        command: "sleep 999".to_string(),
        output: "some output".to_string(),
        exit_code: None,
        cancelled: true,
        truncated: false,
        full_output_path: None,
        timestamp: 1_000_000,
        exclude_from_context: false,
    };

    let text = bash_execution_to_text(&msg);
    assert!(text.contains("(command cancelled)"));
}

#[test]
fn test_bash_execution_to_text_error_exit_code() {
    let msg = BashExecutionMessage {
        command: "exit 1".to_string(),
        output: "error output".to_string(),
        exit_code: Some(1),
        cancelled: false,
        truncated: false,
        full_output_path: None,
        timestamp: 1_000_000,
        exclude_from_context: false,
    };

    let text = bash_execution_to_text(&msg);
    assert!(text.contains("Command exited with code 1"));
}

#[test]
fn test_bash_execution_to_text_truncated() {
    let msg = BashExecutionMessage {
        command: "cat huge.log".to_string(),
        output: "partial output".to_string(),
        exit_code: Some(0),
        cancelled: false,
        truncated: true,
        full_output_path: Some("/tmp/full-output.log".to_string()),
        timestamp: 1_000_000,
        exclude_from_context: false,
    };

    let text = bash_execution_to_text(&msg);
    assert!(text.contains("[Output truncated. Full output: /tmp/full-output.log]"));
}

#[test]
fn test_bash_execution_to_text_truncated_no_path() {
    let msg = BashExecutionMessage {
        command: "cat huge.log".to_string(),
        output: "partial output".to_string(),
        exit_code: Some(0),
        cancelled: false,
        truncated: true,
        full_output_path: None,
        timestamp: 1_000_000,
        exclude_from_context: false,
    };

    let text = bash_execution_to_text(&msg);
    assert!(!text.contains("[Output truncated. Full output:"));
}

#[test]
fn test_bash_execution_to_text_exclude_from_context() {
    let msg = BashExecutionMessage {
        command: "secret".to_string(),
        output: "secret output".to_string(),
        exit_code: Some(0),
        cancelled: false,
        truncated: false,
        full_output_path: None,
        timestamp: 1_000_000,
        exclude_from_context: true,
    };

    // The exclude_from_context flag is consumed by convert_to_llm,
    // not by bash_execution_to_text. The text itself is generated regardless.
    let text = bash_execution_to_text(&msg);
    assert!(text.contains("Ran `secret`"));
}

// ---------------------------------------------------------------------------
// convert_to_llm tests
// ---------------------------------------------------------------------------

#[test]
fn test_convert_to_llm_passes_through_llm_messages() {
    use chrono::Utc;
    use hamr_ai::types::{Message, MessageContent, TextContent, UserMessage};
    use hamr_harness::types::AgentMessage;

    let user_msg = UserMessage {
        role: hamr_ai::types::MessageRole::User,
        content: vec![MessageContent::Text(TextContent {
            text: "hello".to_string(),
            text_signature: None,
        })],
        timestamp: Utc::now(),
    };

    let msgs = vec![AgentMessage::User(user_msg)];
    let llm_msgs = convert_to_llm(&msgs);

    assert_eq!(llm_msgs.len(), 1);
    match &llm_msgs[0] {
        Message::User(m) => match &m.content[0] {
            MessageContent::Text(t) => assert_eq!(t.text, "hello"),
            _ => panic!("expected text content"),
        },
        _ => panic!("expected user message"),
    }
}

// ---------------------------------------------------------------------------
// Branch summary tests
// ---------------------------------------------------------------------------

#[test]
fn test_create_branch_summary_message() {
    let msg = create_branch_summary_message(
        "summarized content".to_string(),
        "entry-123".to_string(),
        1_000_000,
    );

    assert_eq!(msg.summary, "summarized content");
    assert_eq!(msg.from_id, "entry-123");
    assert_eq!(msg.timestamp, 1_000_000);
}

#[test]
fn test_branch_summary_prefix_and_suffix() {
    let msg = create_branch_summary_message(
        "important changes".to_string(),
        "abc".to_string(),
        2_000_000,
    );

    let full = format!(
        "{}{}{}",
        BRANCH_SUMMARY_PREFIX, msg.summary, BRANCH_SUMMARY_SUFFIX
    );
    assert!(full.contains("<summary>"));
    assert!(full.contains("</summary>"));
    assert!(full.contains("important changes"));
}

// ---------------------------------------------------------------------------
// Compaction summary tests
// ---------------------------------------------------------------------------

#[test]
fn test_create_compaction_summary_message() {
    let msg = create_compaction_summary_message("compacted content".to_string(), 50000, 3_000_000);

    assert_eq!(msg.summary, "compacted content");
    assert_eq!(msg.tokens_before, 50000);
    assert_eq!(msg.timestamp, 3_000_000);
}

#[test]
fn test_compaction_summary_prefix_and_suffix() {
    let msg =
        create_compaction_summary_message("compacted conversation".to_string(), 42000, 4_000_000);

    let full = format!(
        "{}{}{}",
        COMPACTION_SUMMARY_PREFIX, msg.summary, COMPACTION_SUMMARY_SUFFIX
    );
    assert!(full.contains("<summary>"));
    assert!(full.contains("</summary>"));
    assert!(full.contains("compacted conversation"));
}

// ---------------------------------------------------------------------------
// Custom message tests
// ---------------------------------------------------------------------------

#[test]
fn test_create_custom_message_text() {
    let msg = create_custom_message(
        "my-type".to_string(),
        CustomMessageContent::Text("hello from extension".to_string()),
        true,
        Some(serde_json::json!({"key": "value"})),
        1_000_000,
    );

    assert_eq!(msg.custom_type, "my-type");
    assert!(msg.display);
    assert_eq!(msg.timestamp, 1_000_000);

    match &msg.content {
        CustomMessageContent::Text(t) => assert_eq!(t, "hello from extension"),
        _ => panic!("expected text content"),
    }

    assert_eq!(
        msg.details
            .as_ref()
            .unwrap()
            .get("key")
            .unwrap()
            .as_str()
            .unwrap(),
        "value"
    );
}

#[test]
fn test_create_custom_message_blocks() {
    use hamr_ai::types::{MessageContent, TextContent};

    let blocks = vec![
        MessageContent::Text(TextContent {
            text: "block 1".to_string(),
            text_signature: None,
        }),
        MessageContent::Text(TextContent {
            text: "block 2".to_string(),
            text_signature: None,
        }),
    ];

    let msg = create_custom_message(
        "my-type".to_string(),
        CustomMessageContent::Blocks(blocks),
        false,
        None,
        2_000_000,
    );

    assert!(!msg.display);
    match &msg.content {
        CustomMessageContent::Blocks(b) => assert_eq!(b.len(), 2),
        _ => panic!("expected blocks content"),
    }
}

// ---------------------------------------------------------------------------
// Serialization round-trip tests
// ---------------------------------------------------------------------------

#[test]
fn test_bash_execution_message_serde_roundtrip() {
    let msg = BashExecutionMessage {
        command: "echo test".to_string(),
        output: "test".to_string(),
        exit_code: Some(0),
        cancelled: false,
        truncated: false,
        full_output_path: None,
        timestamp: 1_700_000_000_000,
        exclude_from_context: false,
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: BashExecutionMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.command, "echo test");
    assert_eq!(parsed.output, "test");
    assert_eq!(parsed.timestamp, 1_700_000_000_000);
    assert!(!parsed.cancelled);
}

#[test]
fn test_bash_execution_message_with_full_output_path() {
    let msg = BashExecutionMessage {
        command: "ls".to_string(),
        output: "".to_string(),
        exit_code: Some(0),
        cancelled: false,
        truncated: true,
        full_output_path: Some("/tmp/hamr-bash-abc123.log".to_string()),
        timestamp: 1_000_000,
        exclude_from_context: false,
    };

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: BashExecutionMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed.full_output_path,
        Some("/tmp/hamr-bash-abc123.log".to_string())
    );
    assert!(parsed.truncated);
}

#[test]
fn test_branch_summary_message_serde_roundtrip() {
    let msg = create_branch_summary_message(
        "summary of branch".to_string(),
        "entry-456".to_string(),
        5_000_000,
    );

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: BranchSummaryMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.summary, "summary of branch");
    assert_eq!(parsed.from_id, "entry-456");
    assert_eq!(parsed.timestamp, 5_000_000);
}

#[test]
fn test_compaction_summary_message_serde_roundtrip() {
    let msg = create_compaction_summary_message("compacted".to_string(), 99999, 6_000_000);

    let json = serde_json::to_string(&msg).unwrap();
    let parsed: CompactionSummaryMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.summary, "compacted");
    assert_eq!(parsed.tokens_before, 99999);
    assert_eq!(parsed.timestamp, 6_000_000);
}

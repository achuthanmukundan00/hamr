//! Port of `packages/coding-agent/src/core/compaction/branch-summarization.ts`.
//!
//! Branch summarization for tree navigation.
//! When navigating to a different point in the session tree, this generates
//! a summary of the branch being left so context isn't lost.

use crate::core::compaction::compaction::estimate_tokens;
use crate::core::compaction::utils::{
    FileOperations, SUMMARIZATION_SYSTEM_PROMPT, compute_file_lists, create_file_ops,
    extract_file_ops_from_message, format_file_operations, serialize_conversation,
};
use crate::core::messages::{
    convert_to_llm, create_branch_summary_message, create_compaction_summary_message,
    create_custom_message,
};
use hamr_ai::types::{Context, Message, MessageContent, Model, SimpleStreamOptions, TextContent};
use hamr_harness::harness::types::SessionTreeEntry;
use hamr_harness::types::AgentMessage;
use std::collections::{HashMap, HashSet};

// ============================================================================
// Types
// ============================================================================

pub struct BranchSummaryResult {
    pub summary: Option<String>,
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub aborted: bool,
    pub error: Option<String>,
}

/// Details stored in BranchSummaryEntry.details for file tracking
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchSummaryDetails {
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

pub struct BranchPreparation {
    /// Messages extracted for summarization, in chronological order
    pub messages: Vec<AgentMessage>,
    /// File operations extracted from tool calls
    pub file_ops: FileOperations,
    /// Total estimated tokens in messages
    pub total_tokens: u64,
}

pub struct CollectEntriesResult {
    /// Entries to summarize, in chronological order
    pub entries: Vec<SessionTreeEntry>,
    /// Common ancestor between old and new position, if any
    pub common_ancestor_id: Option<String>,
}

pub struct GenerateBranchSummaryOptions {
    /// Model to use for summarization
    pub model: Model,
    /// API key for the model
    pub api_key: Option<String>,
    /// Request headers for the model
    pub headers: Option<HashMap<String, String>>,
    /// Provider-scoped environment values for the model
    pub env: Option<HashMap<String, String>>,
    /// Abort signal for cancellation
    pub signal: Option<tokio::sync::watch::Receiver<bool>>,
    /// Optional custom instructions for summarization
    pub custom_instructions: Option<String>,
    /// If true, customInstructions replaces the default prompt instead of being appended
    pub replace_instructions: bool,
    /// Tokens reserved for prompt + LLM response (default 16384)
    pub reserve_tokens: u64,
}

// ============================================================================
// Entry Collection
// ============================================================================

/// Collect entries that should be summarized when navigating from one position to another.
///
/// Walks from old_leaf_id back to the common ancestor with target_id, collecting entries
/// along the way. Does NOT stop at compaction boundaries - those are included and their
/// summaries become context.
pub fn collect_entries_for_branch_summary(
    path_entries: &[SessionTreeEntry],
    old_leaf_id: Option<&str>,
    target_id: &str,
) -> CollectEntriesResult {
    // If no old position, nothing to summarize
    let Some(old_leaf_id) = old_leaf_id else {
        return CollectEntriesResult {
            entries: Vec::new(),
            common_ancestor_id: None,
        };
    };

    // Build a map for quick lookup by id
    let entry_map: HashMap<&str, &SessionTreeEntry> =
        path_entries.iter().map(|e| (e.id(), e)).collect();

    // Collect the old path as a set of ids (walk parent chain from old_leaf_id)
    let mut old_path_set = HashSet::new();
    let mut current = Some(old_leaf_id);
    while let Some(id) = current {
        old_path_set.insert(id);
        current = entry_map.get(id).and_then(|e| e.parent_id());
    }

    // Walk target path (from target_id to root) and find common ancestor
    let mut common_ancestor_id: Option<&str> = None;
    let mut current = Some(target_id);
    while let Some(id) = current {
        if old_path_set.contains(id) {
            common_ancestor_id = Some(id);
            break;
        }
        current = entry_map.get(id).and_then(|e| e.parent_id());
    }

    // Collect entries from old leaf back to common ancestor
    let mut entries: Vec<SessionTreeEntry> = Vec::new();
    let mut current = Some(old_leaf_id);

    while let Some(id) = current {
        if common_ancestor_id == Some(id) {
            break;
        }
        if let Some(entry) = entry_map.get(id) {
            entries.push((*entry).clone());
            current = entry.parent_id();
        } else {
            break;
        }
    }

    // Reverse to get chronological order
    entries.reverse();

    CollectEntriesResult {
        entries,
        common_ancestor_id: common_ancestor_id.map(|s| s.to_string()),
    }
}

// ============================================================================
// Entry to Message Conversion
// ============================================================================

/// Extract AgentMessage from a session entry.
/// Similar to getMessageFromEntry in compaction.ts but also handles compaction entries.
fn get_message_from_entry(entry: &SessionTreeEntry) -> Option<AgentMessage> {
    match entry {
        SessionTreeEntry::Message { entry: msg_entry } => {
            // Skip tool results - context is in assistant's tool call
            match &msg_entry.message {
                AgentMessage::ToolResult(_) => None,
                _ => Some(msg_entry.message.clone()),
            }
        }
        SessionTreeEntry::CustomMessage { entry: ce } => {
            Some(AgentMessage::Custom(create_custom_message(
                ce.custom_type.clone(),
                ce.content.clone(),
                ce.display,
                ce.details.clone(),
                // timestamp from the entry base as millis
                chrono::DateTime::parse_from_rfc3339(&ce.base.timestamp)
                    .map(|t| t.with_timezone(&chrono::Utc).timestamp_millis())
                    .unwrap_or(0),
            )))
        }
        SessionTreeEntry::BranchSummary { entry: bs } => {
            Some(AgentMessage::BranchSummary(create_branch_summary_message(
                bs.summary.clone(),
                bs.from_id.clone(),
                chrono::DateTime::parse_from_rfc3339(&bs.base.timestamp)
                    .map(|t| t.with_timezone(&chrono::Utc).timestamp_millis())
                    .unwrap_or(0),
            )))
        }
        SessionTreeEntry::Compaction { entry: ce } => Some(AgentMessage::CompactionSummary(
            create_compaction_summary_message(
                ce.summary.clone(),
                ce.tokens_before,
                chrono::DateTime::parse_from_rfc3339(&ce.base.timestamp)
                    .map(|t| t.with_timezone(&chrono::Utc).timestamp_millis())
                    .unwrap_or(0),
            ),
        )),
        // These don't contribute to conversation content
        SessionTreeEntry::ThinkingLevelChange { .. }
        | SessionTreeEntry::ModelChange { .. }
        | SessionTreeEntry::Custom { .. }
        | SessionTreeEntry::Label { .. }
        | SessionTreeEntry::SessionInfo { .. }
        | SessionTreeEntry::ActiveToolsChange { .. }
        | SessionTreeEntry::Leaf { .. } => None,
    }
}

/// Prepare entries for summarization with token budget.
///
/// Walks entries from NEWEST to OLDEST, adding messages until we hit the token budget.
/// This ensures we keep the most recent context when the branch is too long.
///
/// Also collects file operations from:
/// - Tool calls in assistant messages
/// - Existing branch_summary entries' details (for cumulative tracking)
pub fn prepare_branch_entries(
    entries: &[SessionTreeEntry],
    token_budget: u64,
) -> BranchPreparation {
    let mut messages: Vec<AgentMessage> = Vec::new();
    let mut file_ops = create_file_ops();
    let mut total_tokens: u64 = 0;

    // First pass: collect file ops from ALL entries (even if they don't fit in token budget)
    // This ensures we capture cumulative file tracking from nested branch summaries
    // Only extract from pi-generated summaries (from_hook !== true), not extension-generated ones
    for entry in entries {
        if let SessionTreeEntry::BranchSummary { entry: bs } = entry {
            if bs.from_hook != Some(true) {
                if let Some(details) = &bs.details {
                    if let Ok(details) =
                        serde_json::from_value::<BranchSummaryDetails>(details.clone())
                    {
                        for f in details.read_files {
                            file_ops.read.insert(f);
                        }
                        for f in details.modified_files {
                            file_ops.edited.insert(f);
                        }
                    }
                }
            }
        }
    }

    // Second pass: walk from newest to oldest, adding messages until token budget
    for i in (0..entries.len()).rev() {
        let entry = &entries[i];
        let Some(message) = get_message_from_entry(entry) else {
            continue;
        };

        // Extract file ops from assistant messages (tool calls)
        extract_file_ops_from_message(&message, &mut file_ops);

        let tokens = estimate_tokens(&message as &AgentMessage);

        // Check budget before adding
        if token_budget > 0 && total_tokens + tokens > token_budget {
            // If this is a summary entry, try to fit it anyway as it's important context
            let is_summary = matches!(
                entry,
                SessionTreeEntry::Compaction { .. } | SessionTreeEntry::BranchSummary { .. }
            );
            if is_summary && total_tokens < (token_budget as f64 * 0.9) as u64 {
                messages.insert(0, message);
                total_tokens += tokens;
            }
            // Stop - we've hit the budget
            break;
        }

        messages.insert(0, message);
        total_tokens += tokens;
    }

    BranchPreparation {
        messages,
        file_ops,
        total_tokens,
    }
}

// ============================================================================
// Summary Generation
// ============================================================================

const BRANCH_SUMMARY_PREAMBLE: &str = "The user explored a different conversation branch before returning here.\nSummary of that exploration:\n\n";

const BRANCH_SUMMARY_PROMPT: &str =
    "Create a structured summary of this conversation branch for context when returning later.

Use this EXACT format:

## Goal
[What was the user trying to accomplish in this branch?]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned]
- [Or \"(none)\" if none were mentioned]

## Progress
### Done
- [x] [Completed tasks/changes]

### In Progress
- [ ] [Work that was started but not finished]

### Blocked
- [Issues preventing progress, if any]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [What should happen next to continue this work]

Keep each section concise. Preserve exact file paths, function names, and error messages.";

/// Generate a summary of abandoned branch entries.
pub async fn generate_branch_summary(
    entries: &[SessionTreeEntry],
    options: GenerateBranchSummaryOptions,
) -> BranchSummaryResult {
    let context_window = options.model.context_window;
    let reserve_tokens = options.reserve_tokens;
    let token_budget = context_window.saturating_sub(reserve_tokens);

    let preparation = prepare_branch_entries(entries, token_budget);

    if preparation.messages.is_empty() {
        return BranchSummaryResult {
            summary: Some("No content to summarize".to_string()),
            read_files: Vec::new(),
            modified_files: Vec::new(),
            aborted: false,
            error: None,
        };
    }

    // Transform to LLM-compatible messages, then serialize to text
    let llm_messages = convert_to_llm(&preparation.messages);
    let conversation_text = serialize_conversation(&llm_messages);

    // Build prompt
    let instructions = if options.replace_instructions {
        options
            .custom_instructions
            .unwrap_or_else(|| BRANCH_SUMMARY_PROMPT.to_string())
    } else if let Some(ci) = options.custom_instructions {
        format!("{BRANCH_SUMMARY_PROMPT}\n\nAdditional focus: {ci}")
    } else {
        BRANCH_SUMMARY_PROMPT.to_string()
    };

    let prompt_text =
        format!("<conversation>\n{conversation_text}\n</conversation>\n\n{instructions}");

    let now = chrono::Utc::now();
    let summarization_messages = vec![Message::User(hamr_ai::types::UserMessage {
        role: hamr_ai::types::MessageRole::User,
        content: vec![MessageContent::Text(TextContent {
            text: prompt_text,
            text_signature: None,
        })],
        timestamp: now,
    })];

    let context = Context {
        system_prompt: Some(SUMMARIZATION_SYSTEM_PROMPT.to_string()),
        messages: summarization_messages,
        tools: Vec::new(),
    };

    // Build simple options
    let simple_opts = SimpleStreamOptions {
        base: hamr_ai::types::StreamOptions {
            api_key: options.api_key,
            headers: options.headers,
            signal: options.signal,
            env: options.env,
            max_tokens: Some(2048),
            ..Default::default()
        },
        reasoning: None,
        thinking_budgets: None,
    };

    // Call LLM for summarization
    // TODO: when streamFn support is added, wire it here
    // For now just use complete_simple directly
    let response =
        match hamr_ai::stream::complete_simple(options.model.clone(), context, Some(simple_opts))
            .await
        {
            Ok(msg) => msg,
            Err(e) => {
                return BranchSummaryResult {
                    summary: None,
                    read_files: Vec::new(),
                    modified_files: Vec::new(),
                    aborted: false,
                    error: Some(format!("Summarization failed: {e}")),
                };
            }
        };

    // Check if aborted or errored
    if response.stop_reason == hamr_ai::types::StopReason::Aborted {
        return BranchSummaryResult {
            summary: None,
            read_files: Vec::new(),
            modified_files: Vec::new(),
            aborted: true,
            error: None,
        };
    }
    if response.stop_reason == hamr_ai::types::StopReason::Error {
        return BranchSummaryResult {
            summary: None,
            read_files: Vec::new(),
            modified_files: Vec::new(),
            aborted: false,
            error: Some(
                response
                    .error_message
                    .unwrap_or_else(|| "Summarization failed".to_string()),
            ),
        };
    }

    let text_content: String = response
        .content
        .iter()
        .filter_map(|c| match c {
            hamr_ai::types::AssistantContentBlock::Text(tc) => Some(tc.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Prepend preamble to provide context about the branch summary
    let mut summary = format!("{BRANCH_SUMMARY_PREAMBLE}{text_content}");

    // Compute file lists and append to summary
    let file_lists = compute_file_lists(&preparation.file_ops);
    let files_formatted =
        format_file_operations(&file_lists.read_files, &file_lists.modified_files);
    summary.push_str(&files_formatted);

    BranchSummaryResult {
        summary: if summary.is_empty() {
            Some("No summary generated".to_string())
        } else {
            Some(summary)
        },
        read_files: file_lists.read_files,
        modified_files: file_lists.modified_files,
        aborted: false,
        error: None,
    }
}

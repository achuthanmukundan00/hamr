//! Port of `packages/coding-agent/src/core/compaction/compaction.ts`.
//!
//! Context compaction for long sessions.
//!
//! Pure functions for compaction logic. The session manager handles I/O,
//! and after compaction the session is reloaded.

use crate::core::compaction::utils::{
    FileOperations, SUMMARIZATION_SYSTEM_PROMPT, compute_file_lists, create_file_ops,
    extract_file_ops_from_message, format_file_operations, serialize_conversation,
};
use crate::core::messages::{
    convert_to_llm, create_branch_summary_message, create_compaction_summary_message,
    create_custom_message,
};
use hamr_ai::types::{
    AssistantContentBlock, Context, Message, MessageContent, MessageRole, Model,
    SimpleStreamOptions, StopReason, TextContent, Usage,
};
use hamr_harness::harness::session::session::build_session_context;
use hamr_harness::harness::types::SessionTreeEntry;
use hamr_harness::types::AgentMessage;
use std::collections::HashMap;

// ============================================================================
// File Operation Tracking
// ============================================================================

/// Details stored in CompactionEntry.details for file tracking
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionDetails {
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

/// Extract file operations from messages and previous compaction entries.
fn extract_file_operations(
    messages: &[AgentMessage],
    path_entries: &[SessionTreeEntry],
    prev_compaction_index: Option<usize>,
) -> FileOperations {
    let mut file_ops = create_file_ops();

    // Collect from previous compaction's details (if pi-generated)
    if let Some(idx) = prev_compaction_index {
        if let SessionTreeEntry::Compaction { entry: ce } = &path_entries[idx] {
            if ce.from_hook != Some(true) {
                if let Some(details) = &ce.details {
                    if let Ok(details) =
                        serde_json::from_value::<CompactionDetails>(details.clone())
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

    // Extract from tool calls in messages
    for msg in messages {
        extract_file_ops_from_message(msg, &mut file_ops);
    }

    file_ops
}

// ============================================================================
// Message Extraction
// ============================================================================

/// Extract AgentMessage from an entry if it produces one.
/// Returns None for entries that don't contribute to LLM context.
fn get_message_from_entry(entry: &SessionTreeEntry) -> Option<AgentMessage> {
    match entry {
        SessionTreeEntry::Message { entry: msg_entry } => Some(msg_entry.message.clone()),
        SessionTreeEntry::CustomMessage { entry: ce } => {
            Some(AgentMessage::Custom(create_custom_message(
                ce.custom_type.clone(),
                ce.content.clone(),
                ce.display,
                ce.details.clone(),
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
        _ => None,
    }
}

fn get_message_from_entry_for_compaction(entry: &SessionTreeEntry) -> Option<AgentMessage> {
    if matches!(entry, SessionTreeEntry::Compaction { .. }) {
        return None;
    }
    get_message_from_entry(entry)
}

/// Result from compact() - SessionManager adds uuid/parentUuid when saving
pub struct CompactionResult<T = serde_json::Value> {
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    /// Extension-specific data (e.g., ArtifactIndex, version markers for structured compaction)
    pub details: Option<T>,
}

// ============================================================================
// Compaction Settings
// ============================================================================

#[derive(Debug, Clone)]
pub struct CompactionSettings {
    pub enabled: bool,
    pub reserve_tokens: u64,
    pub keep_recent_tokens: u64,
}

pub const DEFAULT_COMPACTION_SETTINGS: CompactionSettings = CompactionSettings {
    enabled: true,
    reserve_tokens: 16384,
    keep_recent_tokens: 20000,
};

// ============================================================================
// Token calculation
// ============================================================================

/// Calculate total context tokens from usage.
pub fn calculate_context_tokens(usage: &Usage) -> u64 {
    let computed = usage.input + usage.output + usage.cache_read + usage.cache_write;
    if usage.total_tokens > 0 {
        usage.total_tokens
    } else {
        computed
    }
}

/// Get usage from an assistant message if available.
/// Skips aborted and error messages as they don't have valid usage data.
fn get_assistant_usage(msg: &AgentMessage) -> Option<&Usage> {
    if let AgentMessage::Assistant(assistant) = msg {
        if assistant.stop_reason != StopReason::Aborted
            && assistant.stop_reason != StopReason::Error
            && assistant.usage.total_tokens > 0
        {
            return Some(&assistant.usage);
        }
    }
    None
}

/// Find the last non-aborted assistant message usage from session entries.
pub fn get_last_assistant_usage(entries: &[SessionTreeEntry]) -> Option<&Usage> {
    for entry in entries.iter().rev() {
        if let SessionTreeEntry::Message { entry: msg_entry } = entry {
            if let Some(usage) = get_assistant_usage(&msg_entry.message) {
                return Some(usage);
            }
        }
    }
    None
}

pub struct ContextUsageEstimate {
    pub tokens: u64,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

fn get_last_assistant_usage_info(messages: &[AgentMessage]) -> Option<(&Usage, usize)> {
    for (i, msg) in messages.iter().enumerate().rev() {
        if let Some(usage) = get_assistant_usage(msg) {
            return Some((usage, i));
        }
    }
    None
}

/// Estimate context tokens from messages, using the last assistant usage when available.
pub fn estimate_context_tokens(messages: &[AgentMessage]) -> ContextUsageEstimate {
    let usage_info = get_last_assistant_usage_info(messages);

    let Some((usage, usage_index)) = usage_info else {
        let mut estimated: u64 = 0;
        for message in messages {
            estimated += estimate_tokens(message);
        }
        return ContextUsageEstimate {
            tokens: estimated,
            usage_tokens: 0,
            trailing_tokens: estimated,
            last_usage_index: None,
        };
    };

    let usage_tokens = calculate_context_tokens(usage);

    // Some providers don't return valid usage. Fall back to estimation.
    if usage_tokens == 0 && !messages.is_empty() {
        let mut estimated: u64 = 0;
        for message in messages {
            estimated += estimate_tokens(message);
        }
        return ContextUsageEstimate {
            tokens: estimated,
            usage_tokens: 0,
            trailing_tokens: estimated,
            last_usage_index: None,
        };
    }

    let mut trailing_tokens: u64 = 0;
    for msg in messages.iter().skip(usage_index + 1) {
        trailing_tokens += estimate_tokens(msg);
    }

    ContextUsageEstimate {
        tokens: usage_tokens + trailing_tokens,
        usage_tokens,
        trailing_tokens,
        last_usage_index: Some(usage_index),
    }
}

/// Check if compaction should trigger based on context usage.
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: &CompactionSettings,
) -> bool {
    if !settings.enabled {
        return false;
    }
    context_tokens > context_window.saturating_sub(settings.reserve_tokens)
}

// ============================================================================
// Cut point detection
// ============================================================================

const ESTIMATED_IMAGE_CHARS: usize = 4800;

fn estimate_text_and_image_content_chars(content: &[hamr_ai::types::MessageContent]) -> usize {
    let mut chars = 0;
    for block in content {
        match block {
            MessageContent::Text(tc) => chars += tc.text.len(),
            MessageContent::Image(_) => chars += ESTIMATED_IMAGE_CHARS,
        }
    }
    chars
}

/// Estimate token count for a message using chars/4 heuristic.
/// This is conservative (overestimates tokens).
pub fn estimate_tokens(message: &AgentMessage) -> u64 {
    let chars = match message {
        AgentMessage::User(user_msg) => {
            estimate_text_and_image_content_chars(&user_msg.content) as u64
        }
        AgentMessage::Assistant(assistant_msg) => {
            let mut c: u64 = 0;
            for block in &assistant_msg.content {
                match block {
                    AssistantContentBlock::Text(tc) => c += tc.text.len() as u64,
                    AssistantContentBlock::Thinking(tc) => c += tc.thinking.len() as u64,
                    AssistantContentBlock::ToolCall(tc) => {
                        c += tc.name.len() as u64
                            + serde_json::to_string(&tc.arguments)
                                .unwrap_or_default()
                                .len() as u64;
                    }
                }
            }
            c
        }
        AgentMessage::Custom(custom_msg) => match &custom_msg.content {
            hamr_harness::types::CustomMessageContent::Text(t) => t.len() as u64,
            hamr_harness::types::CustomMessageContent::Blocks(blocks) => {
                estimate_text_and_image_content_chars(blocks) as u64
            }
        },
        AgentMessage::ToolResult(tool_msg) => {
            estimate_text_and_image_content_chars(&tool_msg.content) as u64
        }
        AgentMessage::BashExecution(bash_msg) => {
            (bash_msg.command.len() + bash_msg.output.len()) as u64
        }
        AgentMessage::BranchSummary(bs_msg) => bs_msg.summary.len() as u64,
        AgentMessage::CompactionSummary(cs_msg) => cs_msg.summary.len() as u64,
    };

    (chars + 3) / 4 // ceil division
}

/// Find valid cut points: indices of user, assistant, custom, or bashExecution messages.
/// Never cut at tool results (they must follow their tool call).
fn find_valid_cut_points(
    entries: &[SessionTreeEntry],
    start_index: usize,
    end_index: usize,
) -> Vec<usize> {
    let mut cut_points = Vec::new();
    for i in start_index..end_index.min(entries.len()) {
        let entry = &entries[i];
        match entry {
            SessionTreeEntry::Message { entry: msg_entry } => {
                match &msg_entry.message {
                    AgentMessage::BashExecution(_)
                    | AgentMessage::Custom(_)
                    | AgentMessage::BranchSummary(_)
                    | AgentMessage::CompactionSummary(_)
                    | AgentMessage::User(_)
                    | AgentMessage::Assistant(_) => cut_points.push(i),
                    AgentMessage::ToolResult(_) => {} // skip
                }
            }
            SessionTreeEntry::BranchSummary { .. } | SessionTreeEntry::CustomMessage { .. } => {
                cut_points.push(i);
            }
            // Not valid cut points
            SessionTreeEntry::ThinkingLevelChange { .. }
            | SessionTreeEntry::ModelChange { .. }
            | SessionTreeEntry::Compaction { .. }
            | SessionTreeEntry::Custom { .. }
            | SessionTreeEntry::Label { .. }
            | SessionTreeEntry::SessionInfo { .. }
            | SessionTreeEntry::ActiveToolsChange { .. }
            | SessionTreeEntry::Leaf { .. } => {}
        }
    }
    cut_points
}

/// Find the user message (or bashExecution) that starts the turn containing the given entry index.
pub fn find_turn_start_index(
    entries: &[SessionTreeEntry],
    entry_index: usize,
    start_index: usize,
) -> i32 {
    for i in (start_index..=entry_index).rev() {
        let entry = &entries[i];
        // branch_summary and custom_message are user-role messages, can start a turn
        if matches!(
            entry,
            SessionTreeEntry::BranchSummary { .. } | SessionTreeEntry::CustomMessage { .. }
        ) {
            return i as i32;
        }
        if let SessionTreeEntry::Message { entry: msg_entry } = entry {
            match &msg_entry.message {
                AgentMessage::User(_) | AgentMessage::BashExecution(_) => return i as i32,
                _ => {}
            }
        }
    }
    -1
}

pub struct CutPointResult {
    /// Index of first entry to keep
    pub first_kept_entry_index: usize,
    /// Index of user message that starts the turn being split, or -1 if not splitting
    pub turn_start_index: i32,
    /// Whether this cut splits a turn (cut point is not a user message)
    pub is_split_turn: bool,
}

/// Find the cut point in session entries that keeps approximately `keep_recent_tokens`.
///
/// Algorithm: Walk backwards from newest, accumulating estimated message sizes.
/// Stop when we've accumulated >= keepRecentTokens. Cut at that point.
pub fn find_cut_point(
    entries: &[SessionTreeEntry],
    start_index: usize,
    end_index: usize,
    keep_recent_tokens: u64,
) -> CutPointResult {
    let cut_points = find_valid_cut_points(entries, start_index, end_index);

    if cut_points.is_empty() {
        return CutPointResult {
            first_kept_entry_index: start_index,
            turn_start_index: -1,
            is_split_turn: false,
        };
    }

    // Walk backwards from newest, accumulating estimated message sizes
    let mut accumulated_tokens: u64 = 0;
    let mut cut_index = cut_points[0]; // Default: keep from first message

    for i in (start_index..end_index.min(entries.len())).rev() {
        let entry = &entries[i];
        let SessionTreeEntry::Message { entry: msg_entry } = entry else {
            continue;
        };

        // Estimate this message's size
        let message_tokens = estimate_tokens(&msg_entry.message);
        accumulated_tokens += message_tokens;

        // Check if we've exceeded the budget
        if accumulated_tokens >= keep_recent_tokens {
            // Find the closest valid cut point at or after this entry
            for cp in &cut_points {
                if *cp >= i {
                    cut_index = *cp;
                    break;
                }
            }
            break;
        }
    }

    // Scan backwards from cut_index to include any non-message entries
    let mut ci = cut_index;
    while ci > start_index {
        let prev_entry = &entries[ci - 1];
        // Stop at compaction boundaries
        if matches!(prev_entry, SessionTreeEntry::Compaction { .. }) {
            break;
        }
        if let SessionTreeEntry::Message { .. } = prev_entry {
            // Stop if we hit any message
            break;
        }
        // Include this non-message entry (settings change, etc.)
        ci -= 1;
    }
    cut_index = ci;

    // Determine if this is a split turn
    let cut_entry = &entries[cut_index];
    let is_user_message = matches!(
        cut_entry,
        SessionTreeEntry::Message { entry: msg_entry } if matches!(&msg_entry.message, AgentMessage::User(_))
    );
    let turn_start_index = if is_user_message {
        -1
    } else {
        find_turn_start_index(entries, cut_index, start_index)
    };

    CutPointResult {
        first_kept_entry_index: cut_index,
        turn_start_index,
        is_split_turn: !is_user_message && turn_start_index != -1,
    }
}

// ============================================================================
// Summarization
// ============================================================================

const SUMMARIZATION_PROMPT: &str = "The messages above are a conversation to summarize. Create a structured context checkpoint summary that another LLM will use to continue the work.

Use this EXACT format:

## Goal
[What is the user trying to accomplish? Can be multiple items if the session covers different tasks.]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned by user]
- [Or \"(none)\" if none were mentioned]

## Progress
### Done
- [x] [Completed tasks/changes]

### In Progress
- [ ] [Current work]

### Blocked
- [Issues preventing progress, if any]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [Ordered list of what should happen next]

## Critical Context
- [Any data, examples, or references needed to continue]
- [Or \"(none)\" if not applicable]

Keep each section concise. Preserve exact file paths, function names, and error messages.";

const UPDATE_SUMMARIZATION_PROMPT: &str = "The messages above are NEW conversation messages to incorporate into the existing summary provided in <previous-summary> tags.

Update the existing structured summary with new information. RULES:
- PRESERVE all existing information from the previous summary
- ADD new progress, decisions, and context from the new messages
- UPDATE the Progress section: move items from \"In Progress\" to \"Done\" when completed
- UPDATE \"Next Steps\" based on what was accomplished
- PRESERVE exact file paths, function names, and error messages
- If something is no longer relevant, you may remove it

Use this EXACT format:

## Goal
[Preserve existing goals, add new ones if the task expanded]

## Constraints & Preferences
- [Preserve existing, add new ones discovered]

## Progress
### Done
- [x] [Include previously done items AND newly completed items]

### In Progress
- [ ] [Current work - update based on progress]

### Blocked
- [Current blockers - remove if resolved]

## Key Decisions
- **[Decision]**: [Brief rationale] (preserve all previous, add new)

## Next Steps
1. [Update based on current state]

## Critical Context
- [Preserve important context, add new if needed]

Keep each section concise. Preserve exact file paths, function names, and error messages.";

fn create_summarization_options(
    model: &Model,
    max_tokens: u64,
    api_key: Option<String>,
    headers: Option<HashMap<String, String>>,
    env: Option<HashMap<String, String>>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    thinking_level: Option<&str>,
) -> SimpleStreamOptions {
    let reasoning = match thinking_level {
        Some("minimal") => Some(hamr_ai::types::ThinkingLevel::Minimal),
        Some("low") => Some(hamr_ai::types::ThinkingLevel::Low),
        Some("medium") => Some(hamr_ai::types::ThinkingLevel::Medium),
        Some("high") => Some(hamr_ai::types::ThinkingLevel::High),
        Some("x-high") => Some(hamr_ai::types::ThinkingLevel::XHigh),
        _ => None,
    };
    // Only set reasoning if model supports it and level is not "off"
    let reasoning = reasoning.filter(|_| model.reasoning);

    SimpleStreamOptions {
        base: hamr_ai::types::StreamOptions {
            api_key,
            headers,
            env,
            signal,
            max_tokens: Some(max_tokens),
            ..Default::default()
        },
        reasoning,
        thinking_budgets: None,
    }
}

/// Generate a summary of the conversation using the LLM.
/// If previous_summary is provided, uses the update prompt to merge.
pub async fn generate_summary(
    current_messages: &[AgentMessage],
    model: &Model,
    reserve_tokens: u64,
    api_key: Option<String>,
    headers: Option<HashMap<String, String>>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    custom_instructions: Option<&str>,
    previous_summary: Option<&str>,
    thinking_level: Option<&str>,
    env: Option<HashMap<String, String>>,
) -> Result<String, String> {
    let max_tokens = if model.max_tokens > 0 {
        ((reserve_tokens as f64 * 0.8) as u64).min(model.max_tokens)
    } else {
        (reserve_tokens as f64 * 0.8) as u64
    };

    // Use update prompt if we have a previous summary, otherwise initial prompt
    let mut base_prompt = if previous_summary.is_some() {
        UPDATE_SUMMARIZATION_PROMPT.to_string()
    } else {
        SUMMARIZATION_PROMPT.to_string()
    };
    if let Some(ci) = custom_instructions {
        base_prompt = format!("{base_prompt}\n\nAdditional focus: {ci}");
    }

    // Serialize conversation to text so model doesn't try to continue it
    let llm_messages = convert_to_llm(current_messages);
    let conversation_text = serialize_conversation(&llm_messages);

    // Build the prompt with conversation wrapped in tags
    let mut prompt_text = format!("<conversation>\n{conversation_text}\n</conversation>\n\n");
    if let Some(ps) = previous_summary {
        prompt_text.push_str(&format!(
            "<previous-summary>\n{ps}\n</previous-summary>\n\n"
        ));
    }
    prompt_text.push_str(&base_prompt);

    let now = chrono::Utc::now();
    let summarization_messages = vec![Message::User(hamr_ai::types::UserMessage {
        role: MessageRole::User,
        content: vec![MessageContent::Text(TextContent {
            text: prompt_text,
            text_signature: None,
        })],
        timestamp: now,
    })];

    let completion_options = create_summarization_options(
        model,
        max_tokens,
        api_key,
        headers,
        env,
        signal,
        thinking_level,
    );

    let context = Context {
        system_prompt: Some(SUMMARIZATION_SYSTEM_PROMPT.to_string()),
        messages: summarization_messages,
        tools: Vec::new(),
    };

    let response =
        hamr_ai::stream::complete_simple(model.clone(), context, Some(completion_options))
            .await
            .map_err(|e| format!("Summarization failed: {e}"))?;

    if response.stop_reason == StopReason::Error {
        return Err(format!(
            "Summarization failed: {}",
            response
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string())
        ));
    }

    let text_content: String = response
        .content
        .iter()
        .filter_map(|c| match c {
            AssistantContentBlock::Text(tc) => Some(tc.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(text_content)
}

// ============================================================================
// Compaction Preparation
// ============================================================================

pub struct CompactionPreparation {
    /// UUID/id of first entry to keep
    pub first_kept_entry_id: String,
    /// Messages that will be summarized and discarded
    pub messages_to_summarize: Vec<AgentMessage>,
    /// Messages that will be turned into turn prefix summary (if splitting)
    pub turn_prefix_messages: Vec<AgentMessage>,
    /// Whether this is a split turn (cut point in middle of turn)
    pub is_split_turn: bool,
    pub tokens_before: u64,
    /// Summary from previous compaction, for iterative update
    pub previous_summary: Option<String>,
    /// File operations extracted from messages_to_summarize
    pub file_ops: FileOperations,
    /// Compaction settings
    pub settings: CompactionSettings,
}

pub fn prepare_compaction(
    path_entries: &[SessionTreeEntry],
    settings: &CompactionSettings,
) -> Option<CompactionPreparation> {
    if path_entries
        .last()
        .map_or(false, |e| matches!(e, SessionTreeEntry::Compaction { .. }))
    {
        return None;
    }

    let mut prev_compaction_index: Option<usize> = None;
    for (i, entry) in path_entries.iter().enumerate().rev() {
        if matches!(entry, SessionTreeEntry::Compaction { .. }) {
            prev_compaction_index = Some(i);
            break;
        }
    }

    let mut previous_summary: Option<String> = None;
    let mut boundary_start = 0;

    if let Some(prev_idx) = prev_compaction_index {
        if let SessionTreeEntry::Compaction { entry: ce } = &path_entries[prev_idx] {
            previous_summary = Some(ce.summary.clone());
            let first_kept_id = &ce.first_kept_entry_id;
            if let Some(found_idx) = path_entries.iter().position(|e| e.id() == first_kept_id) {
                boundary_start = found_idx;
            } else {
                boundary_start = prev_idx + 1;
            }
        }
    }

    let boundary_end = path_entries.len();

    let context = build_session_context(path_entries);
    let tokens_est = estimate_context_tokens(&context.messages);
    let tokens_before = tokens_est.tokens;

    let cut_point = find_cut_point(
        path_entries,
        boundary_start,
        boundary_end,
        settings.keep_recent_tokens,
    );

    // Get UUID of first kept entry
    let first_kept_entry = &path_entries[cut_point.first_kept_entry_index];
    let first_kept_entry_id = first_kept_entry.id().to_string();

    let history_end = if cut_point.is_split_turn {
        cut_point.turn_start_index as usize
    } else {
        cut_point.first_kept_entry_index
    };

    // Messages to summarize (will be discarded after summary)
    let mut messages_to_summarize: Vec<AgentMessage> = Vec::new();
    for entry in path_entries.iter().take(history_end).skip(boundary_start) {
        if let Some(msg) = get_message_from_entry_for_compaction(entry) {
            messages_to_summarize.push(msg);
        }
    }

    // Messages for turn prefix summary (if splitting a turn)
    let mut turn_prefix_messages: Vec<AgentMessage> = Vec::new();
    if cut_point.is_split_turn {
        let ts = cut_point.turn_start_index as usize;
        let fk = cut_point.first_kept_entry_index;
        for entry in path_entries.iter().take(fk).skip(ts) {
            if let Some(msg) = get_message_from_entry_for_compaction(entry) {
                turn_prefix_messages.push(msg);
            }
        }
    }

    // Extract file operations
    let mut file_ops =
        extract_file_operations(&messages_to_summarize, path_entries, prev_compaction_index);

    if cut_point.is_split_turn {
        for msg in &turn_prefix_messages {
            extract_file_ops_from_message(msg, &mut file_ops);
        }
    }

    Some(CompactionPreparation {
        first_kept_entry_id,
        messages_to_summarize,
        turn_prefix_messages,
        is_split_turn: cut_point.is_split_turn,
        tokens_before,
        previous_summary,
        file_ops,
        settings: settings.clone(),
    })
}

// ============================================================================
// Main compaction function
// ============================================================================

const TURN_PREFIX_SUMMARIZATION_PROMPT: &str =
    "This is the PREFIX of a turn that was too large to keep. The SUFFIX (recent work) is retained.

Summarize the prefix to provide context for the retained suffix:

## Original Request
[What did the user ask for in this turn?]

## Early Progress
- [Key decisions and work done in the prefix]

## Context for Suffix
- [Information needed to understand the retained recent work]

Be concise. Focus on what's needed to understand the kept suffix.";

/// Generate summaries for compaction using prepared data.
pub async fn compact(
    preparation: &CompactionPreparation,
    model: &Model,
    api_key: Option<String>,
    headers: Option<HashMap<String, String>>,
    custom_instructions: Option<&str>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    thinking_level: Option<&str>,
    env: Option<HashMap<String, String>>,
) -> Result<CompactionResult<CompactionDetails>, String> {
    let summary = if preparation.is_split_turn && !preparation.turn_prefix_messages.is_empty() {
        // Generate both summaries in parallel
        let history_fut = if !preparation.messages_to_summarize.is_empty() {
            let msgs = preparation.messages_to_summarize.clone();
            let model2 = model.clone();
            let rt = preparation.settings.reserve_tokens;
            let ak = api_key.clone();
            let hdrs = headers.clone();
            let sig = signal.clone();
            let ci = custom_instructions.map(String::from);
            let ps = preparation.previous_summary.clone();
            let tl = thinking_level.map(String::from);
            let env2 = env.clone();
            tokio::spawn(async move {
                generate_summary(
                    &msgs,
                    &model2,
                    rt,
                    ak,
                    hdrs,
                    sig,
                    ci.as_deref(),
                    ps.as_deref(),
                    tl.as_deref(),
                    env2,
                )
                .await
            })
        } else {
            tokio::spawn(async move { Ok::<String, String>("No prior history.".to_string()) })
        };

        let prefix_fut = {
            let msgs = preparation.turn_prefix_messages.clone();
            let model2 = model.clone();
            let rt = preparation.settings.reserve_tokens;
            let ak = api_key.clone();
            let hdrs = headers.clone();
            let env2 = env.clone();
            let sig = signal.clone();
            let tl = thinking_level.map(String::from);
            tokio::spawn(async move {
                generate_turn_prefix_summary(&msgs, &model2, rt, ak, hdrs, env2, sig, tl.as_deref())
                    .await
            })
        };

        let (history_result, prefix_result) = tokio::join!(history_fut, prefix_fut);
        let history_str =
            history_result.map_err(|e| format!("History summarization task panicked: {e}"))??;
        let prefix_str = prefix_result
            .map_err(|e| format!("Turn prefix summarization task panicked: {e}"))??;

        format!("{history_str}\n\n---\n\n**Turn Context (split turn):**\n\n{prefix_str}")
    } else {
        generate_summary(
            &preparation.messages_to_summarize,
            model,
            preparation.settings.reserve_tokens,
            api_key,
            headers,
            signal,
            custom_instructions,
            preparation.previous_summary.as_deref(),
            thinking_level,
            env,
        )
        .await?
    };

    // Compute file lists and append to summary
    let file_lists = compute_file_lists(&preparation.file_ops);
    let files_formatted =
        format_file_operations(&file_lists.read_files, &file_lists.modified_files);
    let final_summary = format!("{summary}{files_formatted}");

    Ok(CompactionResult {
        summary: final_summary,
        first_kept_entry_id: preparation.first_kept_entry_id.clone(),
        tokens_before: preparation.tokens_before,
        details: Some(CompactionDetails {
            read_files: file_lists.read_files,
            modified_files: file_lists.modified_files,
        }),
    })
}

/// Generate a summary for a turn prefix (when splitting a turn).
async fn generate_turn_prefix_summary(
    messages: &[AgentMessage],
    model: &Model,
    reserve_tokens: u64,
    api_key: Option<String>,
    headers: Option<HashMap<String, String>>,
    env: Option<HashMap<String, String>>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    thinking_level: Option<&str>,
) -> Result<String, String> {
    let max_tokens = if model.max_tokens > 0 {
        ((reserve_tokens as f64 * 0.5) as u64).min(model.max_tokens)
    } else {
        (reserve_tokens as f64 * 0.5) as u64
    };

    let llm_messages = convert_to_llm(messages);
    let conversation_text = serialize_conversation(&llm_messages);

    let prompt_text = format!(
        "<conversation>\n{conversation_text}\n</conversation>\n\n{TURN_PREFIX_SUMMARIZATION_PROMPT}"
    );

    let now = chrono::Utc::now();
    let summarization_messages = vec![Message::User(hamr_ai::types::UserMessage {
        role: MessageRole::User,
        content: vec![MessageContent::Text(TextContent {
            text: prompt_text,
            text_signature: None,
        })],
        timestamp: now,
    })];

    let completion_options = create_summarization_options(
        model,
        max_tokens,
        api_key,
        headers,
        env,
        signal,
        thinking_level,
    );

    let context = Context {
        system_prompt: Some(SUMMARIZATION_SYSTEM_PROMPT.to_string()),
        messages: summarization_messages,
        tools: Vec::new(),
    };

    let response =
        hamr_ai::stream::complete_simple(model.clone(), context, Some(completion_options))
            .await
            .map_err(|e| format!("Turn prefix summarization failed: {e}"))?;

    if response.stop_reason == StopReason::Error {
        return Err(format!(
            "Turn prefix summarization failed: {}",
            response
                .error_message
                .unwrap_or_else(|| "Unknown error".to_string())
        ));
    }

    let text_content: String = response
        .content
        .iter()
        .filter_map(|c| match c {
            AssistantContentBlock::Text(tc) => Some(tc.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(text_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hamr_ai::types::UsageCost;

    fn mock_usage(input: u64, output: u64, cache_read: u64, cache_write: u64) -> Usage {
        Usage {
            input,
            output,
            cache_read,
            cache_write,
            cache_write_1h: None,
            total_tokens: input + output + cache_read + cache_write,
            cost: UsageCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
                total: 0.0,
            },
        }
    }

    fn default_settings() -> CompactionSettings {
        CompactionSettings {
            enabled: true,
            reserve_tokens: 10_000,
            keep_recent_tokens: 5_000,
        }
    }

    #[test]
    fn test_calculate_context_tokens_with_total() {
        let usage = mock_usage(100, 50, 200, 30);
        assert_eq!(calculate_context_tokens(&usage), 380);
    }

    #[test]
    fn test_calculate_context_tokens_computed() {
        let usage = Usage {
            total_tokens: 0,
            ..mock_usage(100, 50, 200, 30)
        };
        assert_eq!(calculate_context_tokens(&usage), 380);
    }

    #[test]
    fn test_calculate_context_tokens_zero() {
        let usage = mock_usage(0, 0, 0, 0);
        assert_eq!(calculate_context_tokens(&usage), 0);
    }

    #[test]
    fn test_estimate_context_tokens_empty() {
        let messages: Vec<AgentMessage> = vec![];
        let estimate = estimate_context_tokens(&messages);
        assert_eq!(estimate.tokens, 0);
        assert_eq!(estimate.last_usage_index, None);
    }

    #[test]
    fn test_should_compact_below_threshold() {
        let settings = default_settings();
        // context_window=100_000, reserve=10_000 => threshold=90_000
        assert!(!should_compact(50_000, 100_000, &settings));
    }

    #[test]
    fn test_should_compact_above_threshold() {
        let settings = default_settings();
        // context_window=100_000, reserve=10_000 => threshold=90_000
        assert!(should_compact(95_000, 100_000, &settings));
    }

    #[test]
    fn test_should_compact_disabled() {
        let settings = CompactionSettings {
            enabled: false,
            ..default_settings()
        };
        assert!(!should_compact(999_999, 100_000, &settings));
    }

    #[test]
    fn test_should_compact_exact_threshold() {
        let settings = default_settings();
        // exactly at threshold (90_000) should NOT compact (> not >=)
        assert!(!should_compact(90_000, 100_000, &settings));
        assert!(should_compact(90_001, 100_000, &settings));
    }
}

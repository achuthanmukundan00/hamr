//! Port of `packages/agent/src/harness/compaction/compaction.ts`.

use crate::harness::compaction::utils::{
    FileOperations, compute_file_lists, create_file_ops, extract_file_ops_from_message,
    format_file_operations, serialize_conversation,
};
use crate::harness::messages::{
    convert_to_llm, create_branch_summary_message, create_compaction_summary_message,
    create_custom_message,
};
use crate::harness::session::session::build_session_context;
use crate::harness::types::SessionTreeEntry;
use crate::types::{AgentMessage, CustomMessageContent};
use hamr_ai::stream::complete_simple;
use hamr_ai::types::{
    AssistantContentBlock, MessageContent, Model, SimpleStreamOptions, StopReason, ThinkingLevel,
    Usage,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionDetails {
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    pub details: Option<CompactionDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionSettings {
    pub enabled: bool,
    pub reserve_tokens: u64,
    pub keep_recent_tokens: u64,
}

pub const DEFAULT_COMPACTION_SETTINGS: CompactionSettings = CompactionSettings {
    enabled: true,
    reserve_tokens: 16_384,
    keep_recent_tokens: 20_000,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextUsageEstimate {
    pub tokens: u64,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutPointResult {
    pub first_kept_entry_index: usize,
    pub turn_start_index: Option<usize>,
    pub is_split_turn: bool,
}

#[derive(Debug, Clone)]
pub struct CompactionPreparation {
    pub first_kept_entry_id: String,
    pub messages_to_summarize: Vec<AgentMessage>,
    pub turn_prefix_messages: Vec<AgentMessage>,
    pub is_split_turn: bool,
    pub tokens_before: u64,
    pub previous_summary: Option<String>,
    pub file_ops: FileOperations,
    pub settings: CompactionSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompactionError {
    #[error("compaction aborted: {0}")]
    Aborted(String),
    #[error("summarization failed: {0}")]
    SummarizationFailed(String),
    #[error("invalid session: {0}")]
    InvalidSession(String),
    #[error("unknown compaction error: {0}")]
    Unknown(String),
}

fn safe_json_stringify(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[unserializable]".to_string())
}

fn extract_file_operations(
    messages: &[AgentMessage],
    entries: &[SessionTreeEntry],
    prev_compaction_index: Option<usize>,
) -> FileOperations {
    let mut file_ops = create_file_ops();

    if let Some(index) = prev_compaction_index {
        if let Some(SessionTreeEntry::Compaction { entry }) = entries.get(index) {
            if entry.from_hook != Some(true) {
                if let Some(details) = &entry.details {
                    if let Some(read_files) = details.get("readFiles").and_then(|v| v.as_array()) {
                        for file in read_files.iter().filter_map(|v| v.as_str()) {
                            file_ops.read.insert(file.to_string());
                        }
                    }
                    if let Some(modified_files) =
                        details.get("modifiedFiles").and_then(|v| v.as_array())
                    {
                        for file in modified_files.iter().filter_map(|v| v.as_str()) {
                            file_ops.edited.insert(file.to_string());
                        }
                    }
                }
            }
        }
    }

    for message in messages {
        extract_file_ops_from_message(message, &mut file_ops);
    }

    file_ops
}

fn get_message_from_entry(entry: &SessionTreeEntry) -> Option<AgentMessage> {
    match entry {
        SessionTreeEntry::Message { entry } => Some(entry.message.clone()),
        SessionTreeEntry::CustomMessage { entry } => {
            Some(AgentMessage::Custom(create_custom_message(
                entry.custom_type.clone(),
                entry.content.clone(),
                entry.display,
                entry.details.clone(),
                &entry.base.timestamp,
            )))
        }
        SessionTreeEntry::BranchSummary { entry } => {
            Some(AgentMessage::BranchSummary(create_branch_summary_message(
                entry.summary.clone(),
                entry.from_id.clone(),
                &entry.base.timestamp,
            )))
        }
        SessionTreeEntry::Compaction { entry } => Some(AgentMessage::CompactionSummary(
            create_compaction_summary_message(
                entry.summary.clone(),
                entry.tokens_before,
                &entry.base.timestamp,
            ),
        )),
        _ => None,
    }
}

fn get_message_from_entry_for_compaction(entry: &SessionTreeEntry) -> Option<AgentMessage> {
    match entry {
        SessionTreeEntry::Compaction { .. } => None,
        _ => get_message_from_entry(entry),
    }
}

pub fn calculate_context_tokens(usage: &Usage) -> u64 {
    if usage.total_tokens > 0 {
        usage.total_tokens
    } else {
        usage.input + usage.output + usage.cache_read + usage.cache_write
    }
}

fn get_assistant_usage(message: &AgentMessage) -> Option<Usage> {
    match message {
        AgentMessage::Assistant(message)
            if message.stop_reason != StopReason::Aborted
                && message.stop_reason != StopReason::Error =>
        {
            Some(message.usage.clone())
        }
        _ => None,
    }
}

pub fn get_last_assistant_usage(entries: &[SessionTreeEntry]) -> Option<Usage> {
    for entry in entries.iter().rev() {
        if let SessionTreeEntry::Message { entry } = entry {
            if let Some(usage) = get_assistant_usage(&entry.message) {
                return Some(usage);
            }
        }
    }
    None
}

fn get_last_assistant_usage_info(messages: &[AgentMessage]) -> Option<(Usage, usize)> {
    for (index, message) in messages.iter().enumerate().rev() {
        if let Some(usage) = get_assistant_usage(message) {
            return Some((usage, index));
        }
    }
    None
}

pub fn estimate_context_tokens(messages: &[AgentMessage]) -> ContextUsageEstimate {
    let Some((usage, index)) = get_last_assistant_usage_info(messages) else {
        let estimated = messages.iter().map(estimate_tokens).sum();
        return ContextUsageEstimate {
            tokens: estimated,
            usage_tokens: 0,
            trailing_tokens: estimated,
            last_usage_index: None,
        };
    };

    let usage_tokens = calculate_context_tokens(&usage);
    let trailing_tokens = messages
        .iter()
        .skip(index + 1)
        .map(estimate_tokens)
        .sum::<u64>();
    ContextUsageEstimate {
        tokens: usage_tokens + trailing_tokens,
        usage_tokens,
        trailing_tokens,
        last_usage_index: Some(index),
    }
}

pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: &CompactionSettings,
) -> bool {
    settings.enabled && context_tokens > context_window.saturating_sub(settings.reserve_tokens)
}

const ESTIMATED_IMAGE_CHARS: u64 = 4_800;

fn estimate_message_content_chars(content: &[MessageContent]) -> u64 {
    content
        .iter()
        .map(|block| match block {
            MessageContent::Text(text) => text.text.len() as u64,
            MessageContent::Image(_) => ESTIMATED_IMAGE_CHARS,
        })
        .sum()
}

fn estimate_custom_content_chars(content: &CustomMessageContent) -> u64 {
    match content {
        CustomMessageContent::Text(text) => text.len() as u64,
        CustomMessageContent::Blocks(blocks) => estimate_message_content_chars(blocks),
    }
}

pub fn estimate_tokens(message: &AgentMessage) -> u64 {
    let chars = match message {
        AgentMessage::User(message) => estimate_message_content_chars(&message.content),
        AgentMessage::Assistant(message) => message
            .content
            .iter()
            .map(|block| match block {
                AssistantContentBlock::Text(text) => text.text.len() as u64,
                AssistantContentBlock::Thinking(thinking) => thinking.thinking.len() as u64,
                AssistantContentBlock::ToolCall(tool_call) => {
                    (tool_call.name.len() + safe_json_stringify(&tool_call.arguments).len()) as u64
                }
            })
            .sum(),
        AgentMessage::Custom(message) => estimate_custom_content_chars(&message.content),
        AgentMessage::ToolResult(message) => estimate_message_content_chars(&message.content),
        AgentMessage::BashExecution(message) => {
            (message.command.len() + message.output.len()) as u64
        }
        AgentMessage::BranchSummary(message) => message.summary.len() as u64,
        AgentMessage::CompactionSummary(message) => message.summary.len() as u64,
    };

    ((chars as f64) / 4.0).ceil() as u64
}

fn find_valid_cut_points(
    entries: &[SessionTreeEntry],
    start_index: usize,
    end_index: usize,
) -> Vec<usize> {
    let mut cut_points = Vec::new();

    for (index, entry) in entries
        .iter()
        .enumerate()
        .skip(start_index)
        .take(end_index.saturating_sub(start_index))
    {
        match entry {
            SessionTreeEntry::Message { entry } => match &entry.message {
                AgentMessage::ToolResult(_) => {}
                AgentMessage::User(_)
                | AgentMessage::Assistant(_)
                | AgentMessage::BashExecution(_)
                | AgentMessage::Custom(_)
                | AgentMessage::BranchSummary(_)
                | AgentMessage::CompactionSummary(_) => cut_points.push(index),
            },
            SessionTreeEntry::BranchSummary { .. } | SessionTreeEntry::CustomMessage { .. } => {
                cut_points.push(index)
            }
            _ => {}
        }
    }

    cut_points
}

pub fn find_turn_start_index(
    entries: &[SessionTreeEntry],
    entry_index: usize,
    start_index: usize,
) -> Option<usize> {
    for index in (start_index..=entry_index).rev() {
        let entry = &entries[index];
        match entry {
            SessionTreeEntry::BranchSummary { .. } | SessionTreeEntry::CustomMessage { .. } => {
                return Some(index);
            }
            SessionTreeEntry::Message { entry } => match entry.message {
                AgentMessage::User(_) | AgentMessage::BashExecution(_) => return Some(index),
                _ => {}
            },
            _ => {}
        }
    }

    None
}

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
            turn_start_index: None,
            is_split_turn: false,
        };
    }

    let mut accumulated_tokens = 0_u64;
    let mut cut_index = cut_points[0];

    for index in (start_index..end_index).rev() {
        let SessionTreeEntry::Message { entry } = &entries[index] else {
            continue;
        };
        accumulated_tokens += estimate_tokens(&entry.message);
        if accumulated_tokens >= keep_recent_tokens {
            if let Some(found_cut_index) = cut_points.iter().find(|candidate| **candidate >= index)
            {
                cut_index = *found_cut_index;
            }
            break;
        }
    }

    while cut_index > start_index {
        match &entries[cut_index - 1] {
            SessionTreeEntry::Compaction { .. } | SessionTreeEntry::Message { .. } => break,
            _ => cut_index -= 1,
        }
    }

    let cut_entry = &entries[cut_index];
    let is_user_message = matches!(
        cut_entry,
        SessionTreeEntry::Message {
            entry: crate::harness::types::MessageEntry {
                message: AgentMessage::User(_),
                ..
            }
        }
    );
    let turn_start_index = if is_user_message {
        None
    } else {
        find_turn_start_index(entries, cut_index, start_index)
    };

    CutPointResult {
        first_kept_entry_index: cut_index,
        turn_start_index,
        is_split_turn: !is_user_message && turn_start_index.is_some(),
    }
}

pub const SUMMARIZATION_SYSTEM_PROMPT: &str = "You are a context summarization assistant. Your task is to read a conversation between a user and an AI assistant, then produce a structured summary following the exact format specified.\n\nDo NOT continue the conversation. Do NOT respond to any questions in the conversation. ONLY output the structured summary.";

const SUMMARIZATION_PROMPT: &str = "The messages above are a conversation to summarize. Create a structured context checkpoint summary that another LLM will use to continue the work.\n\nUse this EXACT format:\n\n## Goal\n[What is the user trying to accomplish? Can be multiple items if the session covers different tasks.]\n\n## Constraints & Preferences\n- [Any constraints, preferences, or requirements mentioned by user]\n- [Or \"(none)\" if none were mentioned]\n\n## Progress\n### Done\n- [x] [Completed tasks/changes]\n\n### In Progress\n- [ ] [Current work]\n\n### Blocked\n- [Issues preventing progress, if any]\n\n## Key Decisions\n- **[Decision]**: [Brief rationale]\n\n## Next Steps\n1. [Ordered list of what should happen next]\n\n## Critical Context\n- [Any data, examples, or references needed to continue]\n- [Or \"(none)\" if not applicable]\n\nKeep each section concise. Preserve exact file paths, function names, and error messages.";

const UPDATE_SUMMARIZATION_PROMPT: &str = "The messages above are NEW conversation messages to incorporate into the existing summary provided in <previous-summary> tags.\n\nUpdate the existing structured summary with new information. RULES:\n- PRESERVE all existing information from the previous summary\n- ADD new progress, decisions, and context from the new messages\n- UPDATE the Progress section: move items from \"In Progress\" to \"Done\" when completed\n- UPDATE \"Next Steps\" based on what was accomplished\n- PRESERVE exact file paths, function names, and error messages\n- If something is no longer relevant, you may remove it\n\nUse this EXACT format:\n\n## Goal\n[Preserve existing goals, add new ones if the task expanded]\n\n## Constraints & Preferences\n- [Preserve existing, add new ones discovered]\n\n## Progress\n### Done\n- [x] [Include previously done items AND newly completed items]\n\n### In Progress\n- [ ] [Current work - update based on progress]\n\n### Blocked\n- [Current blockers - remove if resolved]\n\n## Key Decisions\n- **[Decision]**: [Brief rationale] (preserve all previous, add new)\n\n## Next Steps\n1. [Update based on current state]\n\n## Critical Context\n- [Preserve important context, add new if needed]\n\nKeep each section concise. Preserve exact file paths, function names, and error messages.";

pub async fn generate_summary(
    current_messages: &[AgentMessage],
    model: Model,
    reserve_tokens: u64,
    api_key: String,
    headers: Option<std::collections::HashMap<String, String>>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    custom_instructions: Option<String>,
    previous_summary: Option<String>,
    thinking_level: Option<ThinkingLevel>,
) -> Result<String, CompactionError> {
    let max_tokens = std::cmp::min(
        ((reserve_tokens as f64) * 0.8).floor() as u64,
        if model.max_tokens > 0 {
            model.max_tokens
        } else {
            u64::MAX
        },
    );

    let mut base_prompt = if previous_summary.is_some() {
        UPDATE_SUMMARIZATION_PROMPT.to_string()
    } else {
        SUMMARIZATION_PROMPT.to_string()
    };
    if let Some(custom_instructions) = custom_instructions {
        base_prompt.push_str("\n\nAdditional focus: ");
        base_prompt.push_str(&custom_instructions);
    }

    let llm_messages = convert_to_llm(current_messages);
    let conversation_text = serialize_conversation(&llm_messages);
    let mut prompt_text = format!("<conversation>\n{conversation_text}\n</conversation>\n\n");
    if let Some(previous_summary) = previous_summary {
        prompt_text.push_str(&format!(
            "<previous-summary>\n{previous_summary}\n</previous-summary>\n\n"
        ));
    }
    prompt_text.push_str(&base_prompt);

    let context = hamr_ai::types::Context {
        system_prompt: Some(SUMMARIZATION_SYSTEM_PROMPT.to_string()),
        messages: vec![hamr_ai::types::Message::User(hamr_ai::types::UserMessage {
            role: hamr_ai::types::MessageRole::User,
            content: vec![MessageContent::Text(hamr_ai::types::TextContent {
                text: prompt_text,
                text_signature: None,
            })],
            timestamp: chrono::Utc::now(),
        })],
        tools: Vec::new(),
    };
    let mut options = SimpleStreamOptions::default();
    options.base.api_key = Some(api_key);
    options.base.headers = headers;
    options.base.max_tokens = Some(max_tokens);
    options.base.signal = signal;
    options.reasoning = thinking_level;

    let response = complete_simple(model, context, Some(options))
        .await
        .map_err(|error| CompactionError::SummarizationFailed(error.to_string()))?;
    match response.stop_reason {
        StopReason::Aborted => Err(CompactionError::Aborted(
            response
                .error_message
                .unwrap_or_else(|| "Summarization aborted".to_string()),
        )),
        StopReason::Error => Err(CompactionError::SummarizationFailed(
            response
                .error_message
                .map(|message| format!("Summarization failed: {message}"))
                .unwrap_or_else(|| "Summarization failed: Unknown error".to_string()),
        )),
        _ => Ok(response
            .content
            .iter()
            .filter_map(|content| match content {
                AssistantContentBlock::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")),
    }
}

pub fn prepare_compaction(
    path_entries: &[SessionTreeEntry],
    settings: &CompactionSettings,
) -> Result<Option<CompactionPreparation>, CompactionError> {
    if path_entries.is_empty()
        || matches!(
            path_entries.last(),
            Some(SessionTreeEntry::Compaction { .. })
        )
    {
        return Ok(None);
    }

    let prev_compaction_index = path_entries
        .iter()
        .rposition(|entry| matches!(entry, SessionTreeEntry::Compaction { .. }));

    let mut previous_summary = None;
    let mut boundary_start = 0_usize;
    if let Some(index) = prev_compaction_index {
        if let SessionTreeEntry::Compaction { entry } = &path_entries[index] {
            previous_summary = Some(entry.summary.clone());
            let first_kept_entry_index = path_entries
                .iter()
                .position(|candidate| candidate.id() == entry.first_kept_entry_id);
            boundary_start = first_kept_entry_index.unwrap_or(index + 1);
        }
    }
    let boundary_end = path_entries.len();

    let tokens_before =
        estimate_context_tokens(&build_session_context(path_entries).messages).tokens;
    let cut_point = find_cut_point(
        path_entries,
        boundary_start,
        boundary_end,
        settings.keep_recent_tokens,
    );
    let first_kept_entry = path_entries
        .get(cut_point.first_kept_entry_index)
        .ok_or_else(|| {
            CompactionError::InvalidSession("First kept entry is missing".to_string())
        })?;
    let first_kept_entry_id = first_kept_entry.id().to_string();

    let history_end = if cut_point.is_split_turn {
        cut_point
            .turn_start_index
            .unwrap_or(cut_point.first_kept_entry_index)
    } else {
        cut_point.first_kept_entry_index
    };

    let mut messages_to_summarize = Vec::new();
    for entry in path_entries.iter().take(history_end).skip(boundary_start) {
        if let Some(message) = get_message_from_entry_for_compaction(entry) {
            messages_to_summarize.push(message);
        }
    }

    let mut turn_prefix_messages = Vec::new();
    if cut_point.is_split_turn {
        for entry in path_entries
            .iter()
            .take(cut_point.first_kept_entry_index)
            .skip(
                cut_point
                    .turn_start_index
                    .unwrap_or(cut_point.first_kept_entry_index),
            )
        {
            if let Some(message) = get_message_from_entry_for_compaction(entry) {
                turn_prefix_messages.push(message);
            }
        }
    }

    let mut file_ops =
        extract_file_operations(&messages_to_summarize, path_entries, prev_compaction_index);
    if cut_point.is_split_turn {
        for message in &turn_prefix_messages {
            extract_file_ops_from_message(message, &mut file_ops);
        }
    }

    Ok(Some(CompactionPreparation {
        first_kept_entry_id,
        messages_to_summarize,
        turn_prefix_messages,
        is_split_turn: cut_point.is_split_turn,
        tokens_before,
        previous_summary,
        file_ops,
        settings: settings.clone(),
    }))
}

const TURN_PREFIX_SUMMARIZATION_PROMPT: &str = "This is the PREFIX of a turn that was too large to keep. The SUFFIX (recent work) is retained.\n\nSummarize the prefix to provide context for the retained suffix:\n\n## Original Request\n[What did the user ask for in this turn?]\n\n## Early Progress\n- [Key decisions and work done in the prefix]\n\n## Context for Suffix\n- [Information needed to understand the retained recent work]\n\nBe concise. Focus on what's needed to understand the kept suffix.";

async fn generate_turn_prefix_summary(
    messages: &[AgentMessage],
    model: Model,
    reserve_tokens: u64,
    api_key: String,
    headers: Option<std::collections::HashMap<String, String>>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    thinking_level: Option<ThinkingLevel>,
) -> Result<String, CompactionError> {
    let max_tokens = std::cmp::min(
        ((reserve_tokens as f64) * 0.5).floor() as u64,
        if model.max_tokens > 0 {
            model.max_tokens
        } else {
            u64::MAX
        },
    );
    let llm_messages = convert_to_llm(messages);
    let conversation_text = serialize_conversation(&llm_messages);
    let prompt_text = format!(
        "<conversation>\n{conversation_text}\n</conversation>\n\n{TURN_PREFIX_SUMMARIZATION_PROMPT}"
    );

    let context = hamr_ai::types::Context {
        system_prompt: Some(SUMMARIZATION_SYSTEM_PROMPT.to_string()),
        messages: vec![hamr_ai::types::Message::User(hamr_ai::types::UserMessage {
            role: hamr_ai::types::MessageRole::User,
            content: vec![MessageContent::Text(hamr_ai::types::TextContent {
                text: prompt_text,
                text_signature: None,
            })],
            timestamp: chrono::Utc::now(),
        })],
        tools: Vec::new(),
    };
    let mut options = SimpleStreamOptions::default();
    options.base.api_key = Some(api_key);
    options.base.headers = headers;
    options.base.max_tokens = Some(max_tokens);
    options.base.signal = signal;
    options.reasoning = thinking_level;

    let response = complete_simple(model, context, Some(options))
        .await
        .map_err(|error| CompactionError::SummarizationFailed(error.to_string()))?;
    match response.stop_reason {
        StopReason::Aborted => Err(CompactionError::Aborted(
            response
                .error_message
                .unwrap_or_else(|| "Turn prefix summarization aborted".to_string()),
        )),
        StopReason::Error => Err(CompactionError::SummarizationFailed(
            response
                .error_message
                .map(|message| format!("Turn prefix summarization failed: {message}"))
                .unwrap_or_else(|| "Turn prefix summarization failed: Unknown error".to_string()),
        )),
        _ => Ok(response
            .content
            .iter()
            .filter_map(|content| match content {
                AssistantContentBlock::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")),
    }
}

pub async fn compact(
    preparation: CompactionPreparation,
    model: Model,
    api_key: String,
    headers: Option<std::collections::HashMap<String, String>>,
    custom_instructions: Option<String>,
    signal: Option<tokio::sync::watch::Receiver<bool>>,
    thinking_level: Option<ThinkingLevel>,
) -> Result<CompactionResult, CompactionError> {
    if preparation.first_kept_entry_id.is_empty() {
        return Err(CompactionError::InvalidSession(
            "First kept entry has no UUID - session may need migration".to_string(),
        ));
    }

    let summary = if preparation.is_split_turn && !preparation.turn_prefix_messages.is_empty() {
        let history_result = if preparation.messages_to_summarize.is_empty() {
            Ok("No prior history.".to_string())
        } else {
            generate_summary(
                &preparation.messages_to_summarize,
                model.clone(),
                preparation.settings.reserve_tokens,
                api_key.clone(),
                headers.clone(),
                signal.clone(),
                custom_instructions,
                preparation.previous_summary.clone(),
                thinking_level,
            )
            .await
        }?;
        let turn_prefix_result = generate_turn_prefix_summary(
            &preparation.turn_prefix_messages,
            model,
            preparation.settings.reserve_tokens,
            api_key,
            headers,
            signal,
            thinking_level,
        )
        .await?;
        format!("{history_result}\n\n---\n\n**Turn Context (split turn):**\n\n{turn_prefix_result}")
    } else {
        generate_summary(
            &preparation.messages_to_summarize,
            model,
            preparation.settings.reserve_tokens,
            api_key,
            headers,
            signal,
            custom_instructions,
            preparation.previous_summary.clone(),
            thinking_level,
        )
        .await?
    };

    let file_lists = compute_file_lists(&preparation.file_ops);
    let summary = format!(
        "{summary}{}",
        format_file_operations(&file_lists.read_files, &file_lists.modified_files)
    );

    Ok(CompactionResult {
        summary,
        first_kept_entry_id: preparation.first_kept_entry_id,
        tokens_before: preparation.tokens_before,
        details: Some(CompactionDetails {
            read_files: file_lists.read_files,
            modified_files: file_lists.modified_files,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CompactionError, CompactionSettings, DEFAULT_COMPACTION_SETTINGS, calculate_context_tokens,
        compact, estimate_context_tokens, estimate_tokens, find_cut_point, find_turn_start_index,
        get_last_assistant_usage, prepare_compaction,
    };
    use crate::harness::session::session::build_session_context;
    use crate::harness::types::{
        CompactionEntry, MessageEntry, ModelChangeEntry, SessionTreeEntry, SessionTreeEntryBase,
        ThinkingLevelChangeEntry,
    };
    use crate::types::{AgentMessage, BranchSummaryMessage, CompactionSummaryMessage};
    use chrono::Utc;
    use hamr_ai::types::{
        AssistantContentBlock, AssistantMessage, MessageContent, MessageRole, StopReason,
        TextContent, ToolCall, ToolResultMessage, Usage, UsageCost, UserMessage,
    };

    fn create_mock_usage(input: u64, output: u64, cache_read: u64, cache_write: u64) -> Usage {
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

    fn create_user_message(text: &str) -> AgentMessage {
        AgentMessage::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(TextContent {
                text: text.to_string(),
                text_signature: None,
            })],
            timestamp: Utc::now(),
        })
    }

    fn create_assistant_message(text: &str, usage: Usage) -> AgentMessage {
        AgentMessage::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content: vec![AssistantContentBlock::Text(TextContent {
                text: text.to_string(),
                text_signature: None,
            })],
            api: "anthropic-messages".to_string(),
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-5".to_string(),
            response_model: None,
            response_id: None,
            usage,
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        })
    }

    fn create_message_entry(message: AgentMessage, parent_id: Option<String>) -> SessionTreeEntry {
        SessionTreeEntry::Message {
            entry: MessageEntry {
                base: SessionTreeEntryBase {
                    id: uuid::Uuid::now_v7().to_string(),
                    parent_id,
                    timestamp: Utc::now().to_rfc3339(),
                },
                message,
            },
        }
    }

    #[test]
    fn calculates_context_tokens_from_usage() {
        assert_eq!(
            calculate_context_tokens(&create_mock_usage(1000, 500, 200, 100)),
            1800
        );
    }

    #[test]
    fn estimates_supported_message_tokens() {
        let assistant = AgentMessage::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content: vec![
                AssistantContentBlock::Thinking(hamr_ai::types::ThinkingContent {
                    thinking: "reasoning".to_string(),
                    thinking_signature: None,
                    redacted: false,
                }),
                AssistantContentBlock::ToolCall(ToolCall {
                    id: "call-1".to_string(),
                    name: "read".to_string(),
                    arguments: serde_json::json!({"path":"file.ts"}),
                    thought_signature: None,
                }),
            ],
            api: "anthropic-messages".to_string(),
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-5".to_string(),
            response_model: None,
            response_id: None,
            usage: create_mock_usage(10, 5, 3, 2),
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        });

        let tool_result = AgentMessage::ToolResult(ToolResultMessage {
            role: MessageRole::ToolResult,
            tool_call_id: "call-1".to_string(),
            tool_name: "read".to_string(),
            content: vec![
                MessageContent::Text(TextContent {
                    text: "tool text".to_string(),
                    text_signature: None,
                }),
                MessageContent::Image(hamr_ai::types::ImageContent {
                    data: "abc".to_string(),
                    mime_type: "image/png".to_string(),
                }),
            ],
            details: None,
            is_error: false,
            timestamp: Utc::now(),
        });

        assert!(estimate_tokens(&create_user_message("plain user")) > 0);
        assert!(estimate_tokens(&assistant) > 0);
        assert!(estimate_tokens(&tool_result) > 1000);
        assert!(
            estimate_tokens(&AgentMessage::BranchSummary(BranchSummaryMessage {
                summary: "branch".to_string(),
                from_id: "x".to_string(),
                timestamp: 0,
            })) > 0
        );
        assert!(
            estimate_tokens(&AgentMessage::CompactionSummary(CompactionSummaryMessage {
                summary: "compact".to_string(),
                tokens_before: 123,
                timestamp: 0,
            })) > 0
        );
    }

    #[test]
    fn finds_cut_points_and_turn_starts() {
        let thinking = SessionTreeEntry::ThinkingLevelChange {
            entry: ThinkingLevelChangeEntry {
                base: SessionTreeEntryBase {
                    id: "thinking".to_string(),
                    parent_id: None,
                    timestamp: Utc::now().to_rfc3339(),
                },
                thinking_level: "high".to_string(),
            },
        };
        let model_change = SessionTreeEntry::ModelChange {
            entry: ModelChangeEntry {
                base: SessionTreeEntryBase {
                    id: "model".to_string(),
                    parent_id: Some("thinking".to_string()),
                    timestamp: Utc::now().to_rfc3339(),
                },
                provider: "openai".to_string(),
                model_id: "gpt-4".to_string(),
            },
        };
        assert_eq!(
            find_turn_start_index(&[thinking.clone(), model_change.clone()], 1, 0),
            None
        );

        let user = create_message_entry(create_user_message("user"), None);
        let assistant = create_message_entry(
            create_assistant_message("assistant", create_mock_usage(0, 100, 2000, 0)),
            Some(user.id().to_string()),
        );
        let result = find_cut_point(&[user.clone(), assistant], 0, 2, 1);
        assert_eq!(result.first_kept_entry_index, 1);
    }

    #[test]
    fn extracts_last_assistant_usage_and_estimates_context() {
        let usage = create_mock_usage(10, 5, 3, 2);
        let assistant = create_assistant_message("assistant", usage.clone());
        let user = create_user_message("tail");
        let entries = vec![
            create_message_entry(create_user_message("user"), None),
            create_message_entry(assistant.clone(), None),
        ];
        let last_usage = get_last_assistant_usage(&entries).unwrap();
        assert_eq!(last_usage.total_tokens, usage.total_tokens);
        let estimate = estimate_context_tokens(&[assistant, user]);
        assert_eq!(estimate.last_usage_index, Some(0));
        assert_eq!(estimate.usage_tokens, 20);
    }

    #[test]
    fn prepares_compaction_from_branch_entries() {
        let user1 = create_message_entry(create_user_message("one"), None);
        let assistant1 = create_message_entry(
            create_assistant_message("two", create_mock_usage(100, 50, 0, 0)),
            Some(user1.id().to_string()),
        );
        let user2 = create_message_entry(
            create_user_message("three"),
            Some(assistant1.id().to_string()),
        );
        let compaction_entry = SessionTreeEntry::Compaction {
            entry: CompactionEntry {
                base: SessionTreeEntryBase {
                    id: "compaction".to_string(),
                    parent_id: Some(user2.id().to_string()),
                    timestamp: Utc::now().to_rfc3339(),
                },
                summary: "old summary".to_string(),
                first_kept_entry_id: user2.id().to_string(),
                tokens_before: 1234,
                details: None,
                from_hook: None,
            },
        };
        let user3 =
            create_message_entry(create_user_message("four"), Some("compaction".to_string()));

        let entries = vec![
            user1,
            assistant1,
            user2.clone(),
            compaction_entry,
            user3.clone(),
        ];
        let preparation = prepare_compaction(&entries, &DEFAULT_COMPACTION_SETTINGS)
            .unwrap()
            .unwrap();
        assert!(!preparation.first_kept_entry_id.is_empty());
        assert_eq!(preparation.previous_summary.as_deref(), Some("old summary"));
        assert!(preparation.tokens_before > 0);
        let context = build_session_context(&entries);
        assert!(!context.messages.is_empty());
    }

    #[test]
    fn compaction_threshold_uses_settings() {
        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 10_000,
            keep_recent_tokens: 20_000,
        };
        assert!(super::should_compact(95_000, 100_000, &settings));
        assert!(!super::should_compact(89_000, 100_000, &settings));
    }

    #[test]
    fn should_compact_respects_disabled_flag() {
        // Even far over the threshold, a disabled config must never compact.
        let disabled = CompactionSettings {
            enabled: false,
            reserve_tokens: 10_000,
            keep_recent_tokens: 20_000,
        };
        assert!(!super::should_compact(999_000, 100_000, &disabled));
    }

    // ── P0-D: full compaction guard → summarize → no silent truncation ─────
    // Drives the async `compact()` path with the deterministic `faux`
    // summarizer (global dispatch via `complete_simple`). Proves the prefix is
    // replaced by a model summary while the recent tail is RETAINED (not
    // dropped), and that a summarizer failure surfaces as an error rather than
    // a silent truncation.

    fn message_text(message: &AgentMessage) -> String {
        match message {
            AgentMessage::User(m) => m
                .content
                .iter()
                .filter_map(|b| match b {
                    MessageContent::Text(t) => Some(t.text.as_str()),
                    _ => None,
                })
                .collect(),
            AgentMessage::Assistant(m) => m
                .content
                .iter()
                .filter_map(|b| match b {
                    AssistantContentBlock::Text(t) => Some(t.text.as_str()),
                    _ => None,
                })
                .collect(),
            _ => String::new(),
        }
    }

    fn build_conversation(texts: &[&str]) -> Vec<SessionTreeEntry> {
        let mut entries = Vec::new();
        let mut parent: Option<String> = None;
        for (i, t) in texts.iter().enumerate() {
            let message = if i % 2 == 0 {
                create_user_message(t)
            } else {
                create_assistant_message(t, create_mock_usage(50, 50, 0, 0))
            };
            let entry = create_message_entry(message, parent.clone());
            parent = Some(entry.id().to_string());
            entries.push(entry);
        }
        entries
    }

    #[tokio::test]
    async fn compact_summarizes_prefix_and_keeps_recent_tail() {
        use hamr_ai::providers::faux::{
            FauxAssistantMessageOptions, RegisterFauxProviderOptions, faux_assistant_message,
            register_faux_provider,
        };
        use hamr_ai::types::Api;

        let _registry_guard = crate::faux_registry_guard();
        let entries = build_conversation(&[
            "oldest user request",
            "old assistant reply",
            "middle user message",
            "middle assistant reply",
            "the most recent user question",
        ]);

        // keep_recent_tokens = 1 forces a cut that keeps only the tail and
        // summarizes everything before it.
        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 1_000,
            keep_recent_tokens: 1,
        };
        let prep = prepare_compaction(&entries, &settings)
            .expect("prepare ok")
            .expect("compaction needed");

        assert!(
            !prep.messages_to_summarize.is_empty(),
            "there must be a prefix to summarize"
        );
        assert!(prep.tokens_before > 0);

        // No silent truncation: the most-recent message is NOT in the summarized
        // prefix — it is part of the retained tail.
        let summarized: String = prep
            .messages_to_summarize
            .iter()
            .map(message_text)
            .collect::<Vec<_>>()
            .join("|");
        assert!(
            !summarized.contains("the most recent user question"),
            "recent tail must be kept, not folded into the summary: {summarized}"
        );

        let expected_kept = prep.first_kept_entry_id.clone();
        let expected_tokens_before = prep.tokens_before;

        // Deterministic summarizer under an isolated Api variant.
        let reg = std::sync::Arc::new(register_faux_provider(RegisterFauxProviderOptions {
            api_enum: Some(Api::MistralConversations),
            ..Default::default()
        }));
        reg.set_responses(vec![
            faux_assistant_message(
                "## Summary\nThe user made several requests.",
                FauxAssistantMessageOptions::default(),
            )
            .into(),
        ]);

        let result = compact(
            prep,
            reg.get_model(),
            "test-key".to_string(),
            None,
            None,
            None,
            None,
        )
        .await
        .expect("compaction must succeed");

        assert!(
            result.summary.contains("The user made several requests"),
            "summary must come from the model: {}",
            result.summary
        );
        assert_eq!(
            result.first_kept_entry_id, expected_kept,
            "kept boundary must be preserved through compaction"
        );
        assert_eq!(result.tokens_before, expected_tokens_before);
        reg.unregister();
    }

    #[tokio::test]
    async fn compact_propagates_summarizer_failure_without_truncating() {
        use hamr_ai::providers::faux::{
            FauxAssistantMessageOptions, RegisterFauxProviderOptions, faux_assistant_message,
            register_faux_provider,
        };
        use hamr_ai::types::Api;

        let _registry_guard = crate::faux_registry_guard();
        let entries = build_conversation(&[
            "first user message",
            "first assistant reply",
            "second user message",
            "second assistant reply",
            "latest user message",
        ]);
        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 1_000,
            keep_recent_tokens: 1,
        };
        let prep = prepare_compaction(&entries, &settings)
            .expect("prepare ok")
            .expect("compaction needed");

        let reg = std::sync::Arc::new(register_faux_provider(RegisterFauxProviderOptions {
            api_enum: Some(Api::GoogleVertex),
            ..Default::default()
        }));
        reg.set_responses(vec![
            faux_assistant_message(
                "partial",
                FauxAssistantMessageOptions {
                    stop_reason: Some(StopReason::Error),
                    error_message: Some("summarizer upstream down".to_string()),
                    ..Default::default()
                },
            )
            .into(),
        ]);

        let err = compact(
            prep,
            reg.get_model(),
            "test-key".to_string(),
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("a summarizer failure must surface as an error, not a silent cut");
        assert!(
            matches!(err, CompactionError::SummarizationFailed(_)),
            "expected SummarizationFailed, got {err:?}"
        );
        reg.unregister();
    }
}

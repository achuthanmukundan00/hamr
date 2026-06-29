//! Port of `packages/agent/src/harness/compaction/branch-summarization.ts`.

use crate::harness::compaction::compaction::{SUMMARIZATION_SYSTEM_PROMPT, estimate_tokens};
use crate::harness::compaction::utils::{
    FileOperations, compute_file_lists, create_file_ops, extract_file_ops_from_message,
    format_file_operations, serialize_conversation,
};
use crate::harness::messages::{
    convert_to_llm, create_branch_summary_message, create_compaction_summary_message,
    create_custom_message,
};
use crate::harness::session::session::Session;
use crate::harness::types::SessionTreeEntry;
use crate::types::AgentMessage;
use hamr_ai::stream::complete_simple;
use hamr_ai::types::{
    AssistantContentBlock, MessageContent, MessageRole, Model, SimpleStreamOptions, StopReason,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSummaryDetails {
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BranchPreparation {
    pub messages: Vec<AgentMessage>,
    pub file_ops: FileOperations,
    pub total_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct CollectEntriesResult {
    pub entries: Vec<SessionTreeEntry>,
    pub common_ancestor_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSummaryResult {
    pub summary: String,
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GenerateBranchSummaryOptions {
    pub model: Model,
    pub api_key: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub signal: Option<tokio::sync::watch::Receiver<bool>>,
    pub custom_instructions: Option<String>,
    pub replace_instructions: bool,
    pub reserve_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BranchSummaryError {
    #[error("branch summary aborted: {0}")]
    Aborted(String),
    #[error("branch summary failed: {0}")]
    SummarizationFailed(String),
    #[error("invalid session: {0}")]
    InvalidSession(String),
}

pub async fn collect_entries_for_branch_summary<TMetadata: Clone + Send + Sync + 'static>(
    session: &Session<TMetadata>,
    old_leaf_id: Option<String>,
    target_id: &str,
) -> Result<CollectEntriesResult, BranchSummaryError> {
    let Some(old_leaf_id) = old_leaf_id else {
        return Ok(CollectEntriesResult {
            entries: Vec::new(),
            common_ancestor_id: None,
        });
    };

    let old_path = session
        .get_branch(Some(old_leaf_id.clone()))
        .await
        .map_err(|error| BranchSummaryError::InvalidSession(error.message))?;
    let old_path_ids = old_path
        .iter()
        .map(|entry| entry.id().to_string())
        .collect::<std::collections::HashSet<_>>();
    let target_path = session
        .get_branch(Some(target_id.to_string()))
        .await
        .map_err(|error| BranchSummaryError::InvalidSession(error.message))?;

    let common_ancestor_id = target_path
        .iter()
        .rev()
        .find(|entry| old_path_ids.contains(entry.id()))
        .map(|entry| entry.id().to_string());

    let mut entries = Vec::new();
    let mut current = Some(old_leaf_id);
    while let Some(current_id) = current {
        if common_ancestor_id.as_deref() == Some(current_id.as_str()) {
            break;
        }
        let entry = session
            .get_entry(&current_id)
            .await
            .map_err(|error| BranchSummaryError::InvalidSession(error.message))?
            .ok_or_else(|| {
                BranchSummaryError::InvalidSession(format!("Entry {current_id} not found"))
            })?;
        current = entry.parent_id().map(ToOwned::to_owned);
        entries.push(entry);
    }
    entries.reverse();

    Ok(CollectEntriesResult {
        entries,
        common_ancestor_id,
    })
}

fn get_message_from_entry(entry: &SessionTreeEntry) -> Option<AgentMessage> {
    match entry {
        SessionTreeEntry::Message { entry } => match &entry.message {
            AgentMessage::ToolResult(_) => None,
            message => Some(message.clone()),
        },
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

pub fn prepare_branch_entries(
    entries: &[SessionTreeEntry],
    token_budget: u64,
) -> BranchPreparation {
    let mut messages = Vec::new();
    let mut file_ops = create_file_ops();
    let mut total_tokens = 0_u64;

    for entry in entries {
        if let SessionTreeEntry::BranchSummary { entry } = entry {
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

    for entry in entries.iter().rev() {
        let Some(message) = get_message_from_entry(entry) else {
            continue;
        };
        extract_file_ops_from_message(&message, &mut file_ops);

        let tokens = estimate_tokens(&message);
        if token_budget > 0 && total_tokens + tokens > token_budget {
            if matches!(
                entry,
                SessionTreeEntry::Compaction { .. } | SessionTreeEntry::BranchSummary { .. }
            ) && total_tokens < ((token_budget as f64) * 0.9) as u64
            {
                messages.insert(0, message);
                total_tokens += tokens;
            }
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

const BRANCH_SUMMARY_PREAMBLE: &str = "The user explored a different conversation branch before returning here.\nSummary of that exploration:\n\n";

const BRANCH_SUMMARY_PROMPT: &str = "Create a structured summary of this conversation branch for context when returning later.\n\nUse this EXACT format:\n\n## Goal\n[What was the user trying to accomplish in this branch?]\n\n## Constraints & Preferences\n- [Any constraints, preferences, or requirements mentioned]\n- [Or \"(none)\" if none were mentioned]\n\n## Progress\n### Done\n- [x] [Completed tasks/changes]\n\n### In Progress\n- [ ] [Work that was started but not finished]\n\n### Blocked\n- [Issues preventing progress, if any]\n\n## Key Decisions\n- **[Decision]**: [Brief rationale]\n\n## Next Steps\n1. [What should happen next to continue this work]\n\nKeep each section concise. Preserve exact file paths, function names, and error messages.";

pub async fn generate_branch_summary(
    entries: &[SessionTreeEntry],
    options: GenerateBranchSummaryOptions,
) -> Result<BranchSummaryResult, BranchSummaryError> {
    let context_window = if options.model.context_window > 0 {
        options.model.context_window
    } else {
        128_000
    };
    let token_budget = context_window.saturating_sub(options.reserve_tokens);
    let preparation = prepare_branch_entries(entries, token_budget);

    if preparation.messages.is_empty() {
        return Ok(BranchSummaryResult {
            summary: "No content to summarize".to_string(),
            read_files: Vec::new(),
            modified_files: Vec::new(),
        });
    }

    let llm_messages = convert_to_llm(&preparation.messages);
    let conversation_text = serialize_conversation(&llm_messages);
    let instructions = if options.replace_instructions {
        options
            .custom_instructions
            .clone()
            .unwrap_or_else(|| BRANCH_SUMMARY_PROMPT.to_string())
    } else if let Some(custom_instructions) = &options.custom_instructions {
        format!("{BRANCH_SUMMARY_PROMPT}\n\nAdditional focus: {custom_instructions}")
    } else {
        BRANCH_SUMMARY_PROMPT.to_string()
    };
    let prompt_text =
        format!("<conversation>\n{conversation_text}\n</conversation>\n\n{instructions}");

    let context = hamr_ai::types::Context {
        system_prompt: Some(SUMMARIZATION_SYSTEM_PROMPT.to_string()),
        messages: vec![hamr_ai::types::Message::User(hamr_ai::types::UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(hamr_ai::types::TextContent {
                text: prompt_text,
                text_signature: None,
            })],
            timestamp: chrono::Utc::now(),
        })],
        tools: Vec::new(),
    };
    let mut stream_options = SimpleStreamOptions::default();
    stream_options.base.api_key = Some(options.api_key);
    stream_options.base.headers = options.headers;
    stream_options.base.max_tokens = Some(2048);
    stream_options.base.signal = options.signal;

    let response = complete_simple(options.model, context, Some(stream_options))
        .await
        .map_err(|error| BranchSummaryError::SummarizationFailed(error.to_string()))?;
    let summary_body = match response.stop_reason {
        StopReason::Aborted => {
            return Err(BranchSummaryError::Aborted(
                response
                    .error_message
                    .unwrap_or_else(|| "Branch summary aborted".to_string()),
            ));
        }
        StopReason::Error => {
            return Err(BranchSummaryError::SummarizationFailed(
                response
                    .error_message
                    .map(|message| format!("Branch summary failed: {message}"))
                    .unwrap_or_else(|| "Branch summary failed: Unknown error".to_string()),
            ));
        }
        _ => response
            .content
            .iter()
            .filter_map(|content| match content {
                AssistantContentBlock::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    };

    let file_lists = compute_file_lists(&preparation.file_ops);
    Ok(BranchSummaryResult {
        summary: format!(
            "{BRANCH_SUMMARY_PREAMBLE}{summary_body}{}",
            format_file_operations(&file_lists.read_files, &file_lists.modified_files)
        ),
        read_files: file_lists.read_files,
        modified_files: file_lists.modified_files,
    })
}

//! Port of `packages/agent/src/harness/session/session.ts`.

use crate::harness::messages::{
    create_branch_summary_message, create_compaction_summary_message, create_custom_message,
};
use crate::harness::types::{
    ActiveToolsChangeEntry, BranchSummaryEntry, CompactionEntry, ModelChangeEntry, SessionContext,
    SessionEntryType, SessionError, SessionErrorCode, SessionInfoEntry, SessionModelRef,
    SessionStorage, SessionTreeEntry, SessionTreeEntryBase, ThinkingLevelChangeEntry,
};
use crate::types::{AgentMessage, CustomMessageContent};
use std::sync::Arc;

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
        SessionTreeEntry::BranchSummary { entry } if !entry.summary.is_empty() => {
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

fn append_entry_message(messages: &mut Vec<AgentMessage>, entry: &SessionTreeEntry) {
    if let Some(message) = get_message_from_entry(entry) {
        messages.push(message);
    }
}

pub fn build_session_context(path_entries: &[SessionTreeEntry]) -> SessionContext {
    let mut thinking_level = "off".to_string();
    let mut model = None;
    let mut active_tool_names = None;
    let mut compaction: Option<CompactionEntry> = None;

    for entry in path_entries {
        match entry {
            SessionTreeEntry::ThinkingLevelChange { entry } => {
                thinking_level = entry.thinking_level.clone();
            }
            SessionTreeEntry::ModelChange { entry } => {
                model = Some(SessionModelRef {
                    provider: entry.provider.clone(),
                    model_id: entry.model_id.clone(),
                });
            }
            SessionTreeEntry::Message { entry } => {
                if let AgentMessage::Assistant(message) = &entry.message {
                    model = Some(SessionModelRef {
                        provider: message.provider.clone(),
                        model_id: message.model.clone(),
                    });
                }
            }
            SessionTreeEntry::ActiveToolsChange { entry } => {
                active_tool_names = Some(entry.active_tool_names.clone());
            }
            SessionTreeEntry::Compaction { entry } => {
                compaction = Some(entry.clone());
            }
            _ => {}
        }
    }

    let mut messages = Vec::new();

    if let Some(compaction) = compaction {
        messages.push(AgentMessage::CompactionSummary(
            create_compaction_summary_message(
                compaction.summary,
                compaction.tokens_before,
                &compaction.base.timestamp,
            ),
        ));
        let compaction_idx = path_entries
            .iter()
            .position(|entry| matches!(entry, SessionTreeEntry::Compaction { entry: e } if e.base.id == compaction.base.id))
            .unwrap_or(0);
        let mut found_first_kept = false;
        for entry in path_entries.iter().take(compaction_idx) {
            if entry.id() == compaction.first_kept_entry_id {
                found_first_kept = true;
            }
            if found_first_kept {
                append_entry_message(&mut messages, entry);
            }
        }
        for entry in path_entries.iter().skip(compaction_idx + 1) {
            append_entry_message(&mut messages, entry);
        }
    } else {
        for entry in path_entries {
            append_entry_message(&mut messages, entry);
        }
    }

    SessionContext {
        messages,
        thinking_level,
        model,
        active_tool_names,
    }
}

#[derive(Clone)]
pub struct Session<TMetadata> {
    storage: Arc<dyn SessionStorage<TMetadata>>,
}

#[cfg(test)]
mod tests {
    use super::Session;
    use crate::harness::session::memory_storage::InMemorySessionStorage;
    use crate::harness::types::SessionMetadata;
    use crate::types::AgentMessage;
    use chrono::Utc;
    use hamr_ai::types::{
        AssistantMessage, MessageContent, MessageRole, StopReason, TextContent, Usage, UsageCost,
        UserMessage,
    };
    use std::sync::Arc;

    fn user_message(text: &str) -> AgentMessage {
        AgentMessage::User(UserMessage {
            role: MessageRole::User,
            content: vec![MessageContent::Text(TextContent {
                text: text.to_string(),
                text_signature: None,
            })],
            timestamp: Utc::now(),
        })
    }

    fn assistant_message(text: &str) -> AgentMessage {
        AgentMessage::Assistant(AssistantMessage {
            role: MessageRole::Assistant,
            content: vec![hamr_ai::types::AssistantContentBlock::Text(TextContent {
                text: text.to_string(),
                text_signature: None,
            })],
            api: "anthropic-messages".to_string(),
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-5".to_string(),
            response_model: None,
            response_id: None,
            usage: Usage {
                input: 0,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: None,
                total_tokens: 0,
                cost: UsageCost {
                    input: 0.0,
                    output: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    total: 0.0,
                },
            },
            stop_reason: StopReason::Stop,
            error_message: None,
            diagnostics: None,
            timestamp: Utc::now(),
        })
    }

    #[tokio::test]
    async fn appends_messages_and_builds_context() {
        let storage = Arc::new(InMemorySessionStorage::<SessionMetadata>::new(None, None).unwrap());
        let session = Session::new(storage);
        session.append_message(user_message("one")).await.unwrap();
        session
            .append_message(assistant_message("two"))
            .await
            .unwrap();
        let context = session.build_context().await.unwrap();
        assert_eq!(context.messages.len(), 2);
        assert!(matches!(context.messages[0], AgentMessage::User(_)));
        assert!(matches!(context.messages[1], AgentMessage::Assistant(_)));
    }

    #[tokio::test]
    async fn supports_branching_and_compaction_summary() {
        let storage = Arc::new(InMemorySessionStorage::<SessionMetadata>::new(None, None).unwrap());
        let session = Session::new(storage);
        let user1 = session.append_message(user_message("one")).await.unwrap();
        session
            .append_message(assistant_message("two"))
            .await
            .unwrap();
        let user2 = session.append_message(user_message("three")).await.unwrap();
        session
            .append_message(assistant_message("four"))
            .await
            .unwrap();
        session
            .append_compaction("summary".to_string(), user2, 1234, None, None)
            .await
            .unwrap();
        session.append_message(user_message("five")).await.unwrap();
        let context = session.build_context().await.unwrap();
        assert!(matches!(
            context.messages.first(),
            Some(AgentMessage::CompactionSummary(_))
        ));

        session.move_to(Some(user1.clone()), None).await.unwrap();
        session
            .append_message(assistant_message("branched"))
            .await
            .unwrap();
        let branch = session.get_branch(None).await.unwrap();
        assert!(branch.iter().any(|entry| entry.id() == user1));
    }
}

impl<TMetadata: Clone + Send + Sync + 'static> Session<TMetadata> {
    pub fn new(storage: Arc<dyn SessionStorage<TMetadata>>) -> Self {
        Self { storage }
    }

    pub async fn get_metadata(&self) -> Result<TMetadata, SessionError> {
        self.storage.get_metadata().await
    }

    pub fn get_storage(&self) -> Arc<dyn SessionStorage<TMetadata>> {
        Arc::clone(&self.storage)
    }

    pub async fn get_leaf_id(&self) -> Result<Option<String>, SessionError> {
        self.storage.get_leaf_id().await
    }

    pub async fn get_entry(&self, id: &str) -> Result<Option<SessionTreeEntry>, SessionError> {
        self.storage.get_entry(id).await
    }

    pub async fn get_entries(&self) -> Result<Vec<SessionTreeEntry>, SessionError> {
        self.storage.get_entries().await
    }

    pub async fn get_branch(
        &self,
        from_id: Option<String>,
    ) -> Result<Vec<SessionTreeEntry>, SessionError> {
        let leaf_id = match from_id {
            Some(id) => Some(id),
            None => self.storage.get_leaf_id().await?,
        };
        self.storage.get_path_to_root(leaf_id).await
    }

    pub async fn build_context(&self) -> Result<SessionContext, SessionError> {
        Ok(build_session_context(&self.get_branch(None).await?))
    }

    pub async fn get_label(&self, id: &str) -> Result<Option<String>, SessionError> {
        self.storage.get_label(id).await
    }

    pub async fn get_session_name(&self) -> Result<Option<String>, SessionError> {
        let entries = self
            .storage
            .find_entries(SessionEntryType::SessionInfo)
            .await?;
        Ok(entries.iter().rev().find_map(|entry| match entry {
            SessionTreeEntry::SessionInfo { entry } => entry
                .name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned),
            _ => None,
        }))
    }

    async fn append_typed_entry(&self, entry: SessionTreeEntry) -> Result<String, SessionError> {
        let id = entry.id().to_string();
        self.storage.append_entry(entry).await?;
        Ok(id)
    }

    pub async fn append_message(&self, message: AgentMessage) -> Result<String, SessionError> {
        self.append_typed_entry(SessionTreeEntry::Message {
            entry: crate::harness::types::MessageEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                message,
            },
        })
        .await
    }

    pub async fn append_thinking_level_change(
        &self,
        thinking_level: String,
    ) -> Result<String, SessionError> {
        self.append_typed_entry(SessionTreeEntry::ThinkingLevelChange {
            entry: ThinkingLevelChangeEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                thinking_level,
            },
        })
        .await
    }

    pub async fn append_model_change(
        &self,
        provider: String,
        model_id: String,
    ) -> Result<String, SessionError> {
        self.append_typed_entry(SessionTreeEntry::ModelChange {
            entry: ModelChangeEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                provider,
                model_id,
            },
        })
        .await
    }

    pub async fn append_active_tools_change(
        &self,
        active_tool_names: Vec<String>,
    ) -> Result<String, SessionError> {
        self.append_typed_entry(SessionTreeEntry::ActiveToolsChange {
            entry: ActiveToolsChangeEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                active_tool_names,
            },
        })
        .await
    }

    pub async fn append_compaction(
        &self,
        summary: String,
        first_kept_entry_id: String,
        tokens_before: u64,
        details: Option<serde_json::Value>,
        from_hook: Option<bool>,
    ) -> Result<String, SessionError> {
        self.append_typed_entry(SessionTreeEntry::Compaction {
            entry: CompactionEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                summary,
                first_kept_entry_id,
                tokens_before,
                details,
                from_hook,
            },
        })
        .await
    }

    pub async fn append_custom_entry(
        &self,
        custom_type: String,
        data: Option<serde_json::Value>,
    ) -> Result<String, SessionError> {
        self.append_typed_entry(SessionTreeEntry::Custom {
            entry: crate::harness::types::CustomEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                custom_type,
                data,
            },
        })
        .await
    }

    pub async fn append_custom_message_entry(
        &self,
        custom_type: String,
        content: CustomMessageContent,
        display: bool,
        details: Option<serde_json::Value>,
    ) -> Result<String, SessionError> {
        self.append_typed_entry(SessionTreeEntry::CustomMessage {
            entry: crate::harness::types::CustomMessageEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                custom_type,
                content,
                display,
                details,
            },
        })
        .await
    }

    pub async fn append_label(
        &self,
        target_id: String,
        label: Option<String>,
    ) -> Result<String, SessionError> {
        if self.storage.get_entry(&target_id).await?.is_none() {
            return Err(SessionError::new(
                SessionErrorCode::NotFound,
                format!("Entry {target_id} not found"),
            ));
        }
        self.append_typed_entry(SessionTreeEntry::Label {
            entry: crate::harness::types::LabelEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                target_id,
                label,
            },
        })
        .await
    }

    pub async fn append_session_name(&self, name: String) -> Result<String, SessionError> {
        self.append_typed_entry(SessionTreeEntry::SessionInfo {
            entry: SessionInfoEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: self.storage.get_leaf_id().await?,
                    timestamp: super::repo_utils::create_timestamp(),
                },
                name: Some(name.trim().to_string()),
            },
        })
        .await
    }

    pub async fn move_to(
        &self,
        entry_id: Option<String>,
        summary: Option<(String, Option<serde_json::Value>, Option<bool>)>,
    ) -> Result<Option<String>, SessionError> {
        if let Some(ref entry_id) = entry_id {
            if self.storage.get_entry(entry_id).await?.is_none() {
                return Err(SessionError::new(
                    SessionErrorCode::NotFound,
                    format!("Entry {entry_id} not found"),
                ));
            }
        }
        self.storage.set_leaf_id(entry_id.clone()).await?;
        let Some((summary_text, details, from_hook)) = summary else {
            return Ok(None);
        };
        self.append_typed_entry(SessionTreeEntry::BranchSummary {
            entry: BranchSummaryEntry {
                base: SessionTreeEntryBase {
                    id: self.storage.create_entry_id().await?,
                    parent_id: entry_id.clone(),
                    timestamp: super::repo_utils::create_timestamp(),
                },
                from_id: entry_id.unwrap_or_else(|| "root".to_string()),
                summary: summary_text,
                details,
                from_hook,
            },
        })
        .await
        .map(Some)
    }
}

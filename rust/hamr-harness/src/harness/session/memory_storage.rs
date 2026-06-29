//! Port of `packages/agent/src/harness/session/memory-storage.ts`.

use crate::harness::types::{
    LeafEntry, SessionEntryType, SessionError, SessionErrorCode, SessionMetadata, SessionStorage,
    SessionTreeEntry, SessionTreeEntryBase,
};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::Mutex;

fn update_label_cache(labels_by_id: &mut HashMap<String, String>, entry: &SessionTreeEntry) {
    if let SessionTreeEntry::Label { entry } = entry {
        let label = entry.label.as_deref().map(str::trim).unwrap_or_default();
        if label.is_empty() {
            labels_by_id.remove(&entry.target_id);
        } else {
            labels_by_id.insert(entry.target_id.clone(), label.to_string());
        }
    }
}

fn build_labels_by_id(entries: &[SessionTreeEntry]) -> HashMap<String, String> {
    let mut labels_by_id = HashMap::new();
    for entry in entries {
        update_label_cache(&mut labels_by_id, entry);
    }
    labels_by_id
}

fn generate_entry_id(by_id: &HashMap<String, SessionTreeEntry>) -> String {
    for _ in 0..100 {
        let id = super::uuid::uuidv7().chars().take(8).collect::<String>();
        if !by_id.contains_key(&id) {
            return id;
        }
    }
    super::uuid::uuidv7()
}

fn leaf_id_after_entry(entry: &SessionTreeEntry) -> Option<String> {
    match entry {
        SessionTreeEntry::Leaf { entry } => entry.target_id.clone(),
        _ => Some(entry.id().to_string()),
    }
}

struct State {
    entries: Vec<SessionTreeEntry>,
    by_id: HashMap<String, SessionTreeEntry>,
    labels_by_id: HashMap<String, String>,
    leaf_id: Option<String>,
}

pub struct InMemorySessionStorage<TMetadata = SessionMetadata> {
    metadata: TMetadata,
    state: Mutex<State>,
}

impl<TMetadata: Clone> InMemorySessionStorage<TMetadata> {
    pub fn new(
        entries: Option<Vec<SessionTreeEntry>>,
        metadata: Option<TMetadata>,
    ) -> Result<Self, SessionError>
    where
        TMetadata: From<SessionMetadata>,
    {
        let entries = entries.unwrap_or_default();
        let by_id = entries
            .iter()
            .cloned()
            .map(|entry| (entry.id().to_string(), entry))
            .collect::<HashMap<_, _>>();
        let labels_by_id = build_labels_by_id(&entries);
        let mut leaf_id = None;
        for entry in &entries {
            leaf_id = leaf_id_after_entry(entry);
        }
        if let Some(ref leaf_id_value) = leaf_id {
            if !by_id.contains_key(leaf_id_value) {
                return Err(SessionError::new(
                    SessionErrorCode::InvalidSession,
                    format!("Entry {leaf_id_value} not found"),
                ));
            }
        }

        Ok(Self {
            metadata: metadata.unwrap_or_else(|| {
                SessionMetadata {
                    id: super::repo_utils::create_session_id(),
                    created_at: super::repo_utils::create_timestamp(),
                }
                .into()
            }),
            state: Mutex::new(State {
                entries,
                by_id,
                labels_by_id,
                leaf_id,
            }),
        })
    }
}

impl InMemorySessionStorage<SessionMetadata> {
    pub fn default_storage() -> Result<Self, SessionError> {
        Self::new(None, None)
    }
}

#[async_trait]
impl<TMetadata> SessionStorage<TMetadata> for InMemorySessionStorage<TMetadata>
where
    TMetadata: Clone + Send + Sync + 'static,
{
    async fn get_metadata(&self) -> Result<TMetadata, SessionError> {
        Ok(self.metadata.clone())
    }

    async fn get_leaf_id(&self) -> Result<Option<String>, SessionError> {
        let state = self.state.lock().await;
        if let Some(ref leaf_id) = state.leaf_id {
            if !state.by_id.contains_key(leaf_id) {
                return Err(SessionError::new(
                    SessionErrorCode::InvalidSession,
                    format!("Entry {leaf_id} not found"),
                ));
            }
        }
        Ok(state.leaf_id.clone())
    }

    async fn set_leaf_id(&self, leaf_id: Option<String>) -> Result<(), SessionError> {
        let mut state = self.state.lock().await;
        if let Some(ref leaf_id) = leaf_id {
            if !state.by_id.contains_key(leaf_id) {
                return Err(SessionError::new(
                    SessionErrorCode::NotFound,
                    format!("Entry {leaf_id} not found"),
                ));
            }
        }

        let entry = SessionTreeEntry::Leaf {
            entry: LeafEntry {
                base: SessionTreeEntryBase {
                    id: generate_entry_id(&state.by_id),
                    parent_id: state.leaf_id.clone(),
                    timestamp: super::repo_utils::create_timestamp(),
                },
                target_id: leaf_id.clone(),
            },
        };
        state.by_id.insert(entry.id().to_string(), entry.clone());
        state.entries.push(entry);
        state.leaf_id = leaf_id;
        Ok(())
    }

    async fn create_entry_id(&self) -> Result<String, SessionError> {
        let state = self.state.lock().await;
        Ok(generate_entry_id(&state.by_id))
    }

    async fn append_entry(&self, entry: SessionTreeEntry) -> Result<(), SessionError> {
        let mut state = self.state.lock().await;
        update_label_cache(&mut state.labels_by_id, &entry);
        state.leaf_id = leaf_id_after_entry(&entry);
        state.by_id.insert(entry.id().to_string(), entry.clone());
        state.entries.push(entry);
        Ok(())
    }

    async fn get_entry(&self, id: &str) -> Result<Option<SessionTreeEntry>, SessionError> {
        let state = self.state.lock().await;
        Ok(state.by_id.get(id).cloned())
    }

    async fn find_entries(
        &self,
        entry_type: SessionEntryType,
    ) -> Result<Vec<SessionTreeEntry>, SessionError> {
        let state = self.state.lock().await;
        Ok(state
            .entries
            .iter()
            .filter(|entry| match entry_type {
                SessionEntryType::Message => matches!(entry, SessionTreeEntry::Message { .. }),
                SessionEntryType::ThinkingLevelChange => {
                    matches!(entry, SessionTreeEntry::ThinkingLevelChange { .. })
                }
                SessionEntryType::ModelChange => {
                    matches!(entry, SessionTreeEntry::ModelChange { .. })
                }
                SessionEntryType::ActiveToolsChange => {
                    matches!(entry, SessionTreeEntry::ActiveToolsChange { .. })
                }
                SessionEntryType::Compaction => {
                    matches!(entry, SessionTreeEntry::Compaction { .. })
                }
                SessionEntryType::BranchSummary => {
                    matches!(entry, SessionTreeEntry::BranchSummary { .. })
                }
                SessionEntryType::Custom => matches!(entry, SessionTreeEntry::Custom { .. }),
                SessionEntryType::CustomMessage => {
                    matches!(entry, SessionTreeEntry::CustomMessage { .. })
                }
                SessionEntryType::Label => matches!(entry, SessionTreeEntry::Label { .. }),
                SessionEntryType::SessionInfo => {
                    matches!(entry, SessionTreeEntry::SessionInfo { .. })
                }
                SessionEntryType::Leaf => matches!(entry, SessionTreeEntry::Leaf { .. }),
            })
            .cloned()
            .collect())
    }

    async fn get_label(&self, id: &str) -> Result<Option<String>, SessionError> {
        let state = self.state.lock().await;
        Ok(state.labels_by_id.get(id).cloned())
    }

    async fn get_path_to_root(
        &self,
        leaf_id: Option<String>,
    ) -> Result<Vec<SessionTreeEntry>, SessionError> {
        let state = self.state.lock().await;
        let Some(mut current_id) = leaf_id else {
            return Ok(Vec::new());
        };

        let mut path = Vec::new();
        loop {
            let Some(current) = state.by_id.get(&current_id).cloned() else {
                return Err(SessionError::new(
                    SessionErrorCode::NotFound,
                    format!("Entry {current_id} not found"),
                ));
            };
            path.push(current.clone());
            let Some(parent_id) = current.parent_id().map(ToOwned::to_owned) else {
                break;
            };
            if !state.by_id.contains_key(&parent_id) {
                return Err(SessionError::new(
                    SessionErrorCode::InvalidSession,
                    format!("Entry {parent_id} not found"),
                ));
            }
            current_id = parent_id;
        }
        path.reverse();
        Ok(path)
    }

    async fn get_entries(&self) -> Result<Vec<SessionTreeEntry>, SessionError> {
        let state = self.state.lock().await;
        Ok(state.entries.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::InMemorySessionStorage;
    use crate::harness::types::{
        LabelEntry, SessionEntryType, SessionMetadata, SessionStorage, SessionTreeEntry,
        SessionTreeEntryBase,
    };
    use crate::types::AgentMessage;
    use chrono::Utc;
    use hamr_ai::types::{MessageContent, MessageRole, TextContent, UserMessage};

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

    #[tokio::test]
    async fn returns_configured_metadata() {
        let metadata = SessionMetadata {
            id: "session-1".to_string(),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        let storage = InMemorySessionStorage::new(None, Some(metadata.clone())).unwrap();
        assert_eq!(storage.get_metadata().await.unwrap(), metadata);
    }

    #[tokio::test]
    async fn maintains_labels_and_paths() {
        let root = SessionTreeEntry::Message {
            entry: crate::harness::types::MessageEntry {
                base: SessionTreeEntryBase {
                    id: "root".to_string(),
                    parent_id: None,
                    timestamp: "2026-01-01T00:00:00.000Z".to_string(),
                },
                message: user_message("root"),
            },
        };
        let child = SessionTreeEntry::Message {
            entry: crate::harness::types::MessageEntry {
                base: SessionTreeEntryBase {
                    id: "child".to_string(),
                    parent_id: Some("root".to_string()),
                    timestamp: "2026-01-01T00:00:00.000Z".to_string(),
                },
                message: user_message("child"),
            },
        };
        let storage =
            InMemorySessionStorage::<SessionMetadata>::new(Some(vec![root.clone(), child]), None)
                .unwrap();
        storage
            .append_entry(SessionTreeEntry::Label {
                entry: LabelEntry {
                    base: SessionTreeEntryBase {
                        id: "label-1".to_string(),
                        parent_id: Some("root".to_string()),
                        timestamp: "2026-01-01T00:00:01.000Z".to_string(),
                    },
                    target_id: "root".to_string(),
                    label: Some("checkpoint".to_string()),
                },
            })
            .await
            .unwrap();

        assert_eq!(
            storage.get_label("root").await.unwrap(),
            Some("checkpoint".to_string())
        );
        assert_eq!(
            storage
                .find_entries(SessionEntryType::Message)
                .await
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            storage
                .get_path_to_root(Some("root".to_string()))
                .await
                .unwrap()
                .len(),
            1
        );
    }
}

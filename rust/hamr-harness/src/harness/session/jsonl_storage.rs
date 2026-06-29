//! Port of `packages/agent/src/harness/session/jsonl-storage.ts`.

use crate::harness::types::{
    FileSystem, JsonlSessionMetadata, LeafEntry, SessionEntryType, SessionError, SessionErrorCode,
    SessionStorage, SessionTreeEntry, SessionTreeEntryBase,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionHeader {
    r#type: String,
    version: u8,
    id: String,
    timestamp: String,
    cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_session: Option<String>,
}

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

fn is_record(value: &serde_json::Value) -> bool {
    value.is_object()
}

fn invalid_session(file_path: &str, message: impl Into<String>) -> SessionError {
    SessionError::new(
        SessionErrorCode::InvalidSession,
        format!("Invalid JSONL session file {file_path}: {}", message.into()),
    )
}

fn invalid_entry(file_path: &str, line_number: usize, message: impl Into<String>) -> SessionError {
    SessionError::new(
        SessionErrorCode::InvalidEntry,
        format!(
            "Invalid JSONL session file {file_path}: line {line_number} {}",
            message.into()
        ),
    )
}

fn parse_header_line(line: &str, file_path: &str) -> Result<SessionHeader, SessionError> {
    let parsed: serde_json::Value = serde_json::from_str(line)
        .map_err(|_| invalid_session(file_path, "first line is not a valid session header"))?;
    if !is_record(&parsed) {
        return Err(invalid_session(
            file_path,
            "first line is not a valid session header",
        ));
    }
    let header: SessionHeader = serde_json::from_value(parsed)
        .map_err(|_| invalid_session(file_path, "first line is not a valid session header"))?;
    if header.r#type != "session" {
        return Err(invalid_session(
            file_path,
            "first line is not a valid session header",
        ));
    }
    if header.version != 3 {
        return Err(invalid_session(file_path, "unsupported session version"));
    }
    if header.id.is_empty() {
        return Err(invalid_session(file_path, "session header is missing id"));
    }
    if header.timestamp.is_empty() {
        return Err(invalid_session(
            file_path,
            "session header is missing timestamp",
        ));
    }
    if header.cwd.is_empty() {
        return Err(invalid_session(file_path, "session header is missing cwd"));
    }
    Ok(header)
}

fn parse_entry_line(
    line: &str,
    file_path: &str,
    line_number: usize,
) -> Result<SessionTreeEntry, SessionError> {
    let parsed: serde_json::Value = serde_json::from_str(line)
        .map_err(|_| invalid_entry(file_path, line_number, "is not valid JSON"))?;
    if !is_record(&parsed) {
        return Err(invalid_entry(
            file_path,
            line_number,
            "is not a valid session entry",
        ));
    }
    let entry: SessionTreeEntry = serde_json::from_value(parsed).map_err(|error| {
        invalid_entry(
            file_path,
            line_number,
            format!("failed to deserialize: {error}"),
        )
    })?;
    Ok(entry)
}

fn leaf_id_after_entry(entry: &SessionTreeEntry) -> Option<String> {
    match entry {
        SessionTreeEntry::Leaf { entry } => entry.target_id.clone(),
        _ => Some(entry.id().to_string()),
    }
}

fn header_to_session_metadata(header: &SessionHeader, path: String) -> JsonlSessionMetadata {
    JsonlSessionMetadata {
        id: header.id.clone(),
        created_at: header.timestamp.clone(),
        cwd: header.cwd.clone(),
        path,
        parent_session_path: header.parent_session.clone(),
    }
}

pub async fn load_jsonl_session_metadata<E: FileSystem>(
    fs: &E,
    file_path: &str,
) -> Result<JsonlSessionMetadata, SessionError> {
    let lines = super::repo_utils::get_file_system_result_or_throw(
        fs.read_text_lines(file_path, Some(1), None).await,
        format!("Failed to read session header {file_path}"),
    )?;
    if let Some(line) = lines.first().filter(|line| !line.trim().is_empty()) {
        Ok(header_to_session_metadata(
            &parse_header_line(line, file_path)?,
            file_path.to_string(),
        ))
    } else {
        Err(invalid_session(file_path, "missing session header"))
    }
}

async fn load_jsonl_storage<E: FileSystem>(
    fs: &E,
    file_path: &str,
) -> Result<(SessionHeader, Vec<SessionTreeEntry>, Option<String>), SessionError> {
    let content = super::repo_utils::get_file_system_result_or_throw(
        fs.read_text_file(file_path, None).await,
        format!("Failed to read session {file_path}"),
    )?;
    let lines = content
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(invalid_session(file_path, "missing session header"));
    }

    let header = parse_header_line(lines[0], file_path)?;
    let mut entries = Vec::new();
    let mut leaf_id = None;
    for (index, line) in lines.iter().enumerate().skip(1) {
        let entry = parse_entry_line(line, file_path, index + 1)?;
        leaf_id = leaf_id_after_entry(&entry);
        entries.push(entry);
    }
    Ok((header, entries, leaf_id))
}

struct State {
    entries: Vec<SessionTreeEntry>,
    by_id: HashMap<String, SessionTreeEntry>,
    labels_by_id: HashMap<String, String>,
    current_leaf_id: Option<String>,
}

pub struct JsonlSessionStorage<E: FileSystem> {
    fs: E,
    file_path: String,
    metadata: JsonlSessionMetadata,
    state: Mutex<State>,
}

impl<E: FileSystem> JsonlSessionStorage<E> {
    fn new(
        fs: E,
        file_path: String,
        header: SessionHeader,
        entries: Vec<SessionTreeEntry>,
        leaf_id: Option<String>,
    ) -> Self {
        let by_id = entries
            .iter()
            .cloned()
            .map(|entry| (entry.id().to_string(), entry))
            .collect::<HashMap<_, _>>();
        let labels_by_id = build_labels_by_id(&entries);
        Self {
            fs,
            metadata: header_to_session_metadata(&header, file_path.clone()),
            file_path,
            state: Mutex::new(State {
                entries,
                by_id,
                labels_by_id,
                current_leaf_id: leaf_id,
            }),
        }
    }

    pub async fn open(fs: E, file_path: String) -> Result<Self, SessionError> {
        let (header, entries, leaf_id) = load_jsonl_storage(&fs, &file_path).await?;
        Ok(Self::new(fs, file_path, header, entries, leaf_id))
    }

    pub async fn create(
        fs: E,
        file_path: String,
        cwd: String,
        session_id: String,
        parent_session_path: Option<String>,
    ) -> Result<Self, SessionError> {
        let header = SessionHeader {
            r#type: "session".to_string(),
            version: 3,
            id: session_id,
            timestamp: super::repo_utils::create_timestamp(),
            cwd,
            parent_session: parent_session_path,
        };
        super::repo_utils::get_file_system_result_or_throw(
            fs.write_file(
                &file_path,
                format!("{}\n", serde_json::to_string(&header).unwrap()).as_bytes(),
                None,
            )
            .await,
            format!("Failed to create session {file_path}"),
        )?;
        Ok(Self::new(fs, file_path, header, Vec::new(), None))
    }
}

#[async_trait]
impl<E> SessionStorage<JsonlSessionMetadata> for JsonlSessionStorage<E>
where
    E: FileSystem + Send + Sync + 'static,
{
    async fn get_metadata(&self) -> Result<JsonlSessionMetadata, SessionError> {
        Ok(self.metadata.clone())
    }

    async fn get_leaf_id(&self) -> Result<Option<String>, SessionError> {
        let state = self.state.lock().await;
        if let Some(ref leaf_id) = state.current_leaf_id {
            if !state.by_id.contains_key(leaf_id) {
                return Err(SessionError::new(
                    SessionErrorCode::InvalidSession,
                    format!("Entry {leaf_id} not found"),
                ));
            }
        }
        Ok(state.current_leaf_id.clone())
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
                    parent_id: state.current_leaf_id.clone(),
                    timestamp: super::repo_utils::create_timestamp(),
                },
                target_id: leaf_id.clone(),
            },
        };
        super::repo_utils::get_file_system_result_or_throw(
            self.fs
                .append_file(
                    &self.file_path,
                    format!("{}\n", serde_json::to_string(&entry).unwrap()).as_bytes(),
                    None,
                )
                .await,
            format!("Failed to append session leaf {}", entry.id()),
        )?;
        state.entries.push(entry.clone());
        state.by_id.insert(entry.id().to_string(), entry);
        state.current_leaf_id = leaf_id;
        Ok(())
    }

    async fn create_entry_id(&self) -> Result<String, SessionError> {
        let state = self.state.lock().await;
        Ok(generate_entry_id(&state.by_id))
    }

    async fn append_entry(&self, entry: SessionTreeEntry) -> Result<(), SessionError> {
        super::repo_utils::get_file_system_result_or_throw(
            self.fs
                .append_file(
                    &self.file_path,
                    format!("{}\n", serde_json::to_string(&entry).unwrap()).as_bytes(),
                    None,
                )
                .await,
            format!("Failed to append session entry {}", entry.id()),
        )?;
        let mut state = self.state.lock().await;
        update_label_cache(&mut state.labels_by_id, &entry);
        state.current_leaf_id = leaf_id_after_entry(&entry);
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
    use super::{JsonlSessionStorage, load_jsonl_session_metadata};
    use crate::harness::env::nodejs::NodeExecutionEnv;
    use crate::harness::types::{SessionStorage, SessionTreeEntry, SessionTreeEntryBase};
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
    async fn creates_and_reloads_storage() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("session.jsonl");
        let env = NodeExecutionEnv::new(dir.path());
        let storage = JsonlSessionStorage::create(
            env.clone(),
            file_path.to_string_lossy().into_owned(),
            dir.path().to_string_lossy().into_owned(),
            "session-1".to_string(),
            None,
        )
        .await
        .unwrap();

        storage
            .append_entry(SessionTreeEntry::Message {
                entry: crate::harness::types::MessageEntry {
                    base: SessionTreeEntryBase {
                        id: "user-1".to_string(),
                        parent_id: None,
                        timestamp: "2026-01-01T00:00:00.000Z".to_string(),
                    },
                    message: user_message("one"),
                },
            })
            .await
            .unwrap();

        let metadata = load_jsonl_session_metadata(&env, &file_path.to_string_lossy())
            .await
            .unwrap();
        assert_eq!(metadata.id, "session-1");

        let loaded = JsonlSessionStorage::open(env, file_path.to_string_lossy().into_owned())
            .await
            .unwrap();
        assert_eq!(
            loaded.get_leaf_id().await.unwrap(),
            Some("user-1".to_string())
        );
        assert_eq!(loaded.get_entries().await.unwrap().len(), 1);
    }
}

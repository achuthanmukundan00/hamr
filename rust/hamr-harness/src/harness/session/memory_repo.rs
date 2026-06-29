//! Port of `packages/agent/src/harness/session/memory-repo.ts`.

use super::memory_storage::InMemorySessionStorage;
use super::repo_utils::{create_session_id, create_timestamp, get_entries_to_fork, to_session};
use crate::harness::types::{SessionError, SessionErrorCode, SessionMetadata};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct InMemorySessionRepo {
    sessions: Mutex<HashMap<String, super::session::Session<SessionMetadata>>>,
}

impl InMemorySessionRepo {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub async fn create(
        &self,
        id: Option<String>,
    ) -> Result<super::session::Session<SessionMetadata>, SessionError> {
        let metadata = SessionMetadata {
            id: id.unwrap_or_else(create_session_id),
            created_at: create_timestamp(),
        };
        let storage = Arc::new(InMemorySessionStorage::new(None, Some(metadata.clone()))?);
        let session = to_session(storage);
        self.sessions
            .lock()
            .await
            .insert(metadata.id.clone(), session.clone());
        Ok(session)
    }

    pub async fn open(
        &self,
        metadata: &SessionMetadata,
    ) -> Result<super::session::Session<SessionMetadata>, SessionError> {
        self.sessions
            .lock()
            .await
            .get(&metadata.id)
            .cloned()
            .ok_or_else(|| {
                SessionError::new(
                    SessionErrorCode::NotFound,
                    format!("Session not found: {}", metadata.id),
                )
            })
    }

    pub async fn list(&self) -> Result<Vec<SessionMetadata>, SessionError> {
        let sessions = self.sessions.lock().await;
        let mut out = Vec::new();
        for session in sessions.values() {
            out.push(session.get_metadata().await?);
        }
        Ok(out)
    }

    pub async fn delete(&self, metadata: &SessionMetadata) -> Result<(), SessionError> {
        self.sessions.lock().await.remove(&metadata.id);
        Ok(())
    }

    pub async fn fork(
        &self,
        source_metadata: &SessionMetadata,
        entry_id: Option<String>,
        position: Option<String>,
        id: Option<String>,
    ) -> Result<super::session::Session<SessionMetadata>, SessionError> {
        let source = self.open(source_metadata).await?;
        let forked_entries = get_entries_to_fork(
            source.get_storage().as_ref(),
            entry_id.as_deref(),
            position.as_deref(),
        )
        .await?;
        let metadata = SessionMetadata {
            id: id.unwrap_or_else(create_session_id),
            created_at: create_timestamp(),
        };
        let storage = Arc::new(InMemorySessionStorage::new(
            Some(forked_entries),
            Some(metadata.clone()),
        )?);
        let session = to_session(storage);
        self.sessions
            .lock()
            .await
            .insert(metadata.id.clone(), session.clone());
        Ok(session)
    }
}

impl Default for InMemorySessionRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::InMemorySessionRepo;
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
    async fn opens_deletes_and_forks() {
        let repo = InMemorySessionRepo::new();
        let session = repo.create(Some("session-1".to_string())).await.unwrap();
        let metadata = session.get_metadata().await.unwrap();
        let user1 = session.append_message(user_message("one")).await.unwrap();
        let user2 = session.append_message(user_message("two")).await.unwrap();

        let opened = repo.open(&metadata).await.unwrap();
        assert_eq!(opened.get_metadata().await.unwrap().id, "session-1");

        let fork = repo
            .fork(
                &metadata,
                Some(user2.clone()),
                Some("before".to_string()),
                Some("session-2".to_string()),
            )
            .await
            .unwrap();
        let entries = fork.get_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id(), user1);

        repo.delete(&metadata).await.unwrap();
        assert!(repo.open(&metadata).await.is_err());
    }
}

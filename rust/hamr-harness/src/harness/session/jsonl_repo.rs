//! Port of `packages/agent/src/harness/session/jsonl-repo.ts`.

use super::jsonl_storage::{JsonlSessionStorage, load_jsonl_session_metadata};
use super::repo_utils::{
    create_session_id, create_timestamp, get_entries_to_fork, get_file_system_result_or_throw,
    to_session,
};
use crate::harness::types::{
    FileSystem, JsonlSessionMetadata, SessionError, SessionErrorCode, SessionStorage,
};
use std::sync::Arc;
use tokio::sync::Mutex;

fn encode_cwd(cwd: &str) -> String {
    format!(
        "--{}--",
        cwd.trim_start_matches(['/', '\\'])
            .replace(['/', '\\', ':'], "-")
    )
}

pub struct JsonlSessionRepo<E: FileSystem + Clone> {
    fs: E,
    sessions_root_input: String,
    sessions_root: Mutex<Option<String>>,
}

impl<E: FileSystem + Clone + Send + Sync + 'static> JsonlSessionRepo<E> {
    pub fn new(fs: E, sessions_root: String) -> Self {
        Self {
            fs,
            sessions_root_input: sessions_root,
            sessions_root: Mutex::new(None),
        }
    }

    async fn get_sessions_root(&self) -> Result<String, SessionError> {
        let mut sessions_root = self.sessions_root.lock().await;
        if sessions_root.is_none() {
            *sessions_root = Some(get_file_system_result_or_throw(
                self.fs.absolute_path(&self.sessions_root_input, None).await,
                format!(
                    "Failed to resolve sessions root {}",
                    self.sessions_root_input
                ),
            )?);
        }
        Ok(sessions_root.clone().unwrap())
    }

    async fn get_session_dir(&self, cwd: &str) -> Result<String, SessionError> {
        get_file_system_result_or_throw(
            self.fs
                .join_path(&[self.get_sessions_root().await?, encode_cwd(cwd)], None)
                .await,
            format!("Failed to resolve session directory for {cwd}"),
        )
    }

    async fn create_session_file_path(
        &self,
        cwd: &str,
        session_id: &str,
        timestamp: &str,
    ) -> Result<String, SessionError> {
        get_file_system_result_or_throw(
            self.fs
                .join_path(
                    &[
                        self.get_session_dir(cwd).await?,
                        format!(
                            "{}_{}.jsonl",
                            timestamp.replace([':', '.'], "-"),
                            session_id
                        ),
                    ],
                    None,
                )
                .await,
            format!("Failed to resolve session file path for {session_id}"),
        )
    }

    pub async fn create(
        &self,
        cwd: String,
        id: Option<String>,
        parent_session_path: Option<String>,
    ) -> Result<super::session::Session<JsonlSessionMetadata>, SessionError> {
        let id = id.unwrap_or_else(create_session_id);
        let created_at = create_timestamp();
        let session_dir = self.get_session_dir(&cwd).await?;
        get_file_system_result_or_throw(
            self.fs.create_dir(&session_dir, true, None).await,
            format!("Failed to create session directory {session_dir}"),
        )?;
        let file_path = self
            .create_session_file_path(&cwd, &id, &created_at)
            .await?;
        let storage = Arc::new(
            JsonlSessionStorage::create(self.fs.clone(), file_path, cwd, id, parent_session_path)
                .await?,
        );
        Ok(to_session(storage))
    }

    pub async fn open(
        &self,
        metadata: &JsonlSessionMetadata,
    ) -> Result<super::session::Session<JsonlSessionMetadata>, SessionError> {
        let exists = get_file_system_result_or_throw(
            self.fs.exists(&metadata.path, None).await,
            format!("Failed to check session {}", metadata.path),
        )?;
        if !exists {
            return Err(SessionError::new(
                SessionErrorCode::NotFound,
                format!("Session not found: {}", metadata.path),
            ));
        }
        Ok(to_session(Arc::new(
            JsonlSessionStorage::open(self.fs.clone(), metadata.path.clone()).await?,
        )))
    }

    pub async fn list(
        &self,
        cwd: Option<String>,
    ) -> Result<Vec<JsonlSessionMetadata>, SessionError> {
        let dirs = if let Some(cwd) = cwd {
            vec![self.get_session_dir(&cwd).await?]
        } else {
            self.list_session_dirs().await?
        };

        let mut sessions = Vec::new();
        for dir in dirs {
            let exists = get_file_system_result_or_throw(
                self.fs.exists(&dir, None).await,
                format!("Failed to check session directory {dir}"),
            )?;
            if !exists {
                continue;
            }
            let files = get_file_system_result_or_throw(
                self.fs.list_dir(&dir, None).await,
                format!("Failed to list sessions in {dir}"),
            )?;
            for file in files
                .into_iter()
                .filter(|file| file.kind != crate::harness::types::FileKind::Directory)
                .filter(|file| file.name.ends_with(".jsonl"))
            {
                match load_jsonl_session_metadata(&self.fs, &file.path).await {
                    Ok(metadata) => sessions.push(metadata),
                    Err(error) if error.code == SessionErrorCode::InvalidSession => {}
                    Err(error) => return Err(error),
                }
            }
        }
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(sessions)
    }

    pub async fn delete(&self, metadata: &JsonlSessionMetadata) -> Result<(), SessionError> {
        get_file_system_result_or_throw(
            self.fs.remove(&metadata.path, false, true, None).await,
            format!("Failed to delete session {}", metadata.path),
        )?;
        Ok(())
    }

    pub async fn fork(
        &self,
        source_metadata: &JsonlSessionMetadata,
        cwd: String,
        entry_id: Option<String>,
        position: Option<String>,
        id: Option<String>,
        parent_session_path: Option<String>,
    ) -> Result<super::session::Session<JsonlSessionMetadata>, SessionError> {
        let source = self.open(source_metadata).await?;
        let forked_entries = get_entries_to_fork(
            source.get_storage().as_ref(),
            entry_id.as_deref(),
            position.as_deref(),
        )
        .await?;
        let id = id.unwrap_or_else(create_session_id);
        let created_at = create_timestamp();
        let session_dir = self.get_session_dir(&cwd).await?;
        get_file_system_result_or_throw(
            self.fs.create_dir(&session_dir, true, None).await,
            format!("Failed to create session directory {session_dir}"),
        )?;
        let file_path = self
            .create_session_file_path(&cwd, &id, &created_at)
            .await?;
        let storage = Arc::new(
            JsonlSessionStorage::create(
                self.fs.clone(),
                file_path,
                cwd,
                id,
                parent_session_path.or_else(|| Some(source_metadata.path.clone())),
            )
            .await?,
        );
        for entry in forked_entries {
            storage.append_entry(entry).await?;
        }
        Ok(to_session(storage))
    }

    async fn list_session_dirs(&self) -> Result<Vec<String>, SessionError> {
        let sessions_root = self.get_sessions_root().await?;
        let exists = get_file_system_result_or_throw(
            self.fs.exists(&sessions_root, None).await,
            format!("Failed to check sessions root {sessions_root}"),
        )?;
        if !exists {
            return Ok(Vec::new());
        }
        let entries = get_file_system_result_or_throw(
            self.fs.list_dir(&sessions_root, None).await,
            format!("Failed to list sessions root {sessions_root}"),
        )?;
        Ok(entries
            .into_iter()
            .filter(|entry| entry.kind == crate::harness::types::FileKind::Directory)
            .map(|entry| entry.path)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::JsonlSessionRepo;
    use crate::harness::env::nodejs::NodeExecutionEnv;
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
    async fn creates_lists_and_forks_sessions() {
        let root = tempfile::tempdir().unwrap();
        let env = NodeExecutionEnv::new(root.path());
        let repo = JsonlSessionRepo::new(env, root.path().to_string_lossy().into_owned());

        let session = repo
            .create(
                "/tmp/my-project".to_string(),
                Some("session-1".to_string()),
                None,
            )
            .await
            .unwrap();
        let metadata = session.get_metadata().await.unwrap();
        let user1 = session.append_message(user_message("one")).await.unwrap();
        let user2 = session.append_message(user_message("two")).await.unwrap();

        let listed = repo
            .list(Some("/tmp/my-project".to_string()))
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, metadata.id);

        let fork = repo
            .fork(
                &metadata,
                "/tmp/target".to_string(),
                Some(user2),
                Some("before".to_string()),
                Some("fork-session".to_string()),
                None,
            )
            .await
            .unwrap();
        let entries = fork.get_entries().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id(), user1);
    }
}

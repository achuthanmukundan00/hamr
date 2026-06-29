//! Port of `packages/agent/src/harness/session/repo-utils.ts`.

use super::session::Session;
use crate::harness::types::{
    FileError, JsonlSessionMetadata, SessionError, SessionErrorCode, SessionStorage,
    SessionTreeEntry,
};
use std::sync::Arc;

pub fn create_session_id() -> String {
    super::uuid::uuidv7()
}

pub fn create_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn to_session<TMetadata: Clone + Send + Sync + 'static>(
    storage: Arc<dyn SessionStorage<TMetadata>>,
) -> Session<TMetadata> {
    Session::new(storage)
}

pub fn get_file_system_result_or_throw<T>(
    result: Result<T, FileError>,
    message: impl Into<String>,
) -> Result<T, SessionError> {
    result.map_err(|error| {
        let code = if error.code == crate::harness::types::FileErrorCode::NotFound {
            SessionErrorCode::NotFound
        } else {
            SessionErrorCode::Storage
        };
        SessionError::new(code, format!("{}: {}", message.into(), error.message))
    })
}

pub async fn get_entries_to_fork<TMetadata>(
    storage: &dyn SessionStorage<TMetadata>,
    entry_id: Option<&str>,
    position: Option<&str>,
) -> Result<Vec<SessionTreeEntry>, SessionError> {
    let Some(entry_id) = entry_id else {
        return storage.get_entries().await;
    };

    let Some(target) = storage.get_entry(entry_id).await? else {
        return Err(SessionError::new(
            SessionErrorCode::InvalidForkTarget,
            format!("Entry {entry_id} not found"),
        ));
    };

    let effective_leaf_id = if position.unwrap_or("before") == "at" {
        Some(target.id().to_string())
    } else {
        match &target {
            SessionTreeEntry::Message { entry }
                if matches!(entry.message, crate::types::AgentMessage::User(_)) =>
            {
                entry.base.parent_id.clone()
            }
            _ => {
                return Err(SessionError::new(
                    SessionErrorCode::InvalidForkTarget,
                    format!("Entry {entry_id} is not a user message"),
                ));
            }
        }
    };

    storage.get_path_to_root(effective_leaf_id).await
}

pub async fn load_jsonl_metadata_from_session(
    session: &Session<JsonlSessionMetadata>,
) -> Result<JsonlSessionMetadata, SessionError> {
    session.get_metadata().await
}

use async_trait::async_trait;
use crate::error::AetherError;
use super::{SessionStore, SessionRecord};

/// 内存会话存储（用于测试和开发，生产用 SQLite）
pub struct InMemorySessionStore {
    sessions: Vec<SessionRecord>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
        }
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn save_session(&self, _session: &SessionRecord) -> Result<(), AetherError> {
        // TODO: 实现内存存储
        Ok(())
    }

    async fn load_session(&self, session_id: &str) -> Result<SessionRecord, AetherError> {
        self.sessions
            .iter()
            .find(|s| s.id == session_id)
            .cloned()
            .ok_or_else(|| AetherError::SessionNotFound(session_id.to_string()))
    }

    async fn search_sessions(&self, _query: &str, _limit: usize) -> Result<Vec<SessionRecord>, AetherError> {
        Ok(self.sessions.clone())
    }

    async fn delete_session(&self, _session_id: &str) -> Result<(), AetherError> {
        // TODO: 实现删除
        Ok(())
    }

    async fn list_sessions(&self) -> Result<Vec<String>, AetherError> {
        Ok(self.sessions.iter().map(|s| s.id.clone()).collect())
    }
}

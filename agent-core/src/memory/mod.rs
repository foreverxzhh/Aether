pub mod state;
pub mod core;
pub mod review;
pub mod curator;

use async_trait::async_trait;
use crate::error::AetherError;
use crate::types::message::Message;

/// 记忆存储抽象
#[async_trait]
pub trait Memory: Send + Sync {
    async fn add(&mut self, messages: &[Message]) -> Result<(), AetherError>;
    async fn get_context(&self, query: &str, limit: usize) -> Result<Vec<Message>, AetherError>;
    async fn clear(&mut self) -> Result<(), AetherError>;
}

/// 会话存储抽象
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn save_session(&self, session: &SessionRecord) -> Result<(), AetherError>;
    async fn load_session(&self, session_id: &str) -> Result<SessionRecord, AetherError>;
    async fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<SessionRecord>, AetherError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), AetherError>;
    async fn list_sessions(&self) -> Result<Vec<String>, AetherError>;
}

/// 会话记录
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub parent_session_id: Option<String>,
    pub source: String,
    pub model: String,
    pub provider: String,
    pub messages: Vec<Message>,
    pub created_at: String,
    pub updated_at: String,
}

pub mod state;

use async_trait::async_trait;
use crate::error::AetherError;
use crate::types::message::Message;

/// 记忆存储抽象
#[async_trait]
pub trait Memory: Send + Sync {
    /// 添加消息到记忆
    async fn add(&mut self, messages: &[Message]) -> Result<(), AetherError>;

    /// 获取相关上下文
    async fn get_context(&self, query: &str, limit: usize) -> Result<Vec<Message>, AetherError>;

    /// 清空记忆
    async fn clear(&mut self) -> Result<(), AetherError>;
}

/// 会话存储抽象
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// 保存会话
    async fn save_session(&self, session: &SessionRecord) -> Result<(), AetherError>;

    /// 加载会话
    async fn load_session(&self, session_id: &str) -> Result<SessionRecord, AetherError>;

    /// 搜索会话
    async fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<SessionRecord>, AetherError>;

    /// 删除会话
    async fn delete_session(&self, session_id: &str) -> Result<(), AetherError>;

    /// 列出所有会话 ID
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

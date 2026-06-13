use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::AetherError;

/// 技能文件（agentskills.io 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub content: String,
}

/// 技能存储抽象
#[async_trait]
pub trait SkillStore: Send + Sync {
    /// 列出所有技能
    async fn list_skills(&self) -> Result<Vec<Skill>, AetherError>;

    /// 获取技能详情
    async fn get_skill(&self, name: &str) -> Result<Skill, AetherError>;

    /// 保存技能
    async fn save_skill(&self, skill: &Skill) -> Result<(), AetherError>;

    /// 删除技能
    async fn delete_skill(&self, name: &str) -> Result<(), AetherError>;

    /// 搜索技能
    async fn search_skills(&self, query: &str) -> Result<Vec<Skill>, AetherError>;
}

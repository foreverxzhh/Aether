use crate::config::AgentConfig;

/// AIAgent — 核心 Agent 类
#[derive(Debug)]
pub struct AIAgent {
    pub config: AgentConfig,
}

impl AIAgent {
    /// 创建 Agent（用 Builder 构建的配置）
    pub fn new(config: AgentConfig) -> Self {
        Self { config }
    }

    /// 简单对话（TODO: Phase 3 实现完整 ReAct 循环）
    pub async fn chat(&self, _message: &str) -> Result<String, crate::error::AetherError> {
        // TODO: 实现 ReAct 循环
        Ok("Aether 响应待实现".to_string())
    }

    /// 当前模型名称
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// 当前供应商
    pub fn provider(&self) -> &str {
        &self.config.provider
    }
}

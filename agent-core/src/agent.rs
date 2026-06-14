use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::AgentConfig;
use crate::error::AetherError;
#[allow(unused_imports)]
use crate::llm::{ChatModel, Streamable};
use crate::llm::provider::create_chat_model;
use crate::loop_mod;
use crate::tools::ToolRegistry;
use crate::types::model::{StreamChunk, TurnResult};

/// AIAgent — 核心 Agent 类
pub struct AIAgent {
    pub config: AgentConfig,
    model: Option<Box<dyn ChatModel>>,
    tools: Arc<RwLock<ToolRegistry>>,
}

impl AIAgent {
    /// 创建 Agent
    pub fn new(config: AgentConfig) -> Self {
        Self {
            config,
            model: None,
            tools: Arc::new(RwLock::new(ToolRegistry::new())),
        }
    }

    /// 获取模型引用
    pub fn model(&self) -> Option<&dyn ChatModel> {
        self.model.as_deref()
    }

    /// 初始化 LLM 供应商（延迟初始化，因为需要 async）
    pub async fn init_model(&mut self) -> Result<(), AetherError> {
        let model = create_chat_model(&self.config)?;
        self.model = Some(model);
        Ok(())
    }

    /// 注册工具
    pub async fn register_tool(&self, tool: impl crate::tools::Tool + 'static) {
        let mut registry = self.tools.write().await;
        registry.register(Box::new(tool));
    }

    /// 获取工具定义（JSON Schema 格式，发给 LLM）
    pub fn get_tool_definitions(&self) -> Vec<serde_json::Value> {
        vec![]
        // TODO: Phase 4 实现从 ToolRegistry 生成 tool definitions
    }

    /// 执行工具
    pub async fn execute_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<String, AetherError> {
        let registry = self.tools.read().await;
        let tool = registry.find(name).ok_or_else(|| {
            AetherError::ToolNotFound(name.to_string())
        })?;
        tool.call(args).await
    }

    /// 简单对话
    pub async fn chat(&self, message: &str) -> Result<String, AetherError> {
        let result = self.run_conversation(message).await?;
        Ok(result.final_response.unwrap_or_default())
    }

    /// 完整对话（返回详细结果）
    pub async fn run_conversation(&self, user_message: &str) -> Result<TurnResult, AetherError> {
        if self.model.is_none() {
            return Err(AetherError::ConfigError(
                "模型未初始化。请先调用 init_model()".to_string(),
            ));
        }
        loop_mod::run_conversation(self, user_message).await
    }

    /// 获取供应商名称
    pub fn provider_name(&self) -> &str {
        self.model
            .as_ref()
            .map(|m| m.provider_name())
            .unwrap_or("unknown")
    }

    /// 流式对话（逐 chunk 回调）
    pub async fn chat_stream<F: FnMut(StreamChunk)>(
        &self,
        message: &str,
        mut callback: F,
    ) -> Result<String, AetherError> {
        let model = self.model.as_ref().ok_or_else(|| {
            AetherError::ConfigError("模型未初始化".to_string())
        })?;

        let system_msg = crate::prompt::PromptBuilder::build_system_message(
            self.config.system_prompt.as_deref(),
            None,
            None,
        );
        let user_msg = crate::types::message::Message::user(message);
        let messages = vec![system_msg, user_msg];

        let mut stream = model.stream(&messages, &[]).await?;
        let mut full_response = String::new();

        while let Some(chunk) = stream.next_chunk().await? {
            full_response.push_str(&chunk.delta);
            callback(chunk);
        }

        Ok(full_response)
    }
}

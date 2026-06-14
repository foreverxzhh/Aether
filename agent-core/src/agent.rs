use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::AgentConfig;
use crate::error::AetherError;
use crate::llm::{ChatModel, Streamable};
use crate::llm::provider::create_chat_model;
use crate::loop_mod;
use crate::tools::ToolRegistry;
use crate::tools::file_tools::{ReadFile, WriteFile, Patch, SearchFiles};
use crate::tools::terminal_tool::Terminal;
use crate::tools::web_tools::{WebSearch, WebExtract};
use crate::tools::memory_tool::Memory;
use crate::tools::skills_tool::{SkillsList, SkillView, SkillManage};
use crate::types::model::{StreamChunk, TurnResult};

pub struct AIAgent {
    pub config: AgentConfig,
    model: Option<Box<dyn ChatModel>>,
    pub(crate) tools: Arc<RwLock<ToolRegistry>>,
}

impl AIAgent {
    pub fn new(config: AgentConfig) -> Self {
        let registry = ToolRegistry::new();
        registry.register(ReadFile);
        registry.register(WriteFile);
        registry.register(Patch);
        registry.register(SearchFiles);
        registry.register(Terminal);
        registry.register(WebSearch);
        registry.register(WebExtract);
        registry.register(Memory);
        registry.register(SkillsList);
        registry.register(SkillView);
        registry.register(SkillManage);

        Self {
            config,
            model: None,
            tools: Arc::new(RwLock::new(registry)),
        }
    }

    pub fn model(&self) -> Option<&dyn ChatModel> {
        self.model.as_deref()
    }

    pub async fn init_model(&mut self) -> Result<(), AetherError> {
        let model = create_chat_model(&self.config)?;
        self.model = Some(model);
        Ok(())
    }

    pub fn get_tool_definitions(&self) -> Vec<serde_json::Value> {
        self.tools.try_read()
            .map(|r| r.get_definitions())
            .unwrap_or_default()
    }

    pub async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, AetherError> {
        let registry = self.tools.read().await;
        registry.execute(name, args).await
    }

    pub async fn chat(&self, message: &str) -> Result<String, AetherError> {
        let result = self.run_conversation(message).await?;
        Ok(result.final_response.unwrap_or_default())
    }

    pub async fn run_conversation(&self, user_message: &str) -> Result<TurnResult, AetherError> {
        if self.model.is_none() {
            return Err(AetherError::ConfigError("模型未初始化。请先调用 init_model()".into()));
        }
        loop_mod::run_conversation(self, user_message).await
    }

    pub fn provider_name(&self) -> &str {
        self.model.as_ref().map(|m| m.provider_name()).unwrap_or("unknown")
    }

    pub async fn chat_stream<F: FnMut(StreamChunk)>(
        &self,
        message: &str,
        mut callback: F,
    ) -> Result<String, AetherError> {
        let model = self.model.as_ref().ok_or_else(|| {
            AetherError::ConfigError("模型未初始化".into())
        })?;

        let system_msg = crate::prompt::PromptBuilder::build_system_message(
            self.config.system_prompt.as_deref(), None, None,
        );
        let user_msg = crate::types::message::Message::user(message);
        let messages = vec![system_msg, user_msg];
        let tools = self.get_tool_definitions();

        let mut stream = model.stream(&messages, &tools).await?;
        let mut full_response = String::new();
        while let Some(chunk) = stream.next_chunk().await? {
            full_response.push_str(&chunk.delta);
            callback(chunk);
        }
        Ok(full_response)
    }
}

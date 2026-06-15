use crate::config::AgentConfig;
use crate::error::AetherError;
use crate::llm::provider::create_chat_model;
use crate::llm::{ChatModel, Streamable};
use crate::loop_mod;
use crate::tools::file_tools::{Patch, ReadFile, SearchFiles, WriteFile};
use crate::tools::memory_tool::Memory;
use crate::tools::skills_tool::{SkillManage, SkillView, SkillsList};
use crate::tools::terminal_backends::{DockerTerminal, ExecuteCode, SshTerminal};
use crate::tools::terminal_tool::Terminal;
use crate::tools::web_tools::{WebExtract, WebSearch};
use crate::tools::ToolRegistry;
use crate::types::model::{StreamChunk, TurnResult};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Aether Agent 主类
///
/// 驱动 ReAct 循环（推理→行动→观察），管理 LLM 供应商、工具系统和记忆。
///
/// # 示例
///
/// ```rust,no_run
/// use agent_core::*;
/// use agent_core::config::AgentConfigBuilder;
///
/// # async fn example() {
/// let mut agent = AIAgent::new(
///     AgentConfigBuilder::new()
///         .provider("deepseek")
///         .model("deepseek-v4-flash")
///         .api_key(std::env::var("DEEPSEEK_API_KEY").unwrap_or_default())
///         .build()
/// );
/// agent.init_model().await.unwrap();
///
/// // 同步对话
/// let reply = agent.chat("你好").await.unwrap();
///
/// // 流式对话
/// agent.chat_stream("讲个故事", |chunk| {
///     print!("{}", chunk.delta);
/// }).await.unwrap();
/// # }
/// ```
pub struct AIAgent {
    /// Agent 配置
    pub config: AgentConfig,
    model: Option<Box<dyn ChatModel>>,
    pub(crate) tools: Arc<RwLock<ToolRegistry>>,
}

impl AIAgent {
    /// 创建新的 Agent 实例
    ///
    /// 自动注册 11 个内置工具（文件/终端/Web/记忆/技能）。
    /// 创建后需要调用 `init_model()` 初始化 LLM。
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
        // T-1.1: CronJob/ImageGenerate/HomeAssistant 已移除（桩函数，向LLM撒谎）
        // T-3.6: 真 delegate 见 future task
        registry.register(DockerTerminal);
        registry.register(SshTerminal);
        registry.register(ExecuteCode);
        Self {
            config,
            model: None,
            tools: Arc::new(RwLock::new(registry)),
        }
    }

    /// 获取 LLM 供应商引用
    ///
    /// 返回 `None` 表示 `init_model()` 尚未调用。
    pub fn model(&self) -> Option<&dyn ChatModel> {
        self.model.as_deref()
    }

    /// 初始化 LLM 供应商
    ///
    /// 根据 `config.provider` 创建对应的供应商实例：
    /// - `openai` → OpenAI GPT
    /// - `anthropic` → Anthropic Claude
    /// - `deepseek` → DeepSeek
    /// - `ollama` → 本地 Ollama
    ///
    /// # 错误
    ///
    /// 返回 `ConfigError`（未知供应商或不完整配置）。
    pub async fn init_model(&mut self) -> Result<(), AetherError> {
        let model = create_chat_model(&self.config)?;
        self.model = Some(model);
        Ok(())
    }

    pub fn get_tool_definitions(&self) -> Vec<serde_json::Value> {
        self.tools
            .try_read()
            .map(|r| r.get_definitions())
            .unwrap_or_default()
    }

    pub async fn execute_tool(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<String, AetherError> {
        let registry = self.tools.read().await;
        registry.execute(name, args).await
    }

    /// 与 Agent 对话（同步）
    ///
    /// 发送一条消息并等待完整回复。
    /// 内部会驱动 ReAct 循环直到 LLM 返回最终回复或工具调用完成。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use agent_core::*;
    /// # use agent_core::config::AgentConfigBuilder;
    /// # async fn example() {
    /// # let mut agent = AIAgent::new(AgentConfigBuilder::new().provider("deepseek").model("m").build());
    /// # agent.init_model().await.unwrap();
    /// let reply = agent.chat("你好").await.unwrap();
    /// println!("{}", reply);
    /// # }
    /// ```
    pub async fn chat(&self, message: &str) -> Result<String, AetherError> {
        let result = self.run_conversation(message).await?;
        Ok(result.final_response.unwrap_or_default())
    }

    pub async fn run_conversation(&self, user_message: &str) -> Result<TurnResult, AetherError> {
        if self.model.is_none() {
            return Err(AetherError::ConfigError(
                "模型未初始化。请先调用 init_model()".into(),
            ));
        }
        let result = loop_mod::run_conversation(self, user_message).await?;

        // 后台触发学习闭环（独立创建 review Agent，避免借用 &self 的生命周期问题）
        let config = self.config.clone();
        let messages = result.messages.clone();
        let tool_count = result.tool_results.len();
        tokio::spawn(async move {
            if let Ok(mut review_agent) = create_chat_model(&config) {
                let hermes_home = crate::memory::core::default_hermes_home();
                if let Err(e) = crate::memory::review::review_and_learn(
                    &messages,
                    tool_count,
                    &hermes_home,
                    review_agent.as_ref(),
                )
                .await
                {
                    tracing::warn!(error = %e, "Background Review 失败");
                }
            }
        });

        // T-3.5: Curator inline check — chat 结束时检查是否需要运行
        let skills_dir = self.hermes_home().join("skills");
        crate::memory::curator::maybe_run_inline(
            &skills_dir,
            &crate::memory::curator::CuratorConfig::default(),
        );

        Ok(result)
    }

    /// 获取当前 profile 的 HERMES_HOME 路径
    pub fn hermes_home(&self) -> std::path::PathBuf {
        crate::profile::ProfileManager::new(self.config.profile.clone()).home()
    }

    pub fn provider_name(&self) -> &str {
        self.model
            .as_ref()
            .map(|m| m.provider_name())
            .unwrap_or("unknown")
    }

    pub async fn chat_stream<F: FnMut(StreamChunk)>(
        &self,
        message: &str,
        mut callback: F,
    ) -> Result<String, AetherError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| AetherError::ConfigError("模型未初始化".into()))?;
        let system_msg = crate::prompt::PromptBuilder::build_system_message(
            self.config.system_prompt.as_deref(),
            None,
            None,
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

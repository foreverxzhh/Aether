use crate::config::AgentConfig;
use crate::error::AetherError;
use crate::llm::provider::create_chat_model;
use crate::llm::{ChatModel, Streamable};
use crate::loop_mod;
use crate::tools::file_tools::{Patch, ReadFile, SearchFiles, WriteFile};
use crate::tools::memory_tool::Memory;
use crate::types::model::StreamEvent;
use tokio::sync::mpsc;
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
    model: Option<Arc<dyn ChatModel>>,
    pub(crate) tools: Arc<RwLock<ToolRegistry>>,
}

impl AIAgent {
    /// 创建新的 Agent 实例
    ///
    /// 自动注册 14 个内置工具（文件/终端/Web/记忆/技能/Docker/SSH/沙箱）。
    /// 创建后需要调用 `init_model()` 初始化 LLM；
    /// init_model 时会额外注册 `delegate` 工具（依赖 model 句柄）。
    pub fn new(config: AgentConfig) -> Self {
        // R-1.4: log_level 接线 — 在 agent 创建时初始化 tracing
        crate::tracing::init_tracing(&config.log_level);

        // T-1.4: 把 profile-aware 的 home 注入到 Memory/Skills 工具中，
        // 避免它们走 default_hermes_home() 绕过 profile 隔离。
        let profile_home = {
            let pm = crate::profile::ProfileManager::new(config.profile.clone());
            pm.home()
        };
        let registry = ToolRegistry::new();

        // R-1.4: enabled_toolsets / disabled_toolsets 过滤工具注册
        let toolset_allowed = |ts: &str| {
            let enabled = config.enabled_toolsets.is_empty()
                || config.enabled_toolsets.iter().any(|e| e == ts);
            let disabled = config.disabled_toolsets.iter().any(|d| d == ts);
            enabled && !disabled
        };

        if toolset_allowed("file") {
            registry.register(ReadFile);
            registry.register(WriteFile);
            registry.register(Patch);
            registry.register(SearchFiles);
        }
        if toolset_allowed("terminal") {
            registry.register(Terminal);
            registry.register(DockerTerminal);
            registry.register(SshTerminal);
            registry.register(ExecuteCode);
        }
        if toolset_allowed("web") {
            registry.register(WebSearch);
            registry.register(WebExtract);
        }
        if toolset_allowed("memory") {
            registry.register(Memory::new(Some(profile_home.clone())));
        }
        // R-1.4: skills_enabled 控制技能工具注册
        if config.skills_enabled && toolset_allowed("skills") {
            registry.register(SkillsList::new(Some(profile_home.clone())));
            registry.register(SkillView::new(Some(profile_home.clone())));
            registry.register(SkillManage::new(Some(profile_home)));
        }
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
        let arc_model: Arc<dyn ChatModel> = Arc::from(model);
        self.model = Some(arc_model.clone());

        // T-3.6: init_model 之后注册真正的 Delegate 工具（依赖 model 句柄）。
        // 受 config.delegation_enabled 控制，默认开启。
        if self.config.delegation_enabled {
            let max_depth = self.config.max_spawn_depth.max(1);
            let max_iter = self.config.max_iterations.min(60);
            let delegate = crate::delegate::Delegate::new(
                arc_model,
                self.tools.clone(),
                max_iter,
                max_depth,
            );
            self.tools.write().await.register(delegate);
        }
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
        // T-1.4: 用 profile-aware home 而非 default_hermes_home()
        let hermes_home = self.hermes_home();
        tokio::spawn(async move {
            if let Ok(review_agent) = create_chat_model(&config) {
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

        // T-3.5 v2: Curator inline check — chat 结束时**异步**触发，不阻塞当前 chat。
        // v1 是同步调用：到期那次 chat 会被 curator 卡住（含若干 LLM 调用 + 文件 IO）。
        // v2 改为：先用 `should_run` 做廉价检查（仅读 marker 文件），到期才 `spawn_blocking`
        // 把真正的 `run_curator` 放到 blocking pool 上跑，主 chat 立即返回。
        let skills_dir = self.hermes_home().join("skills");
        let curator_cfg = crate::memory::curator::CuratorConfig::default();
        if crate::memory::curator::should_run(&skills_dir, &curator_cfg) {
            let skills_dir_bg = skills_dir.clone();
            let cfg_bg = curator_cfg.clone();
            tokio::task::spawn_blocking(move || {
                match crate::memory::curator::run_curator(&skills_dir_bg, &cfg_bg) {
                    Ok(report) => tracing::info!(?report, "Curator 已运行(后台)"),
                    Err(e) => tracing::warn!(%e, "Curator 后台任务失败"),
                }
            });
        }

        Ok(result)
    }

    /// 获取当前 profile 的 HERMES_HOME 路径
    /// T-2.8: 消费 config.profile，按 profile 隔离 session/skills/memory
    pub fn hermes_home(&self) -> std::path::PathBuf {
        let pm = crate::profile::ProfileManager::new(self.config.profile.clone());
        let home = pm.home();
        // 确保 per-profile 目录存在
        std::fs::create_dir_all(&home).ok();
        std::fs::create_dir_all(home.join("skills")).ok();
        std::fs::create_dir_all(home.join("memory")).ok();
        home
    }

    pub fn provider_name(&self) -> &str {
        self.model
            .as_ref()
            .map(|m| m.provider_name())
            .unwrap_or("unknown")
    }

    /// R-1.1: 流式对话 — 返回 `Stream<Item = StreamEvent>`
    ///
    /// 驱动完整 ReAct 循环：
    /// 1. 流式调用 LLM → 输出 `Text(...)` / `ToolCall(...)` 事件
    /// 2. 工具调用完成后 → 输出 `ToolResult(...)` 事件
    /// 3. 循环直到 LLM 不再请求工具 → 输出 `Done(...)` 事件
    ///
    /// 与 `chat_stream`（回调模式）等价，但使用 Rust Stream trait，
    /// 方便 Rust 库用户用 `stream.next().await` 或 `while let Some(event) = ...` 消费。
    pub fn chat_stream_events(
        &self,
        message: &str,
    ) -> MpscStream {
        let (tx, rx) = mpsc::channel::<StreamEvent>(32);
        let model = self.model.clone();
        let tools = self.tools.clone();
        let config = self.config.clone();
        let message = message.to_string();

        tokio::spawn(async move {
            let model = match model {
                Some(m) => m,
                None => {
                    let _ = tx.send(StreamEvent::Error("模型未初始化".into())).await;
                    let _ = tx.send(StreamEvent::Done(String::new())).await;
                    return;
                }
            };

            let system_msg = crate::prompt::PromptBuilder::build_system_message(
                config.system_prompt.as_deref(), None, None,
            );
            let user_msg = crate::types::message::Message::user(&message);
            let mut messages = vec![system_msg, user_msg];

            loop {
                let tool_defs = tools
                    .try_read()
                    .map(|r| r.get_definitions())
                    .unwrap_or_default();

                let mut stream = match model.stream(&messages, &tool_defs).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(StreamEvent::Error(format!("流式调用失败: {}", e))).await;
                        let _ = tx.send(StreamEvent::Done(String::new())).await;
                        return;
                    }
                };

                let mut full_response = String::new();
                let mut tool_calls: Vec<crate::types::model::ToolCallInfo> = Vec::new();

                loop {
                    match stream.next_chunk().await {
                        Ok(Some(chunk)) => {
                            if !chunk.delta.is_empty() {
                                full_response.push_str(&chunk.delta);
                                let _ = tx.send(StreamEvent::Text(chunk.delta.clone())).await;
                            }
                            if let Some(ref tcs) = chunk.tool_calls {
                                for tc in tcs {
                                    let _ = tx
                                        .send(StreamEvent::ToolCall {
                                            id: tc.id.clone(),
                                            name: tc.name.clone(),
                                            arguments: tc.arguments.clone(),
                                        })
                                        .await;
                                }
                                tool_calls.extend(tcs.clone());
                            }
                            if chunk.finish_reason.is_some() {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            let _ = tx
                                .send(StreamEvent::Error(format!("流式读取失败: {}", e)))
                                .await;
                            break;
                        }
                    }
                }

                if tool_calls.is_empty() {
                    let _ = tx.send(StreamEvent::Done(full_response)).await;
                    return;
                }

                // 执行工具，输出 ToolResult 事件
                for tc in &tool_calls {
                    let args = serde_json::from_str(&tc.arguments).unwrap_or_default();
                    let registry = tools.read().await;
                    match registry.execute(&tc.name, args).await {
                        Ok(result) => {
                            let _ = tx
                                .send(StreamEvent::ToolResult {
                                    tool_call_id: tc.id.clone(),
                                    result: result.clone(),
                                })
                                .await;
                            messages.push(crate::types::message::Message::tool_result(
                                &tc.id, &result,
                            ));
                        }
                        Err(e) => {
                            let err_msg = format!("错误: {}", e);
                            let _ = tx
                                .send(StreamEvent::ToolResult {
                                    tool_call_id: tc.id.clone(),
                                    result: err_msg.clone(),
                                })
                                .await;
                            messages.push(crate::types::message::Message::tool_result(
                                &tc.id, &err_msg,
                            ));
                        }
                    }
                }
            }
        });

        MpscStream { rx }
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
            self.config.system_prompt.as_deref(), None, None,
        );
        let user_msg = crate::types::message::Message::user(message);
        let mut messages = vec![system_msg, user_msg];
        let tools = self.get_tool_definitions();

        // T-3.3: ReAct 循环在流式模式下
        loop {
            let mut stream = model.stream(&messages, &tools).await?;
            let mut full_response = String::new();
            let mut tool_calls: Vec<crate::types::model::ToolCallInfo> = Vec::new();

            while let Some(chunk) = stream.next_chunk().await? {
                if !chunk.delta.is_empty() {
                    full_response.push_str(&chunk.delta);
                }
                if let Some(tcs) = &chunk.tool_calls {
                    tool_calls.extend(tcs.clone());
                }
                callback(chunk.clone());
                if chunk.finish_reason.is_some() {
                    break;
                }
            }

            if tool_calls.is_empty() {
                return Ok(full_response);
            }

            // 执行工具后继续循环
            for tc in &tool_calls {
                let args = serde_json::from_str(&tc.arguments).unwrap_or_default();
                match self.execute_tool(&tc.name, args).await {
                    Ok(result) => {
                        messages.push(crate::types::message::Message::tool_result(&tc.id, &result));
                    }
                    Err(e) => {
                        messages.push(crate::types::message::Message::tool_result(&tc.id, &format!("错误: {}", e)));
                    }
                }
            }
        }
    }
}

/// R-1.1: tokio::sync::mpsc::Receiver 的 Stream 适配器
pub struct MpscStream {
    rx: mpsc::Receiver<StreamEvent>,
}

impl futures::stream::Stream for MpscStream {
    type Item = StreamEvent;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

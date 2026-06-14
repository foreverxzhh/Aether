use serde::{Deserialize, Serialize};

/// Agent 配置（通过 `AgentConfigBuilder` 构建）
///
/// # 示例
///
/// ```rust,no_run
/// use agent_core::config::AgentConfigBuilder;
///
/// let config = AgentConfigBuilder::new()
///     .provider("openai")
///     .model("gpt-4o")
///     .api_key("sk-xxx")
///     .system_prompt("你是一个助手")
///     .build();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// LLM 供应商名称 (`openai` / `anthropic` / `deepseek` / `ollama`)
    pub provider: String,
    /// 模型名称 (如 `gpt-4o`, `claude-sonnet-4-6`, `deepseek-v4-flash`)
    pub model: String,
    /// API Key（也可通过环境变量 `{PROVIDER}_API_KEY` 设置）
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    /// 自定义 API 地址（OpenAI 兼容协议时使用）
    pub base_url: Option<String>,
    /// 系统提示词
    pub system_prompt: Option<String>,
    /// 最大迭代次数（单次对话的 LLM 调用次数上限）
    pub max_iterations: u32,
    /// 采样温度
    pub temperature: Option<f32>,
    /// 最大输出 token 数
    pub max_tokens: Option<u32>,
    /// 启用的工具集
    pub enabled_toolsets: Vec<String>,
    /// 禁用的工具集
    pub disabled_toolsets: Vec<String>,
    /// 是否启用记忆
    pub memory_enabled: bool,
    /// 记忆供应商
    pub memory_provider: String,
    /// 是否启用上下文压缩
    pub compression_enabled: bool,
    /// 压缩阈值（占上下文窗口比例）
    pub compression_threshold_ratio: f64,
    /// 是否启用技能
    pub skills_enabled: bool,
    /// 是否启用委托（子 Agent）
    pub delegation_enabled: bool,
    /// 最大并行子 Agent 数
    pub max_concurrent_children: u32,
    /// 最大委托深度
    pub max_spawn_depth: u32,
    /// 会话 ID
    pub session_id: Option<String>,
    /// Profile 名称
    pub profile: Option<String>,
    /// 日志级别
    pub log_level: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
            api_key: None,
            base_url: None,
            system_prompt: None,
            max_iterations: 90,
            temperature: None,
            max_tokens: None,
            enabled_toolsets: vec!["core".to_string()],
            disabled_toolsets: Vec::new(),
            memory_enabled: true,
            memory_provider: "local".to_string(),
            compression_enabled: true,
            compression_threshold_ratio: 0.75,
            skills_enabled: true,
            delegation_enabled: true,
            max_concurrent_children: 3,
            max_spawn_depth: 2,
            session_id: None,
            profile: None,
            log_level: "info".to_string(),
        }
    }
}

/// Agent 配置构建器（Builder 模式）
///
/// # 示例
///
/// ```rust
/// use agent_core::config::AgentConfigBuilder;
///
/// let config = AgentConfigBuilder::new()
///     .provider("deepseek")
///     .model("deepseek-v4-flash")
///     .api_key(std::env::var("DEEPSEEK_API_KEY").unwrap_or_default())
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct AgentConfigBuilder {
    config: AgentConfig,
}

impl AgentConfigBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self { config: AgentConfig::default() }
    }

    /// 设置 LLM 供应商
    ///
    /// 支持: `openai`, `anthropic`, `deepseek`, `ollama`
    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.config.provider = provider.into();
        self
    }

    /// 设置模型名称
    ///
    /// 例如: `gpt-4o`, `claude-sonnet-4-6`, `deepseek-v4-flash`, `llama3`
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    /// 设置 API Key
    ///
    /// 也可通过环境变量设置（如 `DEEPSEEK_API_KEY`），
    /// CLI 工具会自动读取。
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    /// 设置自定义 API 地址
    ///
    /// 用于兼容 OpenAI 协议的 API 服务（如 DeepSeek 等）。
    /// 内置供应商有默认地址，不需要设置。
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = Some(url.into());
        self
    }

    /// 设置系统提示词
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.system_prompt = Some(prompt.into());
        self
    }

    /// 设置最大迭代次数
    pub fn max_iterations(mut self, n: u32) -> Self {
        self.config.max_iterations = n;
        self
    }

    /// 启用工具集
    pub fn enable_toolset(mut self, toolset: impl Into<String>) -> Self {
        self.config.enabled_toolsets.push(toolset.into());
        self
    }

    /// 设置会话 ID
    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.config.session_id = Some(id.into());
        self
    }

    /// 构建 `AgentConfig`
    ///
    /// 返回配置对象，可用于 `AIAgent::new(config)`。
    pub fn build(self) -> AgentConfig {
        self.config
    }
}

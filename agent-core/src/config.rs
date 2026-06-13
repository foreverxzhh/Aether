use serde::{Deserialize, Serialize};

/// Agent 配置（外部接口——通过 Builder 模式构建）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    // ── LLM 配置 ──
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,

    // ── Agent 行为 ──
    pub system_prompt: Option<String>,
    pub max_iterations: u32,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,

    // ── 工具 ──
    pub enabled_toolsets: Vec<String>,
    pub disabled_toolsets: Vec<String>,

    // ── 记忆 ──
    pub memory_enabled: bool,
    pub memory_provider: String,

    // ── 上下文压缩 ──
    pub compression_enabled: bool,
    pub compression_threshold_ratio: f64,

    // ── 技能 ──
    pub skills_enabled: bool,

    // ── 委托 ──
    pub delegation_enabled: bool,
    pub max_concurrent_children: u32,
    pub max_spawn_depth: u32,

    // ── 会话 ──
    pub session_id: Option<String>,
    pub profile: Option<String>,

    // ── 可观测性 ──
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

/// AgentConfig Builder
#[derive(Debug, Default)]
pub struct AgentConfigBuilder {
    config: AgentConfig,
}

impl AgentConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: AgentConfig::default(),
        }
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.config.provider = provider.into();
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.config.api_key = Some(key.into());
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = Some(url.into());
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.system_prompt = Some(prompt.into());
        self
    }

    pub fn max_iterations(mut self, n: u32) -> Self {
        self.config.max_iterations = n;
        self
    }

    pub fn temperature(mut self, t: f32) -> Self {
        self.config.temperature = Some(t);
        self
    }

    pub fn enable_toolset(mut self, toolset: impl Into<String>) -> Self {
        self.config.enabled_toolsets.push(toolset.into());
        self
    }

    pub fn session_id(mut self, id: impl Into<String>) -> Self {
        self.config.session_id = Some(id.into());
        self
    }

    pub fn build(self) -> AgentConfig {
        self.config
    }
}

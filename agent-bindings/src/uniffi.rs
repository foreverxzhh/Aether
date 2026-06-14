//! UniFFI 跨语言绑定接口
//! 供 Kotlin (Android)、Swift (iOS)、C# (Windows) 调用

use std::sync::Arc;
use tokio::sync::Mutex;
use agent_core::config::AgentConfigBuilder;
use agent_core::error::AetherError;

/// SDK 错误（UniFFI 需要用可枚举的错误类型）
#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum AetherSdkError {
    #[error("LLM 错误: {0}")]
    LlmError(String),
    #[error("配置错误: {0}")]
    ConfigError(String),
    #[error("运行时错误: {0}")]
    RuntimeError(String),
    #[error("IO 错误: {0}")]
    IoError(String),
}

impl From<AetherError> for AetherSdkError {
    fn from(e: AetherError) -> Self {
        match e {
            AetherError::LlmError(msg) => Self::LlmError(msg),
            AetherError::ConfigError(msg) => Self::ConfigError(msg),
            AetherError::IoError(msg) => Self::IoError(msg),
            AetherError::LlmEmptyResponse => Self::LlmError("空响应".into()),
            _ => Self::RuntimeError(e.to_string()),
        }
    }
}

/// Agent 配置（对应 .udl 中的 dictionary）
#[derive(uniffi::Record)]
pub struct SdkConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub system_prompt: Option<String>,
}

/// Aether Agent 主接口（对应 .udl 中的 interface）
#[derive(uniffi::Object)]
pub struct AetherAgent {
    inner: Arc<Mutex<agent_core::AIAgent>>,
    provider: String,
    model: String,
}

#[uniffi::export]
impl AetherAgent {
    /// 创建 Agent（同步构造，后续调用 init_model）
    #[uniffi::constructor]
    pub fn with_config(config: SdkConfig) -> Self {
        let mut builder = AgentConfigBuilder::new()
            .provider(&config.provider)
            .model(&config.model);
        if let Some(key) = &config.api_key {
            builder = builder.api_key(key.as_str());
        }
        if let Some(url) = &config.base_url {
            builder = builder.base_url(url.as_str());
        }
        if let Some(prompt) = &config.system_prompt {
            builder = builder.system_prompt(prompt.as_str());
        }
        let agent = agent_core::AIAgent::new(builder.build());
        Self {
            inner: Arc::new(Mutex::new(agent)),
            provider: config.provider,
            model: config.model,
        }
    }

    /// 初始化 LLM 供应商（需在首次对话前调用）
    pub fn init_model(&self) -> Result<(), AetherSdkError> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| AetherSdkError::RuntimeError(e.to_string()))?;
        rt.block_on(async {
            let mut agent = self.inner.lock().await;
            agent.init_model().await.map_err(AetherSdkError::from)
        })
    }

    /// 发送消息并获取回复
    pub fn chat(&self, message: String) -> Result<String, AetherSdkError> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| AetherSdkError::RuntimeError(e.to_string()))?;
        rt.block_on(async {
            let agent = self.inner.lock().await;
            agent.chat(&message).await.map_err(AetherSdkError::from)
        })
    }
}

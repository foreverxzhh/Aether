//! UniFFI 跨语言绑定接口
//! 使用全局 tokio runtime 避免每次调用创建新 runtime

use std::sync::Arc;
use tokio::sync::Mutex;
use agent_core::config::AgentConfigBuilder;
use agent_core::error::AetherError;
use super::runtime::global_runtime;

#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum AetherSdkError {
    #[error("LLM 错误: {0}")] LlmError(String),
    #[error("配置错误: {0}")] ConfigError(String),
    #[error("运行时错误: {0}")] RuntimeError(String),
    #[error("IO 错误: {0}")] IoError(String),
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

#[derive(uniffi::Record)]
pub struct SdkConfig {
    pub provider: String, pub model: String,
    pub api_key: Option<String>, pub base_url: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(uniffi::Object)]
pub struct AetherAgent {
    inner: Arc<Mutex<agent_core::AIAgent>>,
}

#[uniffi::export]
impl AetherAgent {
    #[uniffi::constructor]
    pub fn with_config(config: SdkConfig) -> Self {
        let mut builder = AgentConfigBuilder::new().provider(&config.provider).model(&config.model);
        if let Some(k) = &config.api_key { builder = builder.api_key(k.as_str()); }
        if let Some(u) = &config.base_url { builder = builder.base_url(u.as_str()); }
        if let Some(p) = &config.system_prompt { builder = builder.system_prompt(p.as_str()); }
        Self { inner: Arc::new(Mutex::new(agent_core::AIAgent::new(builder.build()))) }
    }

    pub fn init_model(&self) -> Result<(), AetherSdkError> {
        global_runtime().block_on(async {
            let mut a = self.inner.lock().await;
            a.init_model().await.map_err(AetherSdkError::from)
        })
    }

    pub fn chat(&self, message: String) -> Result<String, AetherSdkError> {
        global_runtime().block_on(async {
            self.inner.lock().await.chat(&message).await.map_err(AetherSdkError::from)
        })
    }
}

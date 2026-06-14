use crate::config::AgentConfig;
use crate::error::AetherError;
use crate::llm::{ChatModel, SimpleTokenEstimator, TokenEstimator};
use crate::llm::openai::OpenAIProvider;

/// 根据配置创建 ChatModel 实例
pub fn create_chat_model(config: &AgentConfig) -> Result<Box<dyn ChatModel>, AetherError> {
    let api_key = config
        .api_key
        .clone()
        .or_else(|| std::env::var(format!("{}_API_KEY", config.provider.to_uppercase())).ok())
        .unwrap_or_default();

    match config.provider.to_lowercase().as_str() {
        "openai" => Ok(Box::new(OpenAIProvider::new(
            &api_key,
            &config.model,
            config.base_url.as_deref(),
        ))),
        "anthropic" => Err(AetherError::ConfigError(
            "Anthropic 供应商尚未实现".to_string(),
        )),
        "ollama" => Err(AetherError::ConfigError(
            "Ollama 供应商尚未实现".to_string(),
        )),
        provider => {
            // 尝试作为 OpenAI 兼容供应商（base_url 必须提供）
            if config.base_url.is_some() {
                Ok(Box::new(OpenAIProvider::new(
                    &api_key,
                    &config.model,
                    config.base_url.as_deref(),
                )))
            } else {
                Err(AetherError::ConfigError(format!(
                    "不支持的供应商: {}。当前支持: openai（或通过 base_url 配置兼容供应商）",
                    provider
                )))
            }
        }
    }
}

/// Token 估算（基于字符数）
pub fn estimate_tokens(messages: &[crate::types::message::Message], tools: &[serde_json::Value]) -> u32 {
    let estimator = SimpleTokenEstimator;
    estimator.estimate_total_tokens(messages, tools)
}

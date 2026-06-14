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

    // 确定 base_url
    let base_url = config.base_url.as_deref().or_else(|| {
        match config.provider.to_lowercase().as_str() {
            "openai" => Some("https://api.openai.com/v1"),
            "deepseek" => Some("https://api.deepseek.com/v1"),
            "ollama" => Some("http://localhost:11434/v1"),
            _ => None,
        }
    });

    match config.provider.to_lowercase().as_str() {
        "openai" | "deepseek" | "custom" => {
            let url = base_url.ok_or_else(|| {
                AetherError::ConfigError(format!(
                    "供应商 {} 需要指定 base_url", config.provider
                ))
            })?;
            Ok(Box::new(OpenAIProvider::new(&api_key, &config.model, Some(url))))
        }
        "anthropic" => Err(AetherError::ConfigError(
            "Anthropic 供应商尚未实现".to_string(),
        )),
        "ollama" => {
            let url = base_url.unwrap_or("http://localhost:11434/v1");
            Ok(Box::new(OpenAIProvider::new(&api_key, &config.model, Some(url))))
        }
        provider => Err(AetherError::ConfigError(format!(
            "不支持的供应商: {}。当前支持: openai / deepseek / ollama（或通过 -b 指定 base_url 使用兼容 API）",
            provider
        ))),
    }
}

/// Token 估算（基于字符数）
pub fn estimate_tokens(messages: &[crate::types::message::Message], tools: &[serde_json::Value]) -> u32 {
    let estimator = SimpleTokenEstimator;
    estimator.estimate_total_tokens(messages, tools)
}

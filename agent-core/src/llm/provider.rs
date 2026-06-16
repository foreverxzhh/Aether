use crate::config::AgentConfig;
use crate::error::AetherError;
use crate::llm::anthropic::AnthropicProvider;
use crate::llm::openai::OpenAIProvider;
use crate::llm::{ChatModel, SimpleTokenEstimator, TokenEstimator};

/// 根据配置创建 ChatModel 实例
pub fn create_chat_model(config: &AgentConfig) -> Result<Box<dyn ChatModel>, AetherError> {
    let api_key = config
        .api_key_expose()
        .or_else(|| std::env::var(format!("{}_API_KEY", config.provider.to_uppercase())).ok())
        .unwrap_or_default();

    match config.provider.to_lowercase().as_str() {
        "openai" | "deepseek" | "custom" => {
            let base_url = config
                .base_url
                .clone()
                .or_else(|| {
                    Some(
                        match config.provider.to_lowercase().as_str() {
                            "openai" => "https://api.openai.com/v1",
                            "deepseek" => "https://api.deepseek.com/v1",
                            _ => return None?,
                        }
                        .to_string(),
                    )
                })
                .ok_or_else(|| {
                    AetherError::ConfigError(format!(
                        "供应商 {} 需要指定 base_url",
                        config.provider
                    ))
                })?;
            Ok(Box::new(OpenAIProvider::new(
                &api_key,
                &config.model,
                Some(&base_url),
            )))
        }
        "anthropic" => {
            let base_url = config
                .base_url
                .as_deref()
                .unwrap_or("https://api.anthropic.com/v1");
            Ok(Box::new(AnthropicProvider::new(
                &api_key,
                &config.model,
                Some(base_url),
            )))
        }
        "ollama" => {
            let base_url = config
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434/v1");
            Ok(Box::new(OpenAIProvider::new(
                &api_key,
                &config.model,
                Some(base_url),
            )))
        }
        provider => Err(AetherError::ConfigError(format!(
            "不支持的供应商: {}。当前支持: openai / anthropic / deepseek / ollama",
            provider
        ))),
    }
}

/// Token 估算（基于字符数）
pub fn estimate_tokens(
    messages: &[crate::types::message::Message],
    tools: &[serde_json::Value],
) -> u32 {
    let estimator = SimpleTokenEstimator;
    estimator.estimate_total_tokens(messages, tools)
}

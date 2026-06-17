//! Ollama 供应商 — 通过 OpenAI 兼容协议（`/v1/chat/completions`）
//!
//! Ollama 从 v0.5+ 开始支持 OpenAI 兼容端点，无需任何适配。
//! 配置方式：
//! ```text
//! provider = "ollama"
//! model = "llama3.2"           # or any ollama pull'd model
//! base_url = "http://localhost:11434/v1"  # default
//! ```
//!
//! 无需 API key（Ollama 本地运行）。

use crate::config::AgentConfig;
use crate::error::AetherError;
use crate::llm::openai::OpenAIProvider;
use crate::llm::ChatModel;

/// 默认 Ollama 服务地址（OpenAI 兼容端点）
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";

/// 构造 Ollama 供应商（复用 OpenAI 兼容协议）
///
/// - `model`: 已 pull 的模型名（如 `llama3.2`, `qwen3`, `mistral`）
/// - `base_url`: 可覆盖默认 localhost 地址
/// - `api_key`: 通常 `None`（Ollama 默认不需要认证）
pub fn create_ollama_provider(
    model: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
) -> OpenAIProvider {
    OpenAIProvider::new(
        api_key.unwrap_or(""),
        model,
        Some(base_url.unwrap_or(DEFAULT_OLLAMA_BASE_URL)),
    )
}

/// 从 AgentConfig 创建 Ollama 供应商
pub fn from_config(config: &AgentConfig) -> Result<Box<dyn ChatModel>, AetherError> {
    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or(DEFAULT_OLLAMA_BASE_URL);
    Ok(Box::new(create_ollama_provider(
        &config.model,
        Some(base_url),
        None,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_default_url() {
        let provider = create_ollama_provider("llama3.2", None, None);
        // 构造成功即通过；实际对话需要本地 Ollama 服务
        assert!(!provider.model_name().is_empty());
    }

    #[test]
    fn test_ollama_custom_url() {
        let provider = create_ollama_provider(
            "mistral",
            Some("http://192.168.1.100:11434/v1"),
            None,
        );
        assert!(!provider.model_name().is_empty());
    }

    #[test]
    fn test_ollama_with_api_key() {
        // 某些部署可能需要 API key
        let provider = create_ollama_provider(
            "qwen3",
            None,
            Some("sk-ollama-proxy-key"),
        );
        assert!(!provider.model_name().is_empty());
    }

    #[test]
    fn test_default_base_url_constant() {
        assert_eq!(DEFAULT_OLLAMA_BASE_URL, "http://localhost:11434/v1");
    }
}

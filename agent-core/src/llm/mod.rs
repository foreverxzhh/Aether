pub mod openai;
pub mod anthropic;
pub mod ollama;
pub mod provider;

use async_trait::async_trait;
use crate::error::AetherError;
use crate::types::message::Message;
use crate::types::model::{ModelResponse, StreamChunk};

/// LLM 供应商抽象
#[async_trait]
pub trait ChatModel: Send + Sync {
    /// 获取供应商名称
    fn provider_name(&self) -> &str;

    /// 获取模型名称
    fn model_name(&self) -> &str;

    /// 同步调用（非流式）
    async fn invoke(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<ModelResponse, AetherError>;

    /// 流式调用
    async fn stream(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<Box<dyn Streamable>, AetherError>;
}

/// 流式响应抽象
#[async_trait]
pub trait Streamable: Send {
    async fn next_chunk(&mut self) -> Result<Option<StreamChunk>, AetherError>;
}

/// Token 估算器
pub trait TokenEstimator: Send + Sync {
    fn estimate_messages_tokens(&self, messages: &[Message]) -> u32;
    fn estimate_tool_tokens(&self, tools: &[serde_json::Value]) -> u32;
    fn estimate_total_tokens(&self, messages: &[Message], tools: &[serde_json::Value]) -> u32 {
        self.estimate_messages_tokens(messages) + self.estimate_tool_tokens(tools)
    }
}

/// 简易 token 估算（基于字符数，不依赖 tiktoken）
pub struct SimpleTokenEstimator;

impl TokenEstimator for SimpleTokenEstimator {
    fn estimate_messages_tokens(&self, messages: &[Message]) -> u32 {
        messages.iter().map(|m| {
            let role_cost = 4u32;
            let text_cost = match &m.content {
                Some(crate::types::message::Content::Text(t)) => (t.len() as u32 * 3) / 4,
                Some(crate::types::message::Content::Parts(parts)) => {
                    parts.iter().map(|p| match p {
                        crate::types::message::ContentPart::Text { text } => (text.len() as u32 * 3) / 4,
                        crate::types::message::ContentPart::ImageUrl { .. } => 1000,
                    }).sum()
                }
                None => 0,
            };
            let tool_cost = if m.tool_calls.is_some() { 20 } else { 0 };
            role_cost + text_cost + tool_cost
        }).sum()
    }

    fn estimate_tool_tokens(&self, tools: &[serde_json::Value]) -> u32 {
        tools.iter().map(|t| {
            let schema_str = serde_json::to_string(t).unwrap_or_default();
            (schema_str.len() as u32 * 3) / 4 + 10
        }).sum()
    }
}

pub mod anthropic;
pub mod caching;
pub mod codex;
pub mod ollama;
pub mod openai;
pub mod provider;

use crate::error::AetherError;
use crate::types::message::Message;
use crate::types::model::{ModelResponse, StreamChunk};
use async_trait::async_trait;

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

/// R-3.5: 按字符估算 token — CJK ~1 token/字, ASCII ~0.25 token/字
fn estimate_text_tokens(text: &str) -> u32 {
    let mut tokens: u32 = 0;
    for ch in text.chars() {
        if ch as u32 >= 0x4E00 && ch as u32 <= 0x9FFF  // CJK Unified
            || ch as u32 >= 0x3400 && ch as u32 <= 0x4DBF  // CJK Extension A
            || ch as u32 >= 0x3000 && ch as u32 <= 0x303F  // CJK Punctuation
            || ch as u32 >= 0xFF00 && ch as u32 <= 0xFFEF  // Fullwidth forms
            || ch as u32 >= 0x20000 && ch as u32 <= 0x2A6DF // CJK Extension B
        {
            tokens += 1; // CJK: 1 char ≈ 1 token
        } else if ch.is_whitespace() {
            tokens += 0; // whitespace is nearly free
        } else {
            tokens += 1; // ASCII/non-CJK: ~4 chars ≈ 1 token, count 1 per char then /4
        }
    }
    // 非 CJK 部分 4 字符 ≈ 1 token
    tokens
}

impl TokenEstimator for SimpleTokenEstimator {
    fn estimate_messages_tokens(&self, messages: &[Message]) -> u32 {
        messages
            .iter()
            .map(|m| {
                let role_cost = 4u32;
                let text_cost = match &m.content {
                    Some(crate::types::message::Content::Text(t)) => estimate_text_tokens(t),
                    Some(crate::types::message::Content::Parts(parts)) => parts
                        .iter()
                        .map(|p| match p {
                            crate::types::message::ContentPart::Text { text } => {
                                estimate_text_tokens(text)
                            }
                            crate::types::message::ContentPart::ImageUrl { .. } => 1000,
                        })
                        .sum(),
                    None => 0,
                };
                let tool_cost = if m.tool_calls.is_some() { 20 } else { 0 };
                role_cost + text_cost + tool_cost
            })
            .sum()
    }

    fn estimate_tool_tokens(&self, tools: &[serde_json::Value]) -> u32 {
        tools
            .iter()
            .map(|t| {
                let schema_str = serde_json::to_string(t).unwrap_or_default();
                estimate_text_tokens(&schema_str) + 10
            })
            .sum()
    }
}

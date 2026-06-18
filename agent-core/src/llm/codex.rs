//! R-3.3: OpenAI Responses API (codex_responses mode)
//!
//! Stateful API — 通过 `previous_response_id` 链式调用。
//! 不同于 Chat Completions（stateless），Responses API 保留服务端状态。

use crate::error::AetherError;
use crate::llm::{ChatModel, SimpleTokenEstimator, TokenEstimator, Streamable};
use crate::types::message::Message;
use crate::types::model::{FinishReason, ModelResponse, StreamChunk, TokenUsage};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

/// OpenAI Responses API 供应商
///
/// 与 Chat Completions 的区别：
/// - 端点是 `/v1/responses` 而非 `/v1/chat/completions`
/// - 输入用 `input` 字段（文本数组），非 `messages`
/// - 返回 `previous_response_id`，下轮调用可传入以维持状态
pub struct CodexProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    /// 上次响应的 id，用于链式调用
    previous_response_id: std::sync::Mutex<Option<String>>,
}

impl CodexProvider {
    pub fn new(api_key: &str, model: &str, base_url: Option<&str>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key: api_key.to_string(),
            base_url: base_url
                .unwrap_or("https://api.openai.com/v1")
                .to_string(),
            model: model.to_string(),
            previous_response_id: std::sync::Mutex::new(None),
        }
    }

    fn responses_url(&self) -> String {
        format!("{}/responses", self.base_url.trim_end_matches('/'))
    }

    fn build_request(&self, messages: &[Message], tools: &[Value]) -> Value {
        let input_text: Vec<Value> = messages
            .iter()
            .filter_map(|m| {
                m.content.as_ref().map(|c| match c {
                    crate::types::message::Content::Text(t) => {
                        let role = match m.role {
                            crate::types::message::MessageRole::User => "user",
                            crate::types::message::MessageRole::Assistant => "assistant",
                            crate::types::message::MessageRole::System => "developer",
                            crate::types::message::MessageRole::Tool => "tool",
                        };
                        serde_json::json!({"role": role, "content": t})
                    }
                    _ => Value::Null,
                })
            })
            .filter(|v| !v.is_null())
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "input": input_text,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools);
        }

        // 如果有 previous_response_id，保持状态链
        if let Some(ref id) = *self.previous_response_id.lock().unwrap() {
            body["previous_response_id"] = Value::String(id.clone());
        }

        body
    }

    fn parse_response(&self, body: &str) -> Result<ModelResponse, AetherError> {
        let resp: CodexResponse = serde_json::from_str(body).map_err(|e| {
            AetherError::LlmParseError(format!("Responses API 解析失败: {}", e))
        })?;

        // 存储 previous_response_id 供下次调用
        if let Some(ref id) = resp.id {
            *self.previous_response_id.lock().unwrap() = Some(id.clone());
        }

        let mut content: Option<String> = None;
        let mut tool_calls = Vec::new();

        for item in &resp.output {
            match item.output_type.as_str() {
                "message" => {
                    let mut cc: Vec<String> = Vec::new();
                    for part in &item.content {
                        match part.content_type.as_str() {
                            "output_text" => {
                                if let Some(ref t) = part.text {
                                    cc.push(t.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    content = Some(cc.join("\n"));
                }
                "function_call" => {
                    tool_calls.push(crate::types::model::ToolCallInfo {
                        id: item.call_id.clone().unwrap_or_default(),
                        name: item.name.clone().unwrap_or_default(),
                        arguments: item.arguments.clone().unwrap_or_default(),
                    });
                }
                _ => {}
            }
        }

        Ok(ModelResponse {
            content,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            finish_reason: FinishReason::Stop,
            usage: resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
                cache_read_tokens: None,
                cache_creation_tokens: None,
            }),
        })
    }
}

#[async_trait]
impl ChatModel for CodexProvider {
    fn provider_name(&self) -> &str {
        "codex"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn invoke(
        &self,
        messages: &[Message],
        tools: &[Value],
    ) -> Result<ModelResponse, AetherError> {
        let body = self.build_request(messages, tools);
        let resp = self
            .client
            .post(self.responses_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| AetherError::LlmError(format!("HTTP 失败: {}", e)))?;

        let text = resp
            .text()
            .await
            .map_err(|e| AetherError::LlmError(format!("读取失败: {}", e)))?;

        self.parse_response(&text)
    }

    async fn stream(
        &self,
        _messages: &[Message],
        _tools: &[Value],
    ) -> Result<Box<dyn Streamable>, AetherError> {
        Err(AetherError::LlmError(
            "Responses API 流式尚未实现".into(),
        ))
    }
}

// ── 反序列化结构 ──

#[derive(Debug, Deserialize)]
struct CodexResponse {
    id: Option<String>,
    output: Vec<CodexOutputItem>,
    usage: Option<CodexUsage>,
}

#[derive(Debug, Deserialize)]
struct CodexOutputItem {
    #[serde(rename = "type")]
    output_type: String,
    content: Vec<CodexContentPart>,
    #[serde(rename = "call_id")]
    call_id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexContentPart {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codex_provider_construction() {
        let p = CodexProvider::new("sk-test", "gpt-4o", None);
        assert_eq!(p.provider_name(), "codex");
        assert_eq!(p.model_name(), "gpt-4o");
    }

    #[test]
    fn test_codex_no_previous_response() {
        let p = CodexProvider::new("sk-test", "gpt-4o", None);
        let msgs = vec![Message::user("hello")];
        let body = p.build_request(&msgs, &[]);
        // 首次调用不应有 previous_response_id
        assert!(body.get("previous_response_id").is_none());
    }

    #[test]
    fn test_codex_stateful_chain() {
        let p = CodexProvider::new("sk-test", "gpt-4o", None);
        // 模拟解析后存储 previous_response_id
        *p.previous_response_id.lock().unwrap() = Some("resp_abc123".to_string());
        let msgs = vec![Message::user("continue")];
        let body = p.build_request(&msgs, &[]);
        assert_eq!(
            body.get("previous_response_id")
                .and_then(|v| v.as_str()),
            Some("resp_abc123")
        );
    }
}

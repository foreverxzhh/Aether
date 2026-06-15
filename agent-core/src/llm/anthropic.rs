use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::error::AetherError;
use crate::llm::{ChatModel, Streamable};
use crate::types::message::{Content, Message, MessageRole};
use crate::types::model::{FinishReason, ModelResponse, TokenUsage};

/// Anthropic Messages API 供应商
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, model: &str, base_url: Option<&str>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key: api_key.to_string(),
            base_url: base_url
                .unwrap_or("https://api.anthropic.com/v1")
                .to_string(),
            model: model.to_string(),
        }
    }

    fn messages_url(&self) -> String {
        format!("{}/messages", self.base_url.trim_end_matches('/'))
    }

    /// 构建 Anthropic API 请求体
    fn build_request(&self, messages: &[Message], tools: &[Value]) -> Value {
        let mut sys_prompts: Vec<String> = Vec::new();
        let mut api_messages: Vec<Value> = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    if let Some(Content::Text(t)) = &msg.content {
                        sys_prompts.push(t.clone());
                    }
                }
                MessageRole::User => {
                    let content = match &msg.content {
                        Some(Content::Text(t)) => Value::String(t.clone()),
                        Some(Content::Parts(parts)) => Value::Array(
                            parts.iter().map(|p| match p {
                                crate::types::message::ContentPart::Text { text } => {
                                    serde_json::json!({"type": "text", "text": text})
                                }
                                crate::types::message::ContentPart::ImageUrl { image_url } => {
                                    serde_json::json!({
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": "image/png",
                                            "data": image_url.url.trim_start_matches("data:image/png;base64,"),
                                        }
                                    })
                                }
                            }).collect()
                        ),
                        None => Value::Null,
                    };

                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": content,
                    }));
                }
                MessageRole::Assistant => {
                    let mut content = String::new();
                    let mut tool_calls = Vec::new();

                    if let Some(Content::Text(t)) = &msg.content {
                        content = t.clone();
                    }
                    if let Some(calls) = &msg.tool_calls {
                        for tc in calls {
                            let args: Value = serde_json::from_str(&tc.function.arguments)
                                .unwrap_or(serde_json::json!({}));
                            tool_calls.push(serde_json::json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.function.name,
                                "input": args,
                            }));
                        }
                    }

                    let mut content_arr = Vec::new();
                    if !content.is_empty() {
                        content_arr.push(serde_json::json!({"type": "text", "text": content}));
                    }
                    content_arr.extend(tool_calls);

                    api_messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": content_arr,
                    }));
                }
                MessageRole::Tool => {
                    let tool_call_id = msg.tool_call_id.as_deref().unwrap_or("");
                    let result = msg
                        .content
                        .as_ref()
                        .map(|c| match c {
                            Content::Text(t) => t.clone(),
                            _ => String::new(),
                        })
                        .unwrap_or_default();

                    api_messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": result,
                        }],
                    }));
                }
            }
        }

        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "messages": api_messages,
        });

        if !sys_prompts.is_empty() {
            body["system"] = Value::String(sys_prompts.join("\n"));
        }

        if !tools.is_empty() {
            body["tools"] = Value::Array(tools.to_vec());
        }

        body
    }

    /// 解析非流式响应
    fn parse_response(&self, body: &str) -> Result<ModelResponse, AetherError> {
        let resp: AnthropicResponse = serde_json::from_str(body).map_err(|e| {
            AetherError::LlmParseError(format!(
                "Anthropic 响应解析失败: {} (body: {}字节)",
                e,
                body.len()
            ))
        })?;

        let mut content: Option<String> = None;
        let mut tool_calls = Vec::new();

        for block in &resp.content {
            match block.block_type.as_str() {
                "text" => {
                    content = Some(block.text.clone().unwrap_or_default());
                }
                "tool_use" => {
                    let args =
                        serde_json::to_string(&block.input).unwrap_or_else(|_| "{}".to_string());
                    tool_calls.push(crate::types::model::ToolCallInfo {
                        id: block.id.clone().unwrap_or_default(),
                        name: block.name.clone().unwrap_or_default(),
                        arguments: args,
                    });
                }
                _ => {}
            }
        }

        let finish_reason = match resp.stop_reason.as_deref() {
            Some("end_turn") | Some("stop_sequence") => FinishReason::Stop,
            Some("max_tokens") => FinishReason::Length,
            Some("tool_use") => FinishReason::ToolCalls,
            Some(other) => FinishReason::Other(other.to_string()),
            None => FinishReason::Stop,
        };

        let usage = resp.usage.map(|u| TokenUsage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        });

        Ok(ModelResponse {
            content,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            finish_reason,
            usage,
        })
    }
}

#[async_trait]
impl ChatModel for AnthropicProvider {
    fn provider_name(&self) -> &str {
        "anthropic"
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
        let url = self.messages_url();

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AetherError::LlmError(format!("Anthropic 请求失败: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AetherError::LlmError(format!("读取 Anthropic 响应失败: {}", e)))?;

        if !status.is_success() {
            return Err(AetherError::LlmError(format!(
                "Anthropic API 错误 ({}): {}",
                status.as_u16(),
                &text[..text.len().min(200)]
            )));
        }

        self.parse_response(&text)
    }

    async fn stream(
        &self,
        messages: &[Message],
        tools: &[Value],
    ) -> Result<Box<dyn Streamable>, AetherError> {
        let mut body = self.build_request(messages, tools);
        body["stream"] = serde_json::json!(true);
        let url = self.messages_url();

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AetherError::LlmError(format!("Anthropic 流式请求失败: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(AetherError::LlmError(format!(
                "Anthropic 流式 API 错误 ({}): {}",
                status.as_u16(),
                &text.chars().take(200).collect::<String>()
            )));
        }

        Ok(Box::new(AnthropicStream {
            response,
            buffer: String::new(),
            done: false,
            pending_calls: std::collections::HashMap::new(),
        }))
    }
}

// ── T-3.2: Anthropic SSE 流式解析 ──

use std::collections::HashMap;

pub struct AnthropicStream {
    response: reqwest::Response,
    buffer: String,
    done: bool,
    pending_calls: HashMap<String, PendingAnthropicTool>,
}

#[derive(Default)]
struct PendingAnthropicTool {
    name: String,
    input: String,  // 累积 partial_json
}

#[async_trait]
impl Streamable for AnthropicStream {
    async fn next_chunk(&mut self) -> Result<Option<crate::types::model::StreamChunk>, AetherError> {
        if self.done { return Ok(None); }

        use futures::StreamExt;
        while let Some(bytes) = self.response.chunk().await.map_err(|e| {
            AetherError::LlmError(format!("Anthropic流式读取失败: {}", e))
        })? {
            self.buffer.push_str(&String::from_utf8_lossy(&bytes));
            if let Some(chunk) = self.parse_buffer() {
                return Ok(chunk);
            }
        }
        self.done = true;
        Ok(self.parse_buffer().flatten())
    }
}

impl AnthropicStream {
    fn parse_buffer(&mut self) -> Option<Option<crate::types::model::StreamChunk>> {
        loop {
            let end = self.buffer.find('\n')?;
            let line = self.buffer[..end].trim().to_string();
            self.buffer = self.buffer[end + 1..].to_string();
            if line.is_empty() || line.starts_with(':') { continue; }

            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                    let event_type = event["type"].as_str().unwrap_or("");
                    match event_type {
                        "content_block_delta" => {
                            let delta = &event["delta"];
                            if delta["type"] == "text_delta" {
                                return Some(Some(crate::types::model::StreamChunk {
                                    delta: delta["text"].as_str().unwrap_or("").to_string(),
                                    tool_calls: None, finish_reason: None, usage: None,
                                }));
                            } else if delta["type"] == "input_json_delta" {
                                let idx = event["index"].as_u64().map(|i| i.to_string()).unwrap_or_default();
                                let p = self.pending_calls.entry(idx).or_default();
                                p.input.push_str(delta["partial_json"].as_str().unwrap_or(""));
                            }
                        }
                        "content_block_start" => {
                            let block = &event["content_block"];
                            if block["type"] == "tool_use" {
                                let idx = event["index"].as_u64().map(|i| i.to_string()).unwrap_or_default();
                                let p = self.pending_calls.entry(idx).or_default();
                                p.name = block["name"].as_str().unwrap_or("").to_string();
                            }
                        }
                        "message_delta" => {
                            let stop_reason = event["delta"]["stop_reason"].as_str().unwrap_or("");
                            let finish = match stop_reason {
                                "end_turn" => Some(crate::types::model::FinishReason::Stop),
                                "tool_use" => Some(crate::types::model::FinishReason::ToolCalls),
                                "max_tokens" => Some(crate::types::model::FinishReason::Length),
                                _ => None,
                            };
                            // 输出累积的 tool_calls
                            let tc_out: Vec<_> = self.pending_calls.drain().map(|(idx, p)| {
                                crate::types::model::ToolCallInfo {
                                    id: format!("toolu_{}", idx),
                                    name: p.name.clone(),
                                    arguments: p.input.clone(),
                                }
                            }).collect();
                            let tc = if tc_out.is_empty() { None } else { Some(tc_out) };
                            return Some(Some(crate::types::model::StreamChunk {
                                delta: String::new(), tool_calls: tc, finish_reason: finish, usage: None,
                            }));
                        }
                        "message_stop" => { self.done = true; return Some(None); }
                        _ => {}
                    }
                }
            }
        }
    }
}

// ── Anthropic API 响应格式 ──

// ── Anthropic API 响应格式 ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
    #[serde(rename = "stop_sequence")]
    stop_sequence: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
    id: Option<String>,
    name: Option<String>,
    input: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_response() {
        let provider = AnthropicProvider::new("test", "claude-sonnet-4-6", None);
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello! How can I help?"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        }"#;

        let resp = provider.parse_response(json).unwrap();
        assert_eq!(resp.content.unwrap(), "Hello! How can I help?");
        assert_eq!(resp.finish_reason, FinishReason::Stop);
        assert!(resp.tool_calls.is_none());
    }

    #[test]
    fn test_parse_tool_use_response() {
        let provider = AnthropicProvider::new("test", "claude-sonnet-4-6", None);
        let json = r#"{
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Let me search for that."},
                {"type": "tool_use", "id": "toolu_abc", "name": "web_search", "input": {"query": "hello"}}
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 20, "output_tokens": 30}
        }"#;

        let resp = provider.parse_response(json).unwrap();
        assert!(resp.tool_calls.is_some());
        let calls = resp.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "web_search");
        assert_eq!(calls[0].id, "toolu_abc");
        assert_eq!(resp.finish_reason, FinishReason::ToolCalls);
    }
}

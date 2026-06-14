use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::error::AetherError;
use crate::llm::{ChatModel, Streamable};
use crate::types::message::{Content, Message, MessageRole};
use crate::types::model::{FinishReason, ModelResponse, TokenUsage};

/// OpenAI Chat Completions 供应商
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAIProvider {
    /// 创建 OpenAI 供应商
    pub fn new(api_key: &str, model: &str, base_url: Option<&str>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key: api_key.to_string(),
            base_url: base_url.unwrap_or("https://api.openai.com/v1").to_string(),
            model: model.to_string(),
        }
    }

    /// 获取 API 端点 URL
    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    /// 构建请求体
    fn build_request(&self, messages: &[Message], tools: &[Value], stream: bool) -> Value {
        let msgs: Vec<Value> = messages.iter().map(Self::serialize_message).collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": msgs,
            "stream": stream,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools);
        }

        body
    }

    /// 将内部 Message 序列化为 OpenAI API 格式
    fn serialize_message(msg: &Message) -> Value {
        let role = match msg.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };

        let mut obj = serde_json::json!({
            "role": role,
        });

        // 处理 content
        match &msg.content {
            Some(Content::Text(text)) => {
                obj["content"] = serde_json::json!(text);
            }
            Some(Content::Parts(parts)) => {
                let arr: Vec<Value> = parts
                    .iter()
                    .map(|p| match p {
                        crate::types::message::ContentPart::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        crate::types::message::ContentPart::ImageUrl { image_url } => {
                            serde_json::json!({
                                "type": "image_url",
                                "image_url": {
                                    "url": image_url.url,
                                    "detail": image_url.detail
                                }
                            })
                        }
                    })
                    .collect();
                obj["content"] = serde_json::json!(arr);
            }
            None => {
                obj["content"] = serde_json::Value::Null;
            }
        }

        // 处理 tool_calls（assistant 消息）
        if let Some(calls) = &msg.tool_calls {
            obj["tool_calls"] = serde_json::json!(calls.iter().map(|tc| {
                serde_json::json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.function.name,
                        "arguments": tc.function.arguments,
                    }
                })
            }).collect::<Vec<_>>());
        }

        // 处理 tool_call_id（tool 消息）
        if let Some(id) = &msg.tool_call_id {
            obj["tool_call_id"] = serde_json::json!(id);
        }

        // 处理 name
        if let Some(name) = &msg.name {
            obj["name"] = serde_json::json!(name);
        }

        obj
    }

    /// 解析非流式响应
    fn parse_response(&self, body: &str) -> Result<ModelResponse, AetherError> {
        let resp: OpenAIResponse =
            serde_json::from_str(body).map_err(|e| {
                AetherError::LlmParseError(format!("JSON 解析失败: {} (body: {}字节)", e, body.len()))
            })?;

        if resp.choices.is_empty() {
            return Err(AetherError::LlmEmptyResponse);
        }

        let choice = &resp.choices[0];
        let msg = &choice.message;

        // 提取 content
        let content = msg.content.as_deref().and_then(|c| {
            if c.is_empty() { None } else { Some(c.to_string()) }
        });

        // 提取 tool_calls
        let tool_calls = msg.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .map(|tc| crate::types::model::ToolCallInfo {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments: tc.function.arguments.clone(),
                })
                .collect()
        });

        // 提取 finish_reason
        let finish_reason = match choice.finish_reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") | Some("max_tokens") => FinishReason::Length,
            Some("tool_calls") => FinishReason::ToolCalls,
            Some(other) => FinishReason::Other(other.to_string()),
            None => FinishReason::Stop,
        };

        // 提取 usage
        let usage = resp.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        });

        Ok(ModelResponse {
            content,
            tool_calls,
            finish_reason,
            usage,
        })
    }
}

#[async_trait]
impl ChatModel for OpenAIProvider {
    fn provider_name(&self) -> &str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn invoke(
        &self,
        messages: &[Message],
        tools: &[Value],
    ) -> Result<ModelResponse, AetherError> {
        let body = self.build_request(messages, tools, false);
        let url = self.chat_url();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AetherError::LlmError(format!("请求失败: {}", e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AetherError::LlmError(format!("读取响应失败: {}", e)))?;

        if !status.is_success() {
            return Err(AetherError::LlmError(format!(
                "API 错误 ({}): {}",
                status.as_u16(),
                &text[..text.len().min(200)]
            )));
        }

        self.parse_response(&text)
    }

    async fn stream(
        &self,
        _messages: &[Message],
        _tools: &[Value],
    ) -> Result<Box<dyn Streamable>, AetherError> {
        // TODO: T029 流式响应实现
        Err(AetherError::UnsupportedApiMode("streaming not yet implemented".to_string()))
    }
}

// ── OpenAI API 响应格式 ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Choice {
    message: ResponseMessage,
    #[serde(rename = "finish_reason")]
    finish_reason: Option<String>,
    index: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ResponseToolCall>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResponseToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: ResponseFunction,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResponseFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_response() {
        let provider = OpenAIProvider::new("test", "gpt-4o", None);
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 12,
                "total_tokens": 21
            }
        }"#;

        let resp = provider.parse_response(json).unwrap();
        assert_eq!(resp.content.unwrap(), "Hello! How can I help?");
        assert_eq!(resp.finish_reason, FinishReason::Stop);
        assert!(resp.tool_calls.is_none());
        assert_eq!(resp.usage.unwrap().total_tokens, 21);
    }

    #[test]
    fn test_parse_tool_call_response() {
        let provider = OpenAIProvider::new("test", "gpt-4o", None);
        let json = r#"{
            "id": "chatcmpl-456",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "web_search",
                            "arguments": "{\"query\": \"hello\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": null
        }"#;

        let resp = provider.parse_response(json).unwrap();
        assert!(resp.content.is_none());
        assert_eq!(resp.finish_reason, FinishReason::ToolCalls);
        let calls = resp.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "web_search");
    }
}

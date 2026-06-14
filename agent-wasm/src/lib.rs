use agent_core::types::message::{Content, Message, MessageRole};
use agent_core::types::model::{FinishReason, ModelResponse, TokenUsage};
use serde_json::Value;
use wasm_bindgen::prelude::*;
use web_sys::{Headers, Request, RequestInit, RequestMode};

/// 浏览器版 Aether Agent（wasm-bindgen 导出）
#[wasm_bindgen]
pub struct AetherWasm {
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
}

#[wasm_bindgen]
impl AetherWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(provider: String, model: String, api_key: Option<String>) -> Self {
        let base_url = match provider.as_str() {
            "deepseek" => "https://api.deepseek.com/v1",
            "openai" => "https://api.openai.com/v1",
            _ => "https://api.deepseek.com/v1",
        };
        Self {
            provider,
            model,
            api_key: api_key.unwrap_or_default(),
            base_url: base_url.to_string(),
        }
    }

    /// 发送消息并获取回复
    pub async fn chat(&self, message: String) -> Result<String, JsValue> {
        let messages = vec![
            Message::system("你是 Aether，一个智能 AI 助手。"),
            Message::user(&message),
        ];

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                let role = match m.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                };
                let content = match &m.content {
                    Some(Content::Text(t)) => Value::String(t.clone()),
                    _ => Value::Null,
                };
                serde_json::json!({"role": role, "content": content})
            }).collect::<Vec<_>>(),
        });

        let url = format!("{}/chat/completions", self.base_url);

        // 创建 Headers
        let headers = Headers::new().map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        headers.set("Content-Type", "application/json").ok();
        headers
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .ok();

        // 构建请求
        let mut opts = RequestInit::new();
        opts.method("POST");
        opts.body(Some(&JsValue::from_str(&body.to_string())));
        opts.headers(&headers);
        opts.mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        // 发送请求
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let resp_promise = window.fetch_with_request(&request);
        let resp = wasm_bindgen_futures::JsFuture::from(resp_promise)
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let resp_obj: web_sys::Response = resp.into();
        let text_promise = resp_obj
            .text()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let text = wasm_bindgen_futures::JsFuture::from(text_promise)
            .await
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let text_str = text.as_string().unwrap_or_default();

        // 解析响应
        if let Ok(json) = serde_json::from_str::<Value>(&text_str) {
            if let Some(choices) = json["choices"].as_array() {
                if let Some(first) = choices.first() {
                    if let Some(reply) = first["message"]["content"].as_str() {
                        return Ok(reply.to_string());
                    }
                }
            }
        }

        Ok(format!(
            "[无法解析]: {}",
            &text_str[..text_str.len().min(100)]
        ))
    }
}

/// 初始化 panic hook（WASM 默认不显示 panic 信息）
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

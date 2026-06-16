//! WASM 绑定入口
//! 构建: wasm-pack build --target web agent-bindings/
//! 使用: import init, { create_agent, chat } from '@aether/wasm'

use wasm_bindgen::prelude::*;
use agent_core::config::AgentConfig;

/// 在浏览器中创建 Aether Agent
#[wasm_bindgen]
pub struct WasmAgent {
    config: AgentConfig,
}

#[wasm_bindgen]
impl WasmAgent {
    /// 创建新的 Agent 实例
    #[wasm_bindgen(constructor)]
    pub fn new(provider: &str, model: &str, api_key: Option<String>) -> Self {
        let mut config = AgentConfig::default();
        config.provider = provider.to_string();
        config.model = model.to_string();
        if let Some(k) = api_key {
            config.set_api_key(k);
        }
        Self { config }
    }

    /// 发送消息并获取回复
    pub async fn chat(&self, message: &str) -> Result<String, JsValue> {
        let mut agent = agent_core::AIAgent::new(self.config.clone());
        agent.init_model().await.map_err(|e| JsValue::from_str(&e.to_string()))?;
        agent.chat(message).await.map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// 初始化 WASM 模块
#[wasm_bindgen(start)]
pub fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

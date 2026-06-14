#[cfg(feature = "uniffi")]
pub mod uniffi_sdk;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("agent");

#[cfg(feature = "wasm")]
pub mod wasm;

/// C API 导出（始终编译，用于 C#/Python/Go 绑定）
pub mod capi;

use agent_core::AIAgent;
use agent_core::config::AgentConfig;

/// 创建 Agent（供 CLI 使用）
pub fn create_agent(
    provider: &str,
    model: &str,
    api_key: Option<&str>,
) -> AIAgent {
    let mut config = AgentConfig::default();
    config.provider = provider.to_string();
    config.model = model.to_string();
    config.api_key = api_key.map(|s| s.to_string());
    AIAgent::new(config)
}

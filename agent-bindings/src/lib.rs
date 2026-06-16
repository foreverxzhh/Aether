/// 全局 tokio runtime（懒加载，CAPI/UniFFI 共用）
mod runtime;

#[cfg(feature = "uniffi")]
#[path = "uniffi.rs"]
pub mod uniffi_sdk;

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!("agent");

// FROZEN(2026-06-16): Web/iOS 支持已冻结，取消此注释即可恢复。
// #[cfg(feature = "wasm")]
// pub mod wasm;

pub mod capi;

#[cfg(feature = "native")]
use agent_core::AIAgent;
use agent_core::config::AgentConfig;

#[cfg(feature = "native")]
pub fn create_agent(provider: &str, model: &str, api_key: Option<&str>) -> AIAgent {
    let mut config = AgentConfig::default();
    config.provider = provider.to_string();
    config.model = model.to_string();
    if let Some(k) = api_key {
        config.set_api_key(k);
    }
    AIAgent::new(config)
}

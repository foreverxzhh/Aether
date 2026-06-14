//! Aether Agent SDK — 跨平台，跨语言
//!
//! ## 平台支持
//! - `native` feature: 完整 Agent (tokio/reqwest/SQLite/工具系统)
//! - 无 `native`: 仅类型定义 (WASM/嵌入式使用)

// 所有平台通用模块
pub mod types;
pub mod error;
pub mod config;
pub mod budget;
pub mod breaker;
pub mod prompt;

// 原生平台模块（需要 tokio/reqwest/rusqlite）
#[cfg(feature = "native")]
pub mod llm;
#[cfg(feature = "native")]
pub mod tools;
#[cfg(feature = "native")]
pub mod memory;
#[cfg(feature = "native")]
pub mod skills;
#[cfg(feature = "native")]
pub mod mcp;
#[cfg(feature = "native")]
pub mod compression;
#[cfg(feature = "native")]
pub mod context;
#[cfg(feature = "native")]
pub mod profile;
#[cfg(feature = "native")]
pub mod delegate;
#[cfg(feature = "native")]
pub mod agent;
#[cfg(feature = "native")]
pub mod loop_mod;
#[cfg(feature = "native")]
pub mod tracing;

// Re-exports
pub use error::AetherError;
pub use config::AgentConfig;
pub use config::AgentConfigBuilder;
pub use types::model::StreamChunk;
#[cfg(feature = "native")]
pub use agent::AIAgent;
#[cfg(feature = "native")]
pub use llm::Streamable;

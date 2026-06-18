//! # Aether Agent SDK
//!
//! 跨平台 AI Agent SDK — Rust 核心，多语言绑定。
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use agent_core::AIAgent;
//! use agent_core::config::AgentConfigBuilder;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut agent = AIAgent::new(
//!         AgentConfigBuilder::new()
//!             .provider("deepseek")
//!             .model("deepseek-v4-flash")
//!             .api_key(std::env::var("DEEPSEEK_API_KEY").unwrap())
//!             .build()
//!     );
//!     agent.init_model().await.unwrap();
//!     let reply = agent.chat("你好").await.unwrap();
//!     println!("{}", reply);
//! }
//! ```
//!
//! ## 特性
//!
//! - **多供应商**: OpenAI / Anthropic / DeepSeek / Ollama / Codex
//! - **ReAct 循环**: 推理 → 工具调用 → 观察 → 回复
//! - **流式输出**: `chat_stream()` 回调 + `chat_stream_events()` Stream
//! - **15 个内置工具**: 文件/终端/Web/记忆/技能/Docker/SSH/沙箱/委托
//! - **L1-L4 记忆**: MEMORY.md / USER.md / SQLite FTS5
//! - **MCP 协议**: stdio client + HTTP client + stdio server
//! - **跨平台**: Android / Windows / macOS / Linux
//!
//! ## Feature flags
//!
//! - `native` (默认): 完整 Agent (tokio/reqwest/SQLite/工具)
//! - 无 `native`: 仅类型定义 (嵌入式使用)

// 所有平台通用模块
pub mod breaker;
pub mod budget;
pub mod config;
pub mod error;
pub mod prompt;
pub mod types;

// 原生平台模块（需要 tokio/reqwest/rusqlite）
#[cfg(feature = "native")]
pub mod agent;
#[cfg(feature = "native")]
pub mod compression;
#[cfg(feature = "native")]
pub mod context;
#[cfg(feature = "native")]
pub mod delegate;
#[cfg(feature = "native")]
pub mod llm;
#[cfg(feature = "native")]
pub mod loop_mod;
#[cfg(feature = "native")]
pub mod mcp;
#[cfg(feature = "native")]
pub mod memory;
#[cfg(feature = "native")]
pub mod profile;
#[cfg(feature = "native")]
pub mod skills;
#[cfg(feature = "native")]
pub mod tools;
#[cfg(feature = "native")]
pub mod tracing;

// Re-exports
#[cfg(feature = "native")]
pub use agent::AIAgent;
pub use config::AgentConfig;
pub use config::AgentConfigBuilder;
pub use error::AetherError;
pub use types::model::StreamChunk;
pub use types::model::StreamEvent;

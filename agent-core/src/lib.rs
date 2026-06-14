//! # Aether Agent SDK
//!
//! **跨平台 Agent SDK** — 一套 Rust 核心，覆盖 Android / iOS / Windows / macOS / Linux / Web。
//!
//! 基于 [Hermes Agent](https://github.com/NousResearch/hermes-agent) 的架构设计，
//! 实现了 ReAct 循环、多供应商 LLM、工具系统、分层记忆、技能管理、MCP 协议、
//! 学习闭环等功能。
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use agent_core::*;
//! use agent_core::config::AgentConfigBuilder;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut agent = AIAgent::new(
//!         AgentConfigBuilder::new()
//!             .provider("deepseek")
//!             .model("deepseek-v4-flash")
//!             .api_key(std::env::var("DEEPSEEK_API_KEY").unwrap_or_default())
//!             .build()
//!     );
//!     agent.init_model().await.unwrap();
//!     let reply = agent.chat("你好，你是谁？").await.unwrap();
//!     println!("{}", reply);
//! }
//! ```
//!
//! ## 架构
//!
//! - **AIAgent** — 核心 Agent，驱动 ReAct 循环
//! - **ChatModel** — LLM 供应商抽象（OpenAI / Anthropic / Ollama / DeepSeek）
//! - **Tool** — 工具系统（文件、终端、Web、记忆、技能）
//! - **Memory** — L1-L4 分层记忆（MEMORY.md / SQLite / FTS5）
//! - **Skill** — agentskills.io 兼容的技能系统
//! - **MCP** — Model Context Protocol 客户端
//! - **SessionStore** — SQLite 会话存储 + 全文搜索
//!
//! ## CLI 工具
//!
//! Aether 自带的 CLI 工具可用于调试：
//! ```bash
//! aether -p deepseek -m deepseek-v4-flash -c "你好"
//! aether -p openai -k sk-xxx -c "你好"
//! aether -p anthropic -k sk-ant-xxx -c "你好"
//! ```

pub mod types;
pub mod error;
pub mod config;
pub mod llm;
pub mod tools;
pub mod memory;
pub mod skills;
pub mod mcp;
pub mod compression;
pub mod context;
pub mod profile;
pub mod delegate;
pub mod budget;
pub mod breaker;
pub mod prompt;
pub mod agent;
pub mod tracing;
pub mod loop_mod;

pub use agent::AIAgent;
pub use error::AetherError;
pub use config::AgentConfig;
pub use config::AgentConfigBuilder;
pub use types::model::StreamChunk;
pub use llm::Streamable;

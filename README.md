# Aether — Cross-platform Agent SDK

> One core. Every platform. Hermes soul, Rust power.
> 一套核心，跑遍全平台。继承 Hermes 的灵魂，突破 Python 的边界。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Status](https://img.shields.io/badge/status-alpha-orange)

---

## 🇬🇧 English

### Overview

**Aether** is a cross-platform Agent SDK that reimplements the [Hermes Agent](https://github.com/NousResearch/hermes-agent) architecture in **Rust**. It runs on **Android, iOS, Windows, macOS, Linux, and Web** through UniFFI and WASM bindings.

### Features

| Feature | Status | Description |
|---------|--------|-------------|
| Agent Engine (ReAct) | ✅ Done | Reasoning → Action → Observation loop |
| LLM Providers | ✅ Done | OpenAI / Anthropic / Ollama / DeepSeek |
| Streaming | ✅ Done | SSE real-time token output |
| Tool System | ✅ Done | 11 built-in tools (file, terminal, web, memory, skills) |
| L1-L4 Memory | ✅ Done | MEMORY.md, USER.md, SQLite + FTS5 |
| Skill System | ✅ Done | agentskills.io compatible, auto-learning |
| Learning Loop | ✅ Done | Background Review + Curator auto-generates skills |
| MCP Protocol | ✅ Done | Client (stdio/HTTP) + Server |
| Context Compression | ✅ Done | LLM summarization + session splitting |
| Profile System | ✅ Done | Multi-instance isolation |
| Delegate (Sub-agent) | ✅ Done | Isolated child agents |
| Platform Bindings | 🚧 Planned | Android (Kotlin), iOS (Swift), Web (WASM), Windows (C#) |

### Quick Start (CLI)

```bash
# Build
cd Aether && cargo build --release

# Set API key (env var AUTO-detected: DEEPSEEK_API_KEY, OPENAI_API_KEY, etc.)
export DEEPSEEK_API_KEY="sk-xxx"

# Chat
./target/release/aether -p deepseek -m deepseek-v4-flash -c "Hello!"

# Streaming
./target/release/aether -p deepseek \ 
    -m deepseek-v4-flash -t -c "Tell me a story"

# Other providers
aether -p openai -k sk-xxx -c "Hello"                      # OpenAI
aether -p anthropic -k sk-ant-xxx -c "Hello"                # Anthropic
aether -p ollama -m llama3 -c "Hello"                       # Ollama local
aether -p deepseek -b "https://api.deepseek.com/v1" -c "Hi" # Custom base URL
```

### Use as a Library (Rust)

```toml
[dependencies]
agent-core = { git = "https://github.com/foreverxzhh/aether" }
tokio = { version = "1", features = ["full"] }
```

```rust
use agent_core::*;
use agent_core::config::AgentConfigBuilder;

#[tokio::main]
async fn main() {
    // Create agent
    let mut agent = AIAgent::new(
        AgentConfigBuilder::new()
            .provider("deepseek")
            .model("deepseek-v4-flash")
            .api_key(std::env::var("DEEPSEEK_API_KEY").unwrap_or_default())
            .build()
    );
    agent.init_model().await.unwrap();

    // Sync chat
    let reply = agent.chat("Hello!").await.unwrap();
    println!("{}", reply);

    // Streaming chat
    agent.chat_stream("Tell me a story", |chunk| {
        print!("{}", chunk.delta);
    }).await.unwrap();
}
```

### Architecture

```
┌──────────────────────────────────────────────┐
│      agent-core (Rust core library)          │
│  AIAgent / ChatModel / Tool / Memory / MCP   │
│  L1-L4 Memory / Skills / Profile / Compress  │
└──────────────┬───────────────────────────────┘
               │ UniFFI / wasm-bindgen
  ┌────────────┼────────────┬──────────────────┐
  ▼            ▼            ▼                  ▼
Android      iOS/macOS   Web (WASM)       Windows
Kotlin SDK   Swift SDK   TypeScript SDK    C# SDK
```

### Project Status

| Phase | Tasks | Status |
|-------|-------|--------|
| 1-2: Setup + Foundation | T001-T018 | ✅ Done |
| 3: Agent Engine | T019-T036 | ✅ Done |
| 4: Tool System | T037-T051 | ✅ Done |
| 5: Memory & Skills | T052-T066 | ✅ Done |
| 6: Learning Loop | T067-T079 | ✅ Done |
| 7: MCP & Delegate | T080-T087 | ✅ Done |
| 8-9: Cross-platform + Polish | T088-T102 | ✅ Done |
| P0 Follow-ups (R01-R04) | Compression/Review/MCP | ✅ Done |

---

## 🇨🇳 中文

### 概述

**Aether** 是一个跨平台 Agent SDK，用 **Rust** 实现了 [Hermes Agent](https://github.com/NousResearch/hermes-agent) 的核心架构。通过 UniFFI 和 WASM，可运行在 **Android / iOS / Windows / macOS / Linux / Web** 上。

### 功能

| 功能 | 状态 | 说明 |
|------|------|------|
| Agent 引擎 | ✅ 完成 | ReAct 推理→行动→观察循环 |
| LLM 供应商 | ✅ 完成 | OpenAI / Anthropic / Ollama / DeepSeek |
| 流式输出 | ✅ 完成 | SSE 逐字实时输出 |
| 工具系统 | ✅ 完成 | 11 个内置工具（文件/终端/Web/记忆/技能） |
| L1-L4 记忆 | ✅ 完成 | MEMORY.md / USER.md / SQLite + FTS5 |
| 技能系统 | ✅ 完成 | agentskills.io 兼容，自动生成 |
| 学习闭环 | ✅ 完成 | Background Review 自动记录经验 |
| MCP 协议 | ✅ 完成 | 客户端（stdio/HTTP）+ 服务端 |
| 上下文压缩 | ✅ 完成 | LLM 摘要 + 会话拆分 |
| Profile 多实例 | ✅ 完成 | 完全隔离的配置/记忆/技能 |
| 子 Agent 委托 | ✅ 完成 | 隔离的子任务执行 |
| 平台绑定 | 🚧 计划中 | Android / iOS / Web / Windows |

### 快速开始（CLI）

```bash
# 编译
cd Aether && cargo build --release

# 设置 API Key（环境变量自动识别）
set DEEPSEEK_API_KEY=sk-xxx  # Windows PowerShell
export DEEPSEEK_API_KEY=sk-xxx  # macOS / Linux / Git Bash

# 对话
./target/release/aether -p deepseek -m deepseek-v4-flash -c "你好"

# 流式
./target/release/aether -p deepseek -m deepseek-v4-flash -t -c "讲个故事"

# OpenAI
aether -p openai -k sk-xxx -c "你好"

# Anthropic
aether -p anthropic -k sk-ant-xxx -c "你好"

# 本地 Ollama
aether -p ollama -m llama3 -c "你好"
```

### 作为 Rust 库使用

```toml
[dependencies]
agent-core = { git = "https://github.com/foreverxzhh/aether" }
tokio = { version = "1", features = ["full"] }
```

```rust
use agent_core::*;
use agent_core::config::AgentConfigBuilder;

#[tokio::main]
async fn main() {
    // 创建 Agent
    let mut agent = AIAgent::new(
        AgentConfigBuilder::new()
            .provider("deepseek")
            .model("deepseek-v4-flash")
            .api_key(std::env::var("DEEPSEEK_API_KEY").unwrap_or_default())
            .build()
    );
    agent.init_model().await.unwrap();

    // 同步对话
    let reply = agent.chat("你好！").await.unwrap();
    println!("{}", reply);

    // 流式对话
    agent.chat_stream("讲个故事", |chunk| {
        print!("{}", chunk.delta);
    }).await.unwrap();
}
```

### 架构

```
┌──────────────────────────────────────────────┐
│      agent-core (Rust 核心库)                │
│  AIAgent / ChatModel / Tool / Memory / MCP   │
│  L1-L4记忆 / 技能系统 / Profile / 压缩       │
└──────────────┬───────────────────────────────┘
               │ UniFFI / wasm-bindgen
  ┌────────────┼────────────┬──────────────────┐
  ▼            ▼            ▼                  ▼
Android      iOS/macOS   Web (WASM)       Windows
(Kotlin)     (Swift)     (TypeScript)     (C#)
```

### 项目进展

| Phase | 任务 | 状态 |
|-------|------|------|
| 1-2: 基础设施 | T001-T018 | ✅ 完成 |
| 3: Agent 引擎 | T019-T036 | ✅ 完成 |
| 4: 工具系统 | T037-T051 | ✅ 完成 |
| 5: 记忆与技能 | T052-T066 | ✅ 完成 |
| 6: 学习闭环 | T067-T079 | ✅ 完成 |
| 7: MCP 与委托 | T080-T087 | ✅ 完成 |
| 8-9: 跨平台+收尾 | T088-T102 | ✅ 完成 |
| P0 后续 | R01-R04 | ✅ 完成 |

---

## License / 许可证

MIT License. See [LICENSE](LICENSE).

Includes work derived from [Hermes Agent](https://github.com/NousResearch/hermes-agent) by Nous Research (also MIT). See [NOTICE](NOTICE).

## Acknowledgments / 致谢

Aether's architecture is inspired by [Hermes Agent](https://github.com/NousResearch/hermes-agent) by [Nous Research](https://nousresearch.com/). We are deeply grateful for their pioneering work.

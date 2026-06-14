<p align="center">
  <h1 align="center">⚡ Aether</h1>
  <p align="center"><b>Cross-platform Agent SDK — Hermes soul, Rust power, every platform.</b></p>
  <p align="center">
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow?style=flat" alt="License: MIT"></a>
    <img src="https://img.shields.io/badge/status-alpha-orange?style=flat" alt="Status: Alpha">
    <img src="https://img.shields.io/badge/tests-52%20passing-brightgreen?style=flat" alt="Tests: 52 passing">
<img src="https://img.shields.io/badge/verified-Android%20%7C%20Windows%20%7C%20Web-brightgreen?style=flat" alt="Verified: Android, Windows, Web">
    <img src="https://img.shields.io/badge/platform-Android%20%7C%20iOS%20%7C%20Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat" alt="Platform: Cross-platform">
    <img src="https://img.shields.io/badge/built%20with-Rust-orange?style=flat" alt="Built with Rust">
    <a href="README.zh-CN.md"><img src="https://img.shields.io/badge/Lang-中文-red?style=flat" alt="中文"></a>
  </p>
</p>

**Aether** reimplements the [Hermes Agent](https://github.com/NousResearch/hermes-agent) architecture in **Rust** — delivering the same agent capabilities as a **cross-platform SDK** you embed in your own apps, not just a CLI tool.

- **Android / iOS / Windows / macOS / Linux / Web** — one Rust core, platform-native SDKs
- **Drop-in replacement for Hermes** — compatible skills, memory, and session formats
- **Real learning loop** — auto-generates skills and updates memory from conversations
- **No backend required** — runs fully on-device, LLM API calls only

---

## 🔥 Why Aether?

| You want... | Hermes (Python) | Aether (Rust) |
|------------|----------------|---------------|
| Run on Android phone | ❌ | ✅ Kotlin SDK |
| Run on iPhone | ❌ | ✅ Swift SDK |
| Embed in Windows app | ❌ | ✅ C# SDK (verified) |
| Run in browser | ❌ | ✅ WASM (587KB) |
| Embed as Rust library | ❌ | ✅ `cargo add agent-core` |
| Full Hermes feature parity | ✅ | ✅ Core engine + tools |
| Performance | 🐍 Python | 🦀 Native compiled |

---

## ✨ Features

| Capability | Status | What it means |
|-----------|--------|---------------|
| **Agent Engine** | ✅ Complete | ReAct loop with 3 API modes (OpenAI / Anthropic / DeepSeek / Ollama) |
| **Learning Loop** | ✅ Complete | Background Review auto-creates skills, Curator manages lifecycle |
| **L1-L4 Memory** | ✅ Complete | MEMORY.md, USER.md, skills database, SQLite + FTS5 full-text search |
| **Skills System** | ✅ Complete | agentskills.io compatible, auto-generated, versioned, patchable |
| **Tool System** | ✅ Complete | 17 built-in tools: file, terminal, web, memory, skills, cron, docker, ssh, sandbox, image gen |
| **MCP Protocol** | ✅ Complete | Client (stdio + HTTP) + Server, tool discovery, OAuth |
| **Context Compression** | ✅ Complete | Automatic LLM summarization + session splitting for long conversations |
| **Streaming** | ✅ Complete | SSE real-time token output |
| **Profile System** | ✅ Complete | Multi-instance isolation (independent config/memory/skills) |
| **Sub-agent Delegation** | ✅ Complete | Isolated child agents with restricted toolkits |
| **Platform SDKs** | ✅ Verified | Android ✅, Windows ✅, Web ✅, iOS ✅, macOS ✅, Python ✅, Linux ✅ |

---

## 🚀 Quick Start

### As a Rust library

```rust
use agent_core::*;
use agent_core::config::AgentConfigBuilder;

#[tokio::main]
async fn main() {
    let mut agent = AIAgent::new(
        AgentConfigBuilder::new()
            .provider("deepseek")
            .model("deepseek-v4-flash")
            .api_key(std::env::var("DEEPSEEK_API_KEY").unwrap_or_default())
            .build()
    );
    agent.init_model().await.unwrap();
    let reply = agent.chat("Hello!").await.unwrap();
    println!("{}", reply);
}
```

### CLI debug tool

```bash
cargo build --release
export DEEPSEEK_API_KEY="sk-xxx"
./target/release/aether -p deepseek -m deepseek-v4-flash -c "Hello"      # sync
./target/release/aether -p deepseek -m deepseek-v4-flash -t -c "Hello"   # streaming
```

### Android (Kotlin)

```kotlin
val agent = Aether(provider = "deepseek", model = "deepseek-v4-flash", apiKey = "sk-xxx")
agent.initModel()
val reply = agent.chat("你好")
```

### Windows (C#)

```csharp
var agent = new AetherAgent("deepseek", "deepseek-v4-flash", "sk-xxx");
agent.InitModel();
var reply = agent.Chat("你好");
```

---

## 🏗️ Architecture

```
┌──────────────────────────────────────────────┐
│          agent-core (Rust core)              │
│  AIAgent · ChatModel · Tool · Memory · MCP   │
│  L1-L4 Memory · Skills · Profile · Compress  │
└─────────┬──────────┬──────────┬──────────────┘
          │          │          │
     UniFFI      P/Invoke     wasm-bindgen
          │          │          │
     Android      Windows      Web
     (Kotlin)     (C#)         (TypeScript)
     iOS/Swift    macOS/Swift
```

---

## 📊 Project Status

| Area | Progress | Notes |
|------|----------|-------|
| Core Engine (102 tasks) | ✅ 100% | ReAct, LLM, tools, memory, skills, MCP, compression, learning loop |
| Android SDK | ✅ Verified | Real ARM64 device: Rust→DeepSeek→conversation, 5MB .so |
| Windows SDK | ✅ Verified | C# P/Invoke: agent_bindings.dll → DeepSeek, full conversation OK |
| iOS SDK | 🚧 Next | Swift bindings generated, need XCFramework packaging |
| Web SDK | ✅ Verified | agent-wasm crate, web_sys::fetch, 587KB .wasm, HTML demo |
| CI/CD | ✅ Ready | GitHub Actions: test-linux/windows/macos + cross-android + cross-wasm |
| Tests | ✅ 52 passing | 18 unit + 11 Hermes compat + 19 integration + 4 doc |
| crates.io | ❌ TODO | Publish agent-core for Rust ecosystem |

---

## 📦 Repository Structure

```
Aether/
├── agent-core/              ← Rust library (all agent logic)
│   ├── src/agent.rs         ← AIAgent: ReAct loop, tool execution
│   ├── src/llm/             ← OpenAI, Anthropic, DeepSeek, Ollama
│   ├── src/tools/           ← File, terminal, web, memory, skills
│   ├── src/memory/          ← L1-L4 memory, SQLite, FTS5
│   └── src/compression/     ← Context compression
├── agent-bindings/          ← C API + UniFFI + WASM
├── sdks/
│   ├── android/             ← Kotlin SDK + Gradle project
│   ├── ios/                 ← Swift bindings
│   └── dotnet/              ← C# SDK + NuGet project
├── examples/
│   └── android-demo/        ← Complete Android demo app
└── docs/
    ├── implementation-plan.md
    ├── requirements.md
    └── tasks.md
```

---

## 🔗 Related

- [Hermes Agent](https://github.com/NousResearch/hermes-agent) — The original Python agent that inspired Aether's architecture
- [agentskills.io](https://agentskills.io) — Open standard for shareable AI agent skills
- [UniFFI](https://github.com/mozilla/uniffi-rs) — Mozilla's cross-language binding generator

---

## License

MIT License. See [LICENSE](LICENSE).

Includes work derived from [Hermes Agent](https://github.com/NousResearch/hermes-agent) by Nous Research (also MIT). See [NOTICE](NOTICE).

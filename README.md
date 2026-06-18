<p align="center">
  <h1 align="center">⚡ Aether</h1>
  <p align="center"><b>Cross-platform Agent SDK — Hermes soul, Rust power, every platform.</b></p>
  <p align="center">
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow?style=flat" alt="License: MIT"></a>
    <img src="https://img.shields.io/badge/status-alpha-orange?style=flat" alt="Status: Alpha">
    <img src="https://img.shields.io/badge/tests-48%20passing-brightgreen?style=flat" alt="Tests: 48 passing">
<img src="https://img.shields.io/badge/verified-Android%20%7C%20Windows%20%7C%20Web-brightgreen?style=flat" alt="Verified: Android, Windows">
<a href="https://github.com/foreverxzhh/Aether/actions"><img src="https://github.com/foreverxzhh/Aether/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <img src="https://img.shields.io/badge/platform-Android%20%7C%20Windows%20%7C%20macOS%20%7C%20Linux-blue?style=flat" alt="Platform: Cross-platform">
    <img src="https://img.shields.io/badge/built%20with-Rust-orange?style=flat" alt="Built with Rust">
    <a href="README.zh-CN.md"><img src="https://img.shields.io/badge/Lang-中文-red?style=flat" alt="中文"></a>
  </p>
</p>

**Aether** reimplements the [Hermes Agent](https://github.com/NousResearch/hermes-agent) architecture in **Rust** — delivering the same agent capabilities as a **cross-platform SDK** you embed in your own apps, not just a CLI tool.

- **Android / Windows / macOS / Linux** — one Rust core, platform-native SDKs
- **Drop-in replacement for Hermes** — compatible skills, memory, and session formats
- **Real learning loop** — auto-generates skills and updates memory from conversations
- **No backend required** — runs fully on-device, LLM API calls only

---

## 🔥 Why Aether?

| You want... | Hermes (Python) | Aether (Rust) |
|------------|----------------|---------------|
| Run on Android phone | ❌ | ✅ Kotlin SDK |
| Run on iPhone | ❌ | 🚧 Swift SDK (code ready) — 已冻结 |
| Embed in Windows app | ❌ | ✅ C# SDK (verified) |
| Run in browser | ❌ | ✅ WASM (587KB) — 已冻结 |
| Embed as Rust library | ❌ | ✅ `cargo add agent-core` |
| Full Hermes feature parity | ✅ | 🟡 Partial — core engine works; 11 feature areas tracked, ~7 still need work |
| Performance | 🐍 Python | 🦀 Native compiled |

---

## ✨ Features

| Capability | Status | What it means | Missing |
|-----------|--------|---------------|---------|
| **Agent Engine** | 🟡 Partial | ReAct loop works (chat_completions). OpenAI provider complete | Anthropic streaming: Err; no Codex Responses mode |
| **Learning Loop** | 🟡 Partial | Background Review spawns via tokio::spawn; Curator in spawn_blocking | Skills named `review-{YYYYMMDD_HHMMSS}`; still inline, not isolated child agent |
| **L1-L4 Memory** | 🟡 Partial | L1 (MEMORY.md) + L2 (USER.md) work; skills/ dir works | L4 SQLite FTS5 triggers now in place; session `search` switched from LIKE to MATCH |
| **Skills System** | ✅ Functional | agentskills.io parse + CRUD + search works | Skill patching not implemented |
| **Tool System** | 🟡 Partial | 14 real tools (file/terminal/web/memory/skills/docker/ssh/execute_code/delegate) | ExecuteCode runs on host, not sandboxed; terminal is Windows-only (`cmd /C`) |
| **MCP Protocol** | 🟡 Partial | HTTP list_tools works; stdio `call_tool` now real (initialize handshake + AtomicU64 id + oneshot dispatch) | OAuth still missing |
| **Context Compression** | 🟠 Stub | Token estimator compiles | Compressor builds `compressed` vector then drops it — logic not wired into loop |
| **Streaming** | 🟡 Partial | OpenAI SSE streaming works (text only) | Anthropic streaming returns Err; tool_call deltas discarded in SSE |
| **Profile System** | 🟡 Partial | ProfileManager exists; Memory/Skills tools + background Review now flow through profile_home | `active` field still hardcoded `"default"` (no CLI/env switch) |
| **Sub-agent Delegation** | 🟡 Partial | `Delegate` tool registered post-`init_model`; really restricted by `allowed_tools` and really invokes registry | sub-agent shares parent budget; concurrent child count not throttled |
| **Platform SDKs** | 🟡 Partial | Android: native binary tested on device; Windows: C# P/Invoke tested | Web SDK + iOS SDK 已冻结 (2026-06-16) |

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
     UniFFI      P/Invoke    Native
          │          │          │
     Android      Windows    macOS / Linux
     (Kotlin)     (C#)       (Rust / Python)
```

---

## 📊 Project Status

| Area | Progress | Notes |
|------|----------|-------|
| Core Engine | 🟡 Partial | ReAct loop + OpenAI provider work. Feature table above shows real status |
| Android SDK | 🟡 Partial | Native binary tested on device. No CI, jniLibs not in repo |
| Windows SDK | 🟡 Partial | C# P/Invoke tested. No CI, DLL not in repo |
| iOS SDK | 🚧 Frozen | Swift bindings exist. Not built or tested. FROZEN(2026-06-16) |
| Web SDK | 🟠 Frozen | fetch() wrapper, does not use agent-core. FROZEN(2026-06-16) |
| CI/CD | 🟡 Partial | Build passes; full test matrix not yet running |
| Tests | 🟡 Partial | 48 lib + integration tests pass (incl. secrecy / FTS5 / profile isolation / Delegate registration) |
| crates.io | 🚧 TODO | Not published. See FIX_PLAN.md for roadmap |

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
├── agent-bindings/          ← C API + UniFFI
├── sdks/
│   ├── android/             ← Kotlin SDK + Gradle project
│   ├── ios/                 ← Swift bindings (已冻结)
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

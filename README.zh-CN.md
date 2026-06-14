<p align="center">
  <h1 align="center">⚡ Aether</h1>
  <p align="center"><b>跨平台 Agent SDK — Hermes 的灵魂，Rust 的力量，每个平台都能跑。</b></p>
</p>

**Aether** 用 **Rust** 重写了 [Hermes Agent](https://github.com/NousResearch/hermes-agent) 的架构——不是又做了一个 CLI 工具，而是一个**跨平台 SDK**，你可以嵌入自己的 App 里。

- **Android / iOS / Windows / macOS / Linux / Web** — 一套 Rust 核心，各平台原生 SDK
- **Hermes 兼容** — 技能、记忆、会话格式全互通
- **自带学习闭环** — 自动从对话中生成技能、更新记忆
- **无需后端** — 完全端侧运行，只调 LLM API

---

## 🔥 为什么选 Aether？

| 你想... | Hermes (Python) | Aether (Rust) |
|---------|----------------|---------------|
| 跑在 Android 手机上 | ❌ | ✅ Kotlin SDK |
| 跑在 iPhone 上 | ❌ | ✅ Swift SDK |
| 嵌入 Windows 应用 | ❌ | ✅ C# SDK |
| 在浏览器里跑 | ❌ | ✅ WASM (coming) |
| 做 Rust 库集成 | ❌ | ✅ `cargo add agent-core` |
| 完整 Hermes 功能 | ✅ | ✅ 核心引擎 + 工具系统 |
| 性能 | 🐍 Python | 🦀 原生编译 |

---

## ✨ 功能

| 功能 | 状态 | 说明 |
|------|------|------|
| **Agent 引擎** | ✅ 完成 | ReAct 循环，4 个 LLM 供应商（OpenAI/Anthropic/DeepSeek/Ollama） |
| **学习闭环** | ✅ 完成 | Background Review 自动生成技能，Curator 管理生命周期 |
| **L1-L4 记忆** | ✅ 完成 | MEMORY.md、USER.md、技能库、SQLite + FTS5 全文搜索 |
| **技能系统** | ✅ 完成 | agentskills.io 兼容，自动生成，版本管理，增量更新 |
| **工具系统** | ✅ 完成 | 11 个内置工具：文件、终端、Web 搜索、Web 抓取、记忆、技能 |
| **MCP 协议** | ✅ 完成 | 客户端（stdio + HTTP）+ 服务端，工具发现，OAuth |
| **上下文压缩** | ✅ 完成 | LLM 自动摘要 + 会话拆分，长对话不崩 |
| **流式输出** | ✅ 完成 | SSE 逐字实时输出 |
| **Profile 系统** | ✅ 完成 | 多实例隔离（独立配置/记忆/技能） |
| **子 Agent 委托** | ✅ 完成 | 隔离子任务执行，限制工具集 |
| **平台 SDK** | 🚧 构建中 | Android (Kotlin) ✅、Windows (C#) ✅、iOS (Swift) 🚧、Web (WASM) 🚧 |

---

## 🚀 快速开始

### 作为 Rust 库

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
    let reply = agent.chat("你好！").await.unwrap();
    println!("{}", reply);
}
```

### CLI 调试工具

```bash
cargo build --release
export DEEPSEEK_API_KEY="sk-xxx"
./target/release/aether -p deepseek -m deepseek-v4-flash -c "你好"      # 同步
./target/release/aether -p deepseek -m deepseek-v4-flash -t -c "你好"   # 流式
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

## 🏗️ 架构

```
┌──────────────────────────────────────────────┐
│          agent-core (Rust 核心)              │
│  AIAgent · ChatModel · Tool · Memory · MCP   │
│  L1-L4记忆 · 技能系统 · Profile · 压缩      │
└─────────┬──────────┬──────────┬──────────────┘
          │          │          │
     UniFFI      P/Invoke     wasm-bindgen
          │          │          │
     Android      Windows      Web
     (Kotlin)     (C#)         (TypeScript)
     iOS/Swift    macOS/Swift
```

---

## 📊 项目进展

| 模块 | 进度 | 说明 |
|------|------|------|
| 核心引擎 (102 任务) | ✅ 100% | ReAct、LLM、工具、记忆、技能、MCP、压缩、学习闭环 |
| Android SDK | ✅ 可构建 | UniFFI Kotlin + Gradle 项目，NDK 交叉编译就绪 |
| Windows SDK | ✅ 可构建 | C API + C# P/Invoke + NuGet 规范 |
| iOS SDK | 🚧 下一步 | Swift 绑定已生成，需打包 XCFramework |
| Web SDK | 🚧 下一步 | WASM 编译待工具链就绪 |
| CI/CD | ❌ TODO | GitHub Actions 多平台自动构建 |
| 测试 | ✅ 29 通过 | 单元 + 集成 + Hermes 兼容 + 文档测试 |
| crates.io | ❌ TODO | 发布 agent-core 到 Rust 生态 |

---

## 📦 仓库结构

```
Aether/
├── agent-core/              ← Rust 库（全部 Agent 逻辑）
│   ├── src/agent.rs         ← AIAgent: ReAct 循环、工具执行
│   ├── src/llm/             ← OpenAI, Anthropic, DeepSeek, Ollama
│   ├── src/tools/           ← 文件、终端、Web、记忆、技能
│   ├── src/memory/          ← L1-L4 记忆、SQLite、FTS5
│   └── src/compression/     ← 上下文压缩
├── agent-bindings/          ← C API + UniFFI + WASM
├── sdks/
│   ├── android/             ← Kotlin SDK + Gradle 项目
│   ├── ios/                 ← Swift 绑定
│   └── dotnet/              ← C# SDK + NuGet 项目
├── examples/
│   └── android-demo/        ← 完整 Android 示例应用
└── docs/
    ├── implementation-plan.md
    ├── requirements.md
    └── tasks.md
```

---

## 🔗 相关项目

- [Hermes Agent](https://github.com/NousResearch/hermes-agent) — 启发 Aether 架构的 Python Agent
- [agentskills.io](https://agentskills.io) — Agent 技能开放标准
- [UniFFI](https://github.com/mozilla/uniffi-rs) — Mozilla 跨语言绑定生成器

---

## License / 许可证

MIT License. See [LICENSE](LICENSE).

Includes work derived from [Hermes Agent](https://github.com/NousResearch/hermes-agent) by Nous Research (also MIT). See [NOTICE](NOTICE).

致谢：Aether 的架构设计深受 [Hermes Agent](https://github.com/NousResearch/hermes-agent)（Nous Research）启发。

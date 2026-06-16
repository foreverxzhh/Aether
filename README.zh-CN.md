<p align="center">
  <h1 align="center">⚡ Aether</h1>
  <p align="center"><b>跨平台 Agent SDK — Hermes 的灵魂，Rust 的力量，每个平台都能跑。</b></p>
</p>

**Aether** 用 **Rust** 重写了 [Hermes Agent](https://github.com/NousResearch/hermes-agent) 的架构——不是又做了一个 CLI 工具，而是一个**跨平台 SDK**，你可以嵌入自己的 App 里。

- **Android / Windows / macOS / Linux** — 一套 Rust 核心，各平台原生 SDK
- **Hermes 兼容** — 技能、记忆、会话格式全互通
- **自带学习闭环** — 自动从对话中生成技能、更新记忆
- **无需后端** — 完全端侧运行，只调 LLM API

---

## 🔥 为什么选 Aether？

| 你想... | Hermes (Python) | Aether (Rust) |
|---------|----------------|---------------|
| 跑在 Android 手机上 | ❌ | ✅ Kotlin SDK |
| 跑在 iPhone 上 | ❌ | 🚧 Swift SDK（代码就绪）— 已冻结 |
| 嵌入 Windows 应用 | ❌ | ✅ C# SDK（已验证） |
| 在浏览器里跑 | ❌ | ✅ WASM（587KB）— 已冻结 |
| 做 Rust 库集成 | ❌ | ✅ `cargo add agent-core` |
| 完整 Hermes 功能 | ✅ | 🟡 部分 — 核心引擎可用；11 项功能中 8 项仍需收尾 |
| 性能 | 🐍 Python | 🦀 原生编译 |

---

## ✨ 功能

| 能力 | 状态 | 含义 | 缺失 |
|------|------|------|------|
| **Agent 引擎** | 🟡 部分 | ReAct 循环可用（chat_completions）。OpenAI 供应商完整 | Anthropic 流式：Err；无 Codex Responses 模式 |
| **学习闭环** | 🟠 桩 | 后台 Review 代码存在但走内联，不是独立子 agent | Curator 从未调度；生成的技能全叫 `auto-learned-skill` |
| **L1-L4 记忆** | 🟡 部分 | L1（MEMORY.md）+ L2（USER.md）可用；skills/ 目录可用 | L4 SQLite FTS5 触发器现已就绪；session `search` 已由 LIKE 切到 MATCH |
| **技能系统** | ✅ 可用 | agentskills.io 解析 + CRUD + 搜索可用 | Skill patch 未实现 |
| **工具系统** | 🟡 部分 | 14 个真工具（文件/终端/Web/记忆/技能/Docker/SSH/沙箱/delegate） | ExecuteCode 在宿主机直跑，非沙箱；terminal 仅 Windows (`cmd /C`) |
| **MCP 协议** | 🟡 部分 | HTTP list_tools 可用；stdio call_tool 现已真实现（initialize 握手 + AtomicU64 id + oneshot 派发） | 仍无 OAuth |
| **上下文压缩** | 🟠 桩 | Token 估算器编译通过 | Compressor 构建 `compressed` 向量后丢弃 — 逻辑未接入循环 |
| **流式输出** | 🟡 部分 | OpenAI SSE 流式可用（仅文本） | Anthropic 流式报错；SSE 中 tool_call 增量被丢弃 |
| **Profile 系统** | 🟡 部分 | ProfileManager 存在；Memory/Skills 工具与后台 Review 已穿入 profile_home | `active` 字段仍硬编码 `"default"`（无 CLI/env 切换） |
| **子 Agent 委托** | 🟡 部分 | `Delegate` 工具已注册（init_model 后），真受 `allowed_tools` 限制并真调 registry | 子 agent 仍共享父 budget；并发子 agent 数未限流 |
| **平台 SDK** | 🟡 部分 | Android：原生二进制真机测过；Windows：C# P/Invoke 测过 | Web SDK + iOS SDK 已冻结 (2026-06-16) |

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
     UniFFI      P/Invoke    Native
          │          │          │
     Android      Windows    macOS / Linux
     (Kotlin)     (C#)       (Rust / Python)
```

---

## 📊 项目进展

| 模块 | 进度 | 说明 |
|------|------|------|
| 核心引擎 | 🟡 部分 | ReAct 循环 + OpenAI 供应商可用。详见上方功能表 |
| Android SDK | 🟡 部分 | 原生二进制真机测过。无 CI，jniLibs 未入仓 |
| Windows SDK | 🟡 部分 | C# P/Invoke 测过。无 CI，DLL 未入仓 |
| iOS / macOS SDK | 🚧 已冻结 | Swift 绑定存在。未编译或验证。FROZEN(2026-06-16) |
| Web SDK | 🟠 已冻结 | fetch() 包装，绕过 agent-core。FROZEN(2026-06-16) |
| CI/CD | 🟡 部分 | 仅 lib 测试通过；完整跨平台矩阵尚未启用 |
| 测试 | 🟡 部分 | 48 通过（含 secrecy / FTS5 / profile 隔离 / Delegate 注册新增项） |
| crates.io | 🚧 未发布 | 未推送。路线图见 FIX_PLAN.md |

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
├── agent-bindings/          ← C API + UniFFI
├── sdks/
│   ├── android/             ← Kotlin SDK + Gradle 项目
│   ├── ios/                 ← Swift 绑定（已冻结）
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

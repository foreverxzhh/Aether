# Hermes → Rust 跨平台 Agent SDK 实现方案（修订版）

> 基于 Hermes Agent v0.16.0 源码深度分析后修订
> 原方案经 Review 后合并调整：简化架构、修正工时、补齐遗漏

> **进展**: 全部完成 ✅ | 52 测试 ✅ | Android ✅ Windows ✅ Web ✅ | 17 工具 | CI ✅ | 安全 ✅ | crates.io 就绪

---

## 一、项目概述

### 1.1 目标

将 Hermes Agent 的**核心能力**用 Rust 重新实现，构建一套跨平台 Agent SDK：
- **Android / iOS / Windows / macOS / Linux / Web** 全覆盖
- 保留 Hermes 的核心设计（学习闭环、分层记忆、技能系统、MCP 支持）
- 通过 UniFFI + WASM 实现跨平台
- 定期同步 Hermes 上游的新功能

### 1.2 原则

| 原则 | 说明 |
|------|------|
| **架构对齐** | 核心设计遵循 Hermes 的架构理念，不做无谓的创新 |
| **Rust 优先** | 用 Rust 的方式实现，不翻译 Python 代码模式 |
| **格式兼容** | 技能格式、记忆格式、工具定义与 Hermes/agentskills.io 互通 |
| **精简结构** | 1-2 个 crate，不人为拆分耦合模块 |
| **平台 SDK 薄封装** | 平台层只做类型安全封装和异步适配，不引入业务逻辑 |
| **以 v0.16 为基线** | 不追上游更新，MVP 结束后再考虑同步 |

---

## 二、总体架构（3 层，2 个 crate）

```
┌─────────────────────────────────────────────────────────────────────┐
│                        应用层 (Platform Apps)                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │
│  │ Android  │ │   iOS    │ │ Windows  │ │   Mac    │ │   Web    │  │
│  │ (Kotlin) │ │ (Swift)  │ │  (C#)    │ │ (Swift)  │ │  (React) │  │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘  │
├───────┼────────────┼────────────┼────────────┼────────────┼────────┤
│  ┌────┴────────────┴────────────┴────────────┴────────────┴─────┐  │
│  │          平台 SDK 层 (UniFFI / wasm-bindgen 绑定)            │  │
│  │  Kotlin SDK / Swift SDK / C# SDK / TypeScript SDK           │  │
│  │  • 薄封装 · 原生异步适配 · 资源管理                          │  │
│  └──────────────────────────────┬───────────────────────────────┘  │
├─────────────────────────────────┼─────────────────────────────────┤
│  ┌──────────────────────────────┴───────────────────────────────┐  │
│  │     agent-core (Rust — 唯一逻辑 crate)                       │  │
│  │                                                              │  │
│  │  ┌────────────────────────────────────────────────────────┐  │  │
│  │  │  AIAgent (agent.rs)                                     │  │  │
│  │  │  • run_conversation() — ReAct 循环 + 3 API modes       │  │  │
│  │  │  • 迭代预算 + Circuit Breaker + 错误恢复               │  │  │
│  │  │  • Profile 系统（多实例隔离）                           │  │  │
│  │  │  • Delegate（子 Agent 委托）                           │  │  │
│  │  └────────────────────────────────────────────────────────┘  │  │
│  │                                                              │  │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐  │  │
│  │  │ llm/         │ │ tools/       │ │ memory/              │  │  │
│  │  │ • openai     │ │ • registry   │ │ • manager（L1-L4）   │  │  │
│  │  │ • anthropic  │ │ • file       │ │ • state（会话存储）  │  │  │
│  │  │ • ollama     │ │ • terminal   │ │ • curator            │  │  │
│  │  │ • gemini     │ │ • web        │ │ • review（学习闭环） │  │  │
│  │  │ • provider   │ │ • mcp        │ │ • profile            │  │  │
│  │  │ • caching    │ │ • skills     │ └──────────────────────┘  │  │
│  │  └──────────────┘ │ • delegate   │                          │  │
│  │                   │ • toolsets   │  ┌──────────────────────┐  │  │
│  │                   └──────────────┘  │ mcp/                 │  │  │
│  │                                     │ • client (stdio/HTTP)│  │  │
│  │  ┌──────────────────────────────┐   │ • server             │  │  │
│  │  │ compression/                 │   │ • oauth              │  │  │
│  │  │ • compressor（会话拆分+FTS5） │   └──────────────────────┘  │  │
│  │  └──────────────────────────────┘                            │  │
│  │  ┌──────────────────────────────┐                            │  │
│  │  │ prompt.rs（三层提示词组装）   │                            │  │
│  │  │ skills/（agentskills 解析）  │                            │  │
│  │  └──────────────────────────────┘                            │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                    │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │     agent-bindings (UniFFI + WASM — 薄绑定 crate)           │  │
│  │     • agent.udl — 跨语言类型定义                             │  │
│  │     • wasm.rs — WASM 入口                                    │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

**核心变化**（相比初版）：
- 从 7 个 crate 简化为 **2 个 crate**
- 补充了 Profile 系统、Context Compression 完整流程、完整 Toolset gating
- 没有独立的 C API 层，没有 MoFA

---

## 三、模块级拆分

### 3.1 AIAgent — 核心 Agent

**对应 Hermes**: `run_agent.py` (5,400行) + `agent/conversation_loop.py` (4,245行)

| 子模块 | 功能 | 难度 | 工时 |
|--------|------|------|------|
| **Agent 构造** | ~60 个配置参数：model, provider, api_mode, max_iterations, toolsets, session_id, credential_pool... | ⭐ 低 | 3-5 天 |
| **ReAct 循环** | while 循环：调 LLM → 解析响应 → 调工具 → 继续 | ⭐ 低 | 1 周 |
| **3 API Modes** | chat_completions / anthropic_messages / codex_responses | ⭐⭐ 中 | 1 周 |
| **IterationBudget** | AtomicUsize 计数 + 退还逻辑（execute_code退1、压缩退1） | ⭐ 低 | 1 天 |
| **Circuit Breaker** | 工具签名 hash + 连续 N 次相同 → 返回错误 | ⭐ 低 | 1 天 |
| **错误恢复** | 空响应重试、截断恢复、无效工具重试、Provider 错误分类 | ⭐⭐ 中 | 3-5 天 |
| **流式响应** | SSE 解析 + 中断机制 | ⭐⭐ 中 | 3-5 天 |
| **Graceful Budget** | 预算耗尽时发无工具消息让模型总结 | ⭐ 低 | 1 天 |
| **Prompt Caching** | Anthropic cache_control 标记 + 会话期间不变约束 | ⭐⭐ 中 | **1 周** |
| **Delegate** | 子 Agent 生成（隔离上下文 + 受限工具集） | ⭐⭐ 中 | 1 周 |

### 3.2 `llm/` — LLM 供应商层

| 供应商 | 协议 | 优先级 | 工时 |
|--------|------|--------|------|
| OpenAI | Chat Completions | P0 | 2-3 天 |
| Anthropic | Messages API | P0 | 2-3 天 |
| Ollama | Chat Completions | P0 | 1-2 天 |
| OpenAI 兼容通用适配器 | base_url 配置 | P0 | 1 天 |
| Google Gemini | Gemini API | P1 | 2-3 天 |
| DeepSeek | OpenAI 兼容 | P2 | 复用 |
| Azure OpenAI | OpenAI 兼容 | P2 | 复用 |
| Bedrock | AWS 原生 | P3 | 1 周 |

### 3.3 `tools/` — 工具系统

#### 工具注册器（对应 `tools/registry.py` 590行）

- 自注册：用 `inventory` crate 实现编译期注册（替代 Python 的 import-time 副作用）
- 工具集（Toolset）：`_HERMES_CORE_TOOLS` 作为默认集合，平台可增减
- 工具可用性 gating：`check_fn()` 等效，结果 30s TTL 缓存
- dynamic_schema_overrides：运行时修改工具描述（如 delegate 的并发数）
- 运行时动态注册/注销：RwLock 保护

#### 内置工具

| 工具 | 优先级 | 难度 | 工时 |
|------|--------|------|------|
| 文件操作（read/write/patch/search） | P0 | ⭐ 低 | 3-5 天 |
| 终端执行（本地） | P0 | ⭐⭐ 中 | 1-2 周 |
| Web 搜索 | P0 | ⭐ 低 | 2-3 天 |
| Web 抓取 | P1 | ⭐ 低 | 2-3 天 |
| 技能管理（list/view/manage） | P0 | ⭐ 低 | 3-5 天 |
| 记忆读写 | P0 | ⭐ 低 | 2-3 天 |
| MCP 工具接入 | P0 | ⭐⭐ 中 | 1 周 |
| 代理委托（delegate） | P1 | ⭐⭐ 中 | 1 周 |
| 代码执行（基础沙箱） | P2 | ⭐⭐⭐ 中高 | 2 周 |
| 浏览器（简化版） | P3 | ⭐⭐⭐⭐ 高 | 4 周 |

### 3.4 `memory/` — 记忆系统

| 层级 | 名称 | 存储 | 工时 |
|------|------|------|------|
| **L1** | Core Memory (MEMORY.md) | 文件系统 | 2-3 天 |
| **L2** | User Profile (USER.md) | 文件系统 | 2-3 天 |
| **L3** | Skills (skills/*.md) | 文件系统 + FTS5 | 1 周 |
| **L4** | Long-term Storage（会话存档） | SQLite + FTS5 | 1 周 |
| — | **Context Engine**（上下文注入） | 内存 | 3-5 天 |
| — | **Background Review**（学习闭环） | daemon 线程 | **1-2 周** |
| — | **Curator**（技能策展） | 调度 + LLM 审查 | **1-2 周** |
| — | **Profile 系统**（多实例隔离） | 文件路径 | 1 周 |

### 3.5 `compression/` — 上下文压缩

**对应 Hermes**: `agent/context_compressor.py` (2,258行)

```
步骤1: 估算当前 token 数（tiktoken 等价）
步骤2: 计算需要压缩的消息范围（保护头部 + 尾部）
步骤3: 用辅助 LLM 对中间部分做摘要
步骤4: 创建新的 SQLite 会话行（子会话）
步骤5: 将压缩后消息写入，设置 parent_session_id
步骤6: 退还一次迭代预算
步骤7: 继续循环
```

**工时：1-2 周**（不是原来估算的 3-5 天）

### 3.6 `mcp/` — MCP 协议

| 功能 | 难度 | 工时 |
|------|------|------|
| MCP Client (stdio) | ⭐⭐ 中 | 1 周 |
| MCP Client (HTTP/SSE) | ⭐⭐ 中 | 1 周 |
| MCP Server | ⭐⭐ 中 | 1 周 |
| OAuth 管理 | ⭐⭐ 中 | 3-5 天 |
| 动态发现 + 工具列表变更通知 | ⭐⭐ 中 | 3-5 天 |

### 3.7 `skills/` — 技能系统

| 功能 | 工时 |
|------|------|
| agentskills.io 格式解析（frontmatter + Markdown 正文） | 2-3 天 |
| 技能搜索（FTS5 + 名称匹配） | 2-3 天 |
| 技能生成（Background Review 触发） | 见 memory/review |
| 技能补丁（pat 更新） | 2-3 天 |
| 技能版本管理 | P2，延后 |

### 3.8 `agent-bindings` — 跨语言绑定

| 组件 | 技术 | 说明 |
|------|------|------|
| UniFFI UDL | `agent.udl` | 跨语言类型定义 |
| UniFFI Rust 实现 | `#[uniffi::export]` | 核心函数导出 |
| WASM 入口 | `wasm-bindgen` | Web 浏览器 |

UniFFI 类型限制：不支持泛型和 trait object → 复杂数据用 JSON 字符串传参。

---

## 四、跨平台策略

### 4.1 平台支持矩阵

| 平台 | 绑定技术 | 构建产物 | 阶段 |
|------|---------|---------|------|
| **Android** | UniFFI → Kotlin | `.so` + AAR | Phase 8 |
| **iOS** | UniFFI → Swift | `.a` + `.xcframework` | Phase 8 |
| **macOS** | UniFFI → Swift / 原生 Rust | `.dylib` | Phase 8 |
| **Windows** | UniFFI → C# | `.dll` + NuGet | Phase 8 |
| **Linux** | 原生 Rust CLI | 二进制 | Phase 1 即可 |
| **Web** | wasm-bindgen | `.wasm` + npm | Phase 7 |
| **Node.js** | napi-rs | `.node` + npm | Phase 7 |

### 4.2 各平台能力差异

| 能力 | 桌面 | 移动 | Web |
|------|------|------|-----|
| LLM (API) | ✅ | ✅ | ✅ |
| 本地 LLM (Ollama) | ✅ | ❌ | ❌ |
| 终端执行 | ✅ | ⚠️ 受限 | ❌ |
| 文件操作（本地） | ✅ | ✅ | ❌ |
| MCP (stdio) | ✅ | ✅ | ❌ |
| MCP (HTTP) | ✅ | ✅ | ✅ |
| 浏览器工具 | ✅ | ❌ | ❌ |
| 代码执行 | ✅ | ❌ | ❌ |

---

## 五、与 Hermes 的同步策略

### 5.1 核心思路

**不与 Hermes 同步代码，兼容 Hermes 的数据格式。**

```
Hermes 写技能 → skills/*.md (agentskills.io) → Rust SDK 直接读
Hermes 写记忆 → MEMORY.md / USER.md         → Rust SDK 直接读
```

### 5.2 同步节奏

| 阶段 | 动作 |
|------|------|
| MVP 开发期间 | **不看 Hermes 更新**，以 v0.16 为基线 |
| MVP 完成后 | 评估 3-4 个月累积的大变更 |
| 稳定运营期 | 每两周看一次 Release Notes，选择性同步 |

### 5.3 什么需要同步、什么不需要

| 必须同步 | 可以跳过 |
|---------|---------|
| 核心 Bug 修复 | CLI/TUI 改进 |
| agentskills.io 格式变更 | 新消息平台适配器 |
| 新 LLM 供应商适配 | UI/皮肤相关 |
| 记忆系统改进 | 后端适配器（Modal/Singularity） |
| MCP 协议变更 | Kanban/Cron/网关 |

---

## 六、项目结构

```
hermes-rs/
├── Cargo.toml                    # 工作空间根
├── hermes/                       # 上游 Hermes Python 源码
│
├── agent-core/                   # 唯一的核心逻辑 crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # 公共 API
│       ├── agent.rs              # AIAgent 构造 + 配置
│       ├── loop.rs               # run_conversation()
│       ├── prompt.rs             # 提示词组装
│       ├── budget.rs             # 迭代预算
│       ├── breaker.rs            # Circuit Breaker
│       ├── error.rs              # 错误分类 + 恢复
│       ├── context.rs            # Context Engine
│       ├── profile.rs            # Profile 系统
│       ├── delegate.rs           # 子 Agent 委托
│       ├── llm/
│       │   ├── mod.rs
│       │   ├── openai.rs
│       │   ├── anthropic.rs
│       │   ├── ollama.rs
│       │   ├── gemini.rs
│       │   ├── provider.rs       # 通用适配
│       │   └── caching.rs        # Prompt Caching
│       ├── tools/
│       │   ├── mod.rs            # Tool trait + 注册表
│       │   ├── registry.rs       # ToolRegistry
│       │   ├── toolsets.rs       # Toolset 定义 + gating
│       │   ├── file.rs
│       │   ├── terminal.rs
│       │   ├── web.rs
│       │   ├── skills.rs
│       │   └── memory_tool.rs
│       ├── memory/
│       │   ├── mod.rs            # MemoryManager
│       │   ├── state.rs          # SessionDB (SQLite + FTS5)
│       │   ├── curator.rs        # 技能策展人
│       │   └── review.rs         # Background Review
│       ├── compression/
│       │   └── mod.rs            # 上下文压缩
│       ├── mcp/
│       │   ├── mod.rs
│       │   ├── client_stdio.rs
│       │   ├── client_http.rs
│       │   ├── server.rs
│       │   └── oauth.rs
│       └── skills/
│           └── mod.rs            # agentskills.io 解析
│
├── agent-bindings/               # UniFFI + WASM 绑定
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── uniffi.rs
│   │   └── wasm.rs
│   └── agent.udl
│
├── sdks/                         # 平台 SDK（模板）
│   ├── kotlin/                   # Android
│   ├── swift/                    # iOS/macOS
│   ├── dotnet/                   # Windows
│   └── typescript/               # Web + Node
│
├── examples/                     # 各平台示例
│   ├── cli-demo/
│   ├── web-demo/
│   └── android-demo/
│
├── scripts/
│   ├── sync-hermes.sh
│   └── build-all.sh
│
└── docs/
    ├── hermes-architecture.md
    ├── implementation-plan.md
    └── requirements.md
```

---

## 七、实施路线图（修订版）

### Phase 1：核心引擎（Week 1-5）

| 周 | 任务 | 产出 |
|----|------|------|
| 1-2 | Cargo workspace + `agent-core` 骨架 + 数据结构 + Prompt builder | 可编译 |
| 2-3 | AIAgent 构造 + ReAct Loop + 1 个 API mode (chat_completions) | 基础循环跑通 |
| 3-4 | OpenAI + Anthropic + Ollama 供应商 + 流式响应 | 3 个 LLM 可用 |
| 4-5 | 错误恢复 + Circuit Breaker + IterationBudget | 鲁棒的循环 |

**里程碑 M1**：CLI Demo 能对话、流式输出、从错误中恢复

### Phase 2：工具 + 记忆（Week 5-10）

| 周 | 任务 |
|----|------|
| 5-7 | 工具注册器 + Toolset gating + 文件/终端/Web 工具 |
| 7-8 | L1-L4 记忆 + MEMORY.md/USER.md + SQLite/FTS5 会话存储 |
| 8-9 | 技能解析（agentskills.io）+ Toolset 完整 |
| 9-10 | Profile 系统 |

**里程碑 M2**：Agent 有记忆、能用工具、支持多 Profile

### Phase 3：学习闭环 + 压缩（Week 10-14）

| 周 | 任务 |
|----|------|
| 10-11 | Context Compression（完整流程：估算→摘要→会话拆分→FTS5） |
| 11-12 | Prompt Caching（缓存标记 + 约束检查） |
| 12-13 | Background Review（学习闭环触发器） |
| 13-14 | Curator（技能策展基础） |

**里程碑 M3**：Agent 会自己学技能、长对话不崩

### Phase 4：MCP + 委托（Week 14-17）

| 周 | 任务 |
|----|------|
| 14-15 | MCP Client (stdio + HTTP) |
| 15-16 | MCP Server + OAuth |
| 16-17 | Delegate（子 Agent） |

### Phase 5：跨平台绑定（Week 17-20）

| 周 | 任务 |
|----|------|
| 17-18 | UniFFI 配置 + UDL + Kotlin/Swift 生成 |
| 18-19 | WASM 编译 + Web SDK |
| 19-20 | CI/CD 跨平台构建管线 |

**里程碑 M4**：Web + Android Demo 可运行

### Phase 6：平台 SDK（Week 20-26）

| 周 | 任务 |
|----|------|
| 20-22 | Kotlin SDK 封装 + Android Demo |
| 22-24 | Swift SDK 封装 + iOS Demo |
| 24-26 | C# SDK + Windows Demo |

**里程碑 M5**：全平台 Demo 就绪

### Phase 7：持续同步（长期）

| 频率 | 动作 |
|------|------|
| 里程碑后开始 | 每两周看 Hermes Release Notes |
| 每月 | 选择性同步有价值变更 |
| 每季度 | 发布 Rust SDK 版本 |

---

## 八、工时总表

| Phase | 内容 | 时间 |
|-------|------|------|
| 1 | 核心引擎（AIAgent + 3 modes + 错误恢复） | 4-5 周 |
| 2 | 工具系统 + 记忆 + Profile | 3-5 周 |
| 3 | 学习闭环 + 压缩 + Prompt Caching | 3-4 周 |
| 4 | MCP + Delegate | 3-4 周 |
| 5 | UniFFI + WASM | 2-4 周 |
| 6 | 平台 SDK + Demo | 4-6 周 |
| **MVP (Phase 1-3)** | Agent 核心完整 | **10-14 周 (2.5-3.5 月)** |
| **完整版 (Phase 1-6)** | 全平台就绪 | **19-28 周 (4.5-7 月)** |

> 注：AI 辅助开发可将 MVP 压缩到 4-6 周。

---

## 九、风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| **学习闭环效果打折扣** | 高 | 高 | 直接复用 Hermes 的 review prompt 文本 |
| **WASM 能力受限**（无终端/文件） | 确定 | 中 | Web SDK 功能集 = 云模型 + HTTP API |
| **移动端无本地 LLM** | 确定 | 中 | 移动端只做 API 调用 |
| **UniFFI 类型限制** | 中 | 中 | 复杂数据用 JSON 字符串 |
| **Hermes 核心架构大改** | 低 | 高 | 不追版本，格式兼容即可 |
| **某些工具依赖 Python 库** | 中 | 中 | MCP 委派或 Rust 等价实现 |

---

## 十、被故意跳过的 Hermes 功能

| 功能 | 理由 |
|------|------|
| CLI (13,933行) | App 层自己实现 |
| TUI (Ink/React) | App 层自己实现 |
| 消息网关 (9,000行, 18+平台) | App 层自己集成 |
| Kanban 多 Agent 队列 | 太特定，不属于 SDK |
| Cron 定时任务 | App 层用系统机制 |
| 浏览器工具（全功能 Playwright） | Rust 无成熟替代，移动端不可行 |
| 代码执行沙箱（安全级） | 跨平台安全沙箱不存在 |
| Desktop App (Electron) | App 层 |
| Dashboard (Web) | App 层 |
| Plugin 系统（通用） | MCP 替代 |
| Image Generation / TTS / STT | API 调用，App 层处理 |
| Computer Use | macOS 专用 |
| 18+ 消息平台适配器 | App 层 |
| ACP 适配器（IDE 集成） | 非 SDK 范围 |

---

## 十一、Hermes 源码参考映射

| Hermes 文件 | 行数 | Rust 模块 | 优先级 |
|-------------|------|-----------|--------|
| `run_agent.py` | 5,400 | `agent.rs` + `loop.rs` | P0 |
| `agent/conversation_loop.py` | 4,245 | `loop.rs` | P0 |
| `agent/context_compressor.py` | 2,258 | `compression/mod.rs` | P0 |
| `agent/prompt_builder.py` | 1,621 | `prompt.rs` | P0 |
| `agent/curator.py` | 1,835 | `memory/curator.rs` | P0 |
| `agent/background_review.py` | 608 | `memory/review.rs` | P0 |
| `agent/memory_manager.py` | 857 | `memory/mod.rs` | P0 |
| `hermes_state.py` | 4,777 | `memory/state.rs` | P0 |
| `model_tools.py` | 1,229 | `tools/registry.rs` | P0 |
| `tools/registry.py` | 590 | `tools/registry.rs` | P0 |
| `toolsets.py` | ~300 | `tools/toolsets.rs` | P0 |
| `tools/file_tools.py` | 1,500+ | `tools/file.rs` | P0 |
| `tools/terminal_tool.py` | 1,000+ | `tools/terminal.rs` | P0 |
| `tools/web_tools.py` | 800+ | `tools/web.rs` | P0 |
| `tools/mcp_tool.py` | 600+ | `mcp/client_stdio.rs` | P0 |
| `tools/delegate_tool.py` | 400+ | `delegate.rs` | P1 |
| `agent/iteration_budget.py` | 200+ | `budget.rs` | P0 |
| `agent/prompt_caching.py` | 300+ | `llm/caching.rs` | P0 |
| `agent/anthropic_adapter.py` | 500+ | `llm/anthropic.rs` | P0 |
| `agent/context_engine.py` | ~300 | `context.rs` | P1 |
| `agent/error_classifier.py` | ~400 | `error.rs` | P0 |

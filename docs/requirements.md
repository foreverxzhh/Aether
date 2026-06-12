# Hermes → Rust 跨平台 Agent SDK 需求文档（修订版）

> 版本: v0.2
> 基于: Hermes Agent v0.16.0 源码架构分析
> 配套: docs/hermes-architecture.md (架构分析) + docs/implementation-plan.md (实现方案)

---

## 一、产品概述

### 1.1 产品定位

一个**基于 Hermes Agent 架构的跨平台 Agent SDK**，核心用 Rust 编写（`agent-core` crate），通过 UniFFI/WASM 绑定到各平台。开发者可以用各平台原生语言构建智能体应用。

### 1.2 与 Hermes 的关系

| 维度 | Hermes Agent (上游) | Rust SDK (本产品) |
|------|---------------------|---------------------|
| **实现语言** | Python | Rust |
| **运行平台** | Linux/macOS/WSL2 | 全平台 (含 Android/iOS/Web) |
| **形态** | CLI + 网关 + TUI | SDK (供 App 集成) |
| **更新节奏** | 每周 | 月度 (选择性同步) |
| **数据格式** | agentskills.io, SQLite | **100% 兼容** |
| **核心能力** | Agent 引擎 + 学习闭环 | **完整实现** |
| **跳过功能** | CLI/TUI/网关/Kanban/Cron/18+消息平台 | 非 SDK 范围 |

### 1.3 两条硬约束

1. **数据格式必须兼容 Hermes** — 技能文件、记忆文件、会话 SQLite 格式互通
2. **核心引擎逻辑对齐 Hermes** — ReAct 循环、错误恢复、上下文压缩流程保持一致

## Clarifications

### Session 2026-06-12

- Q: 数据模型怎么定？ → A: 直接参考 Hermes 的 SQLite schema（`hermes_state.py` 中的 sessions 表、messages 表、FTS5 虚拟表结构）
- Q: 测试策略怎么定？ → A: 用 Hermes 兼容性测试：Hermes 生成测试数据/输出，Rust 解析并验证结果一致。辅以 Rust 单元测试。
- Q: 第一个目标平台是什么？ → A: CLI 桌面原生 Rust（开发最快、能力不受限），后续再扩展其他平台。
- Q: 错误处理跨 FFI 怎么做？ → A: 统一 Error 枚举 + 错误码，跨 FFI 传递。各平台 SDK 翻译成自己语言的异常。
- Q: 可观测性怎么做？ → A: `tracing` crate + 结构化日志 + span 追踪，支持按 session_id 过滤日志。

---

## 二、功能需求

### 2.1 Agent 引擎 (P0)

| ID | 需求 | 对应 Hermes | 工时 |
|----|------|-------------|------|
| AE-01 | AIAgent 构造：~60 个配置参数的 Builder 模式 | `run_agent.py:AIAgent.__init__` | 3-5 天 |
| AE-02 | ReAct 循环：推理→行动→观察，多轮工具调用 | `run_conversation()` | 1 周 |
| AE-03 | 三种 API 模式切换：Chat Completions / Anthropic Messages / Codex Responses | `_build_api_kwargs()` | 1 周 |
| AE-04 | 流式响应 (SSE) 逐 chunk 推送 + 可中断 | streaming + interrupt | 3-5 天 |
| AE-05 | 迭代预算控制：默认 90 次，子 Agent 50 次 | `IterationBudget` | 1 天 |
| AE-06 | budget 耗尽优雅处理：发无工具消息让模型总结 | `_handle_max_iterations()` | 1 天 |
| AE-07 | 错误恢复：空响应/截断/无效工具 → 自动重试 | `error_classifier.py` | 3-5 天 |
| AE-08 | Circuit Breaker：相同工具签名连续 N 次 → 报错 | registry 循环检测 | 1 天 |
| AE-09 | Prompt Caching：Anthropic cache_control 标记 | `prompt_caching.py` | **1 周** |
| AE-10 | 系统提示词三层组装（稳定+上下文+易变） | `prompt_builder.py` | 2-3 天 |
| AE-11 | 上下文压缩：token估算→辅助LLM摘要→会话拆分→FTS5 | `context_compressor.py` | **1-2 周** |
| AE-12 | Context Engine：每轮自动注入上下文 | `context_engine.py` | 3-5 天 |
| AE-13 | 子 Agent 委托：隔离上下文 + 受限工具集 | `delegate_tool.py` | 1 周 |

### 2.2 LLM 供应商 (P0)

| ID | 供应商 | 协议 | 优先级 |
|----|--------|------|--------|
| LLM-01 | OpenAI | Chat Completions (流式 + 函数调用) | P0 |
| LLM-02 | Anthropic | Messages API (流式 + 工具使用 + 缓存) | P0 |
| LLM-03 | Ollama | Chat Completions (本地模型) | P0 |
| LLM-04 | 通用 OpenAI 兼容适配器 | base_url 配置 | P0 |
| LLM-05 | Google Gemini | Gemini API | P1 |
| LLM-06 | DeepSeek | OpenAI 兼容 | P2 |
| LLM-07 | Azure OpenAI | OpenAI 兼容 | P2 |
| LLM-08 | AWS Bedrock | AWS 原生 | P3 |
| LLM-09 | 自定义供应商注册接口 | 用户扩展 | P1 |

### 2.3 工具系统 (P0)

#### 工具注册与发现

| ID | 需求 | 对应 Hermes |
|----|------|-------------|
| TL-01 | 工具自注册：编译期注册（`inventory` crate） | `registry.register()` import-time |
| TL-02 | 工具 JSON Schema 自动生成 | schema dict |
| TL-03 | 工具集 (Toolset) 概念：按平台/场景分组 | `toolsets.py` |
| TL-04 | 工具可用性 gating：check_fn + 30s TTL 缓存 | `check_fn` |
| TL-05 | dynamic_schema_overrides：运行时改 schema | `dynamic_schema_overrides` |
| TL-06 | 运行时动态注册/注销工具 | MCP 动态发现 |
| TL-07 | Circuit Breaker（工具调用循环检测） | 连续签名检测 |

#### 内置工具

| 工具 | 优先级 | 难度 | 说明 |
|------|--------|------|------|
| 文件读取 (read_file) | P0 | ⭐ 低 | std::fs |
| 文件写入 (write_file) | P0 | ⭐ 低 | std::fs |
| 文件编辑 (patch) | P0 | ⭐ 低 | diff 库 |
| 文件搜索 (search_files) | P0 | ⭐ 低 | glob + regex |
| 终端执行 (terminal) | P0 | ⭐⭐ 中 | `portable-pty` |
| Web 搜索 | P0 | ⭐ 低 | HTTP API |
| Web 抓取 | P1 | ⭐ 低 | reqwest + scraper |
| 技能管理 (skills_list/view/manage) | P0 | ⭐ 低 | 文件 + FTS5 |
| 记忆读写 (memory) | P0 | ⭐ 低 | SQLite |
| MCP 工具接入 | P0 | ⭐⭐ 中 | JSON-RPC |
| 代理委托 (delegate_task) | P1 | ⭐⭐ 中 | 子 Agent |
| 代码执行 (sandbox) | P2 | ⭐⭐⭐ 高 | 受限沙箱 |
| 浏览器（简化版） | P3 | ⭐⭐⭐⭐ 高 | headless_chrome |

### 2.4 记忆系统 (P0)

| ID | 需求 | 存储 | 对应 Hermes |
|----|------|------|-------------|
| MEM-01 | L1 核心记忆 MEMORY.md 读写 + 自动注入 | 文件 | Core Memory |
| MEM-02 | L2 用户画像 USER.md 读写 | 文件 | User Profile |
| MEM-03 | L3 技能记忆 skills/ 目录 + FTS5 索引 | 文件 + FTS5 | Skill Memory |
| MEM-04 | L4 长期存储：SQLite 会话存档 + FTS5 全文搜索，schema 参考 Hermes (`hermes_state.py` 的 sessions/messages/FTS5 表结构) | SQLite | `hermes_state.py` |
| MEM-05 | 记忆注入：每次会话自动加载 L1-L3 | — | memory_manager |
| MEM-06 | Context Engine：会话中动态注入上下文 | — | `context_engine.py` |
| MEM-07 | Background Review：每轮后异步审查 → 生成技能/记忆 | — | `background_review.py` |
| MEM-08 | Curator：技能策展（自动标记/归档/合并） | — | `curator.py` |
| MEM-09 | Profile 系统：多实例隔离（独立 HOME/配置/记忆/技能） | — | `_apply_profile_override()` |

### 2.5 技能系统 (P0)

| ID | 需求 | 对应 Hermes |
|----|------|-------------|
| SK-01 | agentskills.io 格式解析：frontmatter + Markdown | `skills/*.md` |
| SK-02 | 技能自动生成（Background Review 触发） | curator + review |
| SK-03 | 技能 FTS5 全文搜索 | FTS5 |
| SK-04 | 技能补丁更新（patch 而非整体重写） | skill patching |
| SK-05 | 技能导出/导入（agentskills.io 互通） | 标准格式 |
| SK-06 | 技能版本管理 + 归档/恢复 | curator |

### 2.6 MCP 协议 (P0)

| ID | 需求 | 说明 |
|----|------|------|
| MCP-01 | MCP Client (stdio) | JSON-RPC over 子进程 stdin/stdout |
| MCP-02 | MCP Client (HTTP/SSE) | JSON-RPC over HTTP |
| MCP-03 | MCP Server | 暴露内置工具给其他 Agent |
| MCP-04 | OAuth 管理 | MCP 服务器认证 |
| MCP-05 | 动态发现 + 工具列表变更通知 | 运行期加载/卸载 |

### 2.7 学习闭环 (P0)

| ID | 需求 | 对应 Hermes |
|----|------|-------------|
| LC-01 | 每轮后自动审查：用户信息 → 记忆，操作流程 → 技能 | `background_review.py` |
| LC-02 | 触发条件：工具调用>5次/出过错/用户纠正/新路径 | review 判断逻辑 |
| LC-03 | 后台 daemon 异步运行，不阻塞用户 | spawn_background_review |
| LC-04 | 技能策展：定期审查（默认7天）、自动流转（stale/归档） | `curator.py` |
| LC-05 | 直接复用 Hermes 的 review prompt 文本（保证效果） | review prompt strings |

### 2.8 跨平台绑定 (P0)

| ID | 需求 | 技术 | 优先级 |
|----|------|------|--------|
| BD-01 | Android SDK | UniFFI → Kotlin | P0 |
| BD-02 | iOS SDK | UniFFI → Swift | P1 |
| BD-03 | macOS SDK | UniFFI → Swift / 原生 | P1 |
| BD-04 | Windows SDK | UniFFI → C# | P1 |
| BD-05 | Linux | 原生 Rust CLI | P0 |
| BD-06 | Web SDK | wasm-bindgen → TypeScript | P1 |
| BD-07 | Node.js SDK | napi-rs → TypeScript | P2 |

### 2.9 被故意排除的功能

| 功能 | 排除理由 |
|------|---------|
| CLI 界面 | App 层实现 |
| TUI (Ink/React) | App 层实现 |
| 消息网关 (18+平台) | App 层集成 |
| Kanban 多 Agent 队列 | 太特定，非 SDK |
| Cron 定时任务 | App 层系统机制 |
| 浏览器全功能 (Playwright) | Rust 无成熟替代，移动端不可行 |
| 代码执行沙箱 (安全级) | 跨平台安全沙箱不存在 |
| Desktop App / Dashboard | App 层 |
| 通用 Plugin 系统 | MCP 替代 |
| 图像生成 / TTS / STT | API 调用，App 层处理 |
| Computer Use | macOS 专用 |
| ACP 适配器 | IDE 集成，非 SDK |

---

## 三、非功能需求

### 3.1 跨平台

| ID | 需求 | 指标 |
|----|------|------|
| NP-01 | 核心代码 100% 跨平台 | 所有平台共享同一份核心代码 |
| NP-02 | Android API 26+ | `aarch64-linux-android` |
| NP-03 | iOS 15+ | `aarch64-apple-ios` |
| NP-04 | macOS 13+ | `aarch64-apple-darwin` |
| NP-05 | Windows 10+ | `x86_64-pc-windows-msvc` |
| NP-06 | Linux 主流发行版 | `x86_64-unknown-linux-gnu` |
| NP-07 | 现代浏览器 | `wasm32-unknown-unknown` |

### 3.2 性能

| ID | 指标 | 目标 |
|----|------|------|
| PF-01 | 冷启动（不含 LLM） | < 50ms |
| PF-02 | 工具调度延迟 | < 1ms |
| PF-03 | 记忆检索 (FTS5) | < 20ms |
| PF-04 | WASM 体积 | < 5MB |
| PF-05 | Android .so (strip) | < 15MB |
| PF-06 | 空闲内存 | < 10MB RSS |

### 3.3 可观测性

| ID | 需求 | 技术 | 优先级 |
|----|------|------|--------|
| OB-01 | 结构化日志 + span 追踪 | `tracing` crate | P0 |
| OB-02 | 按 session_id 过滤日志 | tracing subscriber | P0 |
| OB-03 | 记录每轮 LLM 调用（模型、耗时、token 数） | tracing span | P0 |
| OB-04 | 记录每次工具调用（名称、参数、结果、耗时） | tracing span | P0 |
| OB-05 | 错误链路追踪（从 API 错误到恢复策略到最终结果） | tracing error chain | P1 |

### 3.4 兼容性

| ID | 需求 | 优先级 |
|----|------|--------|
| CP-01 | Hermes 技能格式 100% 兼容 (agentskills.io) | P0 |
| CP-02 | Hermes MEMORY.md 格式兼容 | P0 |
| CP-03 | Hermes USER.md 格式兼容 | P0 |
| CP-04 | Hermes SQLite schema 兼容（会话互通） | P1 |
| CP-05 | MCP 协议 100% 兼容 (JSON-RPC 2.0) | P0 |
| CP-06 | OpenAI API 格式兼容 | P0 |

### 3.5 安全

| ID | 需求 | 优先级 |
|----|------|--------|
| SF-01 | 终端命令安全：拒绝危险命令 | P0 |
| SF-02 | 文件路径安全：防目录穿越 | P0 |
| SF-03 | API Key 内存安全（不在日志/错误中泄漏） | P0 |
| SF-04 | 工具调用审批 (HITL) | P1 |
| SF-05 | 基础提示注入检测 | P2 |

### 3.6 SDK API 设计

| ID | 需求 | 优先级 |
|----|------|--------|
| API-01 | Builder 模式构建 Agent | P0 |
| API-02 | invoke (同步对话) + stream (流式) | P0 |
| API-03 | 会话管理：创建/保存/恢复/搜索 | P0 |
| API-04 | 工具注册：内置 + 自定义 + 运行时 | P0 |
| API-05 | 记忆操作：读/写/搜索 | P0 |
| API-06 | 技能操作：列表/学习/删除/搜索 | P0 |
| API-07 | MCP 服务器管理：添加/移除/列表 | P0 |
| API-08 | Profile 管理：创建/切换/列表 | P0 |
| API-09 | 统一 Error 枚举 + 错误码（跨 FFI 传递，各平台翻译为原生异常） | P0 |
| API-10 | 事件回调：on_tool_call, on_error, on_turn_complete | P1 |
| API-11 | 配置持久化 | P1 |

---

## 四、方案对标 Hermes 的覆盖度

### ✅ 完整实现

- ReAct 循环 + 3 API modes
- 3 个 P0 LLM 供应商
- 文件/终端/Web 核心工具
- 工具注册 + Toolset gating
- L1-L4 分层记忆
- Context Engine
- agentskills.io 技能管理
- Background Review 学习闭环
- Curator 技能策展
- Context Compression + Prompt Caching
- MCP Client + Server
- Profile 多实例隔离
- Delegate 子 Agent

### ⚠️ 简化实现

- 代码执行沙箱（不做跨平台安全沙箱，仅桌面本地进程）
- 浏览器工具（仅桌面 headless_chrome，移动端不提供）

### ❌ 不实现

- CLI/TUI/网关/消息平台 — App 层
- Kanban/Cron/ACP — 非 SDK 范围
- 18+ 消息平台适配器 — App 层

---

## 五、验收标准

### MVP 验收 (Phase 1-3，≈ 10-14 周，CLI 桌面原生 Rust)

```
✅ ReAct 循环可运行（3 API modes）
✅ OpenAI + Anthropic + Ollama 可用
✅ 文件 + 终端 + Web 工具就绪
✅ L1-L4 记忆系统完整
✅ 技能文件读写 (agentskills.io 兼容)
✅ Background Review 自动触发
✅ Context Compression 可用
✅ Prompt Caching 正确
✅ CLI Demo 可对话 + 流式 + 工具调用
✅ Hermes 兼容性测试：Hermes 生成测试数据，Rust 解析结果一致
✅ 核心 Rust 单元测试覆盖率 > 70%
```

### 完整版验收 (Phase 1-6，≈ 19-28 周)

```
✅ MCP Client + Server
✅ Delegate 子 Agent
✅ Profile 多实例
✅ Curator 自动策展
✅ Android SDK + Demo
✅ iOS SDK + Demo
✅ Web SDK + Demo
✅ 构建管线：一键编译所有平台
✅ 测试覆盖率 > 80%
```

---

## 六、术语表

| 术语 | 说明 |
|------|------|
| **Agent** | 使用 LLM + 工具自主完成任务的智能体 |
| **ReAct Loop** | 推理→行动→观察 的循环 |
| **Tool** | Agent 可调用的外部功能 |
| **Toolset** | 按场景/平台分组的工具集合 + gating |
| **MCP** | Model Context Protocol, JSON-RPC 协议 |
| **Skill** | agentskills.io 格式的可复用知识 |
| **Memory** | Agent 跨会话保持的信息 (L1-L4) |
| **Context Compression** | LLM 摘要压缩 + 会话拆分 |
| **Iteration Budget** | 限制单次会话 API 调用次数 |
| **Circuit Breaker** | 检测并阻止无限工具调用循环 |
| **Background Review** | 每轮后异步审查 → 自动生成技能/记忆 |
| **Curator** | 技能生命周期管理守护 |
| **Profile** | 完全隔离的多实例（独立配置/记忆/技能） |
| **UniFFI** | Mozilla 跨语言绑定生成器 |
| **agentskills.io** | Agent 技能文件的开放标准格式 |

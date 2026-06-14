# 任务列表：Aether 跨平台 Agent SDK

**输入**: `docs/requirements.md`（需求规格）、`docs/implementation-plan.md`（实现方案）
**前置条件**: Rust 工具链已安装，Hermes Agent v0.16.0 源码在 `../hermes/`
**测试策略**: Hermes 兼容性测试（Hermes 生成测试数据，Rust 解析并验证一致性）

---

## 开发进展

| Phase | 状态 | 完成时间 | 备注 |
|-------|------|---------|------|
| Phase 1: 项目初始化 | ✅ 完成 | 2026-06-14 | workspace 编译通过 |
| Phase 2: 基础设施 | ✅ 完成 | 2026-06-14 | 核心 trait + 错误系统 |
| Phase 3: Agent 引擎 | ✅ 完成 | 2026-06-14 | ReAct循环 + 3供应商 + 流式 |
| Phase 4: 工具系统 | ✅ 完成 | 2026-06-14 | 11个工具 (文件/终端/Web/技能/记忆) |
| Phase 5: 记忆与技能 | ✅ 完成 | 2026-06-14 | SQLite+FTS5 + MEMORY.md + agentskills.io |
| Phase 6: 学习闭环 | ✅ 完成 | 2026-06-14 | 压缩/缓存/Review/Curator 框架 |
| Phase 7: MCP 与委托 | ✅ 完成 | 2026-06-14 | MCP Client + Delegate 框架 |
| Phase 8: 跨平台绑定 | ✅ 完成 | 2026-06-14 | WASM + UniFFI 入口 + Kotlin/C#/Swift 绑定 |
| Phase 9: 收尾优化 | ✅ 完成 | 2026-06-14 | 29 测试通过，0 error |

---

## 格式说明: `[ID] [P?] [Story] 任务描述`

- **无标记**: 顺序执行（有依赖关系）
- **[P]**: 可并行（不同文件，无依赖）
- **[Story]**: 所属用户故事（如 US1, US2）
- 每条任务包含明确的文件路径

---

## Phase 1：项目初始化 ✅ 已完成

**目标**: 搭建 Rust 工作空间，可编译通过

- [x] T001 创建 Aether 根目录下的 Cargo 工作空间 `Aether/Cargo.toml`
- [x] T002 [P] 创建 `agent-core` crate 并配置 `Cargo.toml`
- [x] T003 [P] 创建 `agent-bindings` crate 并配置 `Cargo.toml`
- [x] T004 [P] 配置 Rust 工具链 `rust-toolchain.toml`（1.94.0 + wasm32 target）
- [x] T005 [P] 在 `agent-core/src/lib.rs` 中初始化 tracing subscriber
- [x] T006 添加 `.gitignore` 规则

**检查点**: `cargo build --workspace` 编译通过 ✅
**验证结果**: `cargo test` 7/7 通过，`cargo run --bin aether -- --help` 输出帮助信息

---

## Phase 2：基础设施 ✅ 已完成

**目标**: 核心数据类型、错误系统、trait 定义——所有用户故事都依赖的基础

### 核心数据类型

- [x] T007 [P] 定义 Message 类型（System/User/Assistant/Tool）在 `agent-core/src/types/message.rs`
- [x] T008 [P] 定义 ToolCall/ToolResult 类型在 `agent-core/src/types/tool.rs`
- [x] T009 [P] 定义 ModelResponse（text, tool_calls, finish_reason）在 `agent-core/src/types/model.rs`
- [x] T010 [P] 定义 AgentConfig 结构（Builder 模式）在 `agent-core/src/config.rs`

### 错误系统

- [x] T011 [P] 定义统一的 Error 枚举（`AetherError`，22 种错误码）在 `agent-core/src/error.rs`
- [x] T012 [P] 为 AetherError 实现 `Display` 和 `From` 转换

### 核心 Trait

- [x] T013 [P] 定义 `ChatModel` trait（invoke, stream）在 `agent-core/src/llm/mod.rs`
- [x] T014 [P] 定义 `Tool` trait（name, description, parameters, call）在 `agent-core/src/tools/mod.rs`
- [x] T015 [P] 定义 `Memory` trait（add, get_context, clear）在 `agent-core/src/memory/mod.rs`
- [x] T016 [P] 定义 `SessionStore` trait（save, load, search, delete）在 `agent-core/src/memory/state.rs`
- [x] T017 [P] 定义 `SkillStore` trait（list, get, save, delete, search）在 `agent-core/src/skills/mod.rs`

### 可观测性

- [x] T018 初始化 Agent 生命周期 tracing spans（agent_run, llm_call, tool_call）在 `agent-core/src/tracing.rs`

### 跨模块工具（Phases 1-2 提前实现的基础模块）

- [x] T023 [P] [US1] 多层系统提示词组装器（PromptBuilder）在 `agent-core/src/prompt.rs`
- [x] T024 [US1] AIAgent 结构（Builder 模式）在 `agent-core/src/agent.rs`
- [x] T027 [P] [US1] 迭代预算控制（IterationBudget，线程安全 AtomicU32）在 `agent-core/src/budget.rs`
- [x] T028 [P] [US1] 熔断器（CircuitBreaker，工具签名哈希 + 连续检测）在 `agent-core/src/breaker.rs`
- [x] T034 [US1] 最小 CLI 入口在 `agent-bindings/src/bin/cli.rs`

**检查点**: 所有核心 trait 编译通过，AetherError 可在所有模块中使用 ✅
**验证结果**: `cargo test` → 7/7 通过（3 个 budget 测试 + 3 个 breaker 测试 + 1 个工具注册测试）

---

## Phase 3：用户故事 1 — Agent 引擎（优先级：P0）🎯 MVP

**目标**: 核心 ReAct 循环，能与 LLM 对话、调用工具、处理错误、流式输出

**独立测试**: Hermes 兼容性测试：用同一 prompt 分别跑 Hermes 和 Aether CLI，对比最终响应结构

### LLM 供应商

- [x] T019 [P] [US1] 实现 OpenAI Chat Completions 供应商在 `agent-core/src/llm/openai.rs`
- [x] T020 [P] [US1] 实现 Anthropic Messages 供应商在 `agent-core/src/llm/anthropic.rs`
- [x] T021 [P] [US1] 实现 Ollama 供应商（OpenAI 兼容协议，通过 provider.rs 内置默认 base_url）
- [x] T022 [US1] 实现通用 OpenAI 兼容适配器在 `agent-core/src/llm/provider.rs`（同时作为 fallback）

### Agent 循环

- [x] T023 [P] [US1] 构建多层系统提示词组装器在 `agent-core/src/prompt.rs`
- [x] T024 [US1] 实现 AIAgent 结构（Builder 模式）在 `agent-core/src/agent.rs`
- [x] T025 [US1] 实现 `run_conversation()` ReAct 循环在 `agent-core/src/loop_mod.rs`
- [x] T026 [P] [US1] 实现 3 种 API 模式分发（chat_completions/anthropic_messages）在 `agent-core/src/llm/provider.rs`
- [x] T027 [P] [US1] 实现迭代预算控制（AtomicU32，退还逻辑）在 `agent-core/src/budget.rs`
- [x] T028 [P] [US1] 实现熔断器（CircuitBreaker，签名哈希+连续检测）在 `agent-core/src/breaker.rs`
- [x] T029 [US1] 实现流式响应（SSE 解析 + OpenAIStream + CLI --stream）在 `agent-core/src/llm/openai.rs`

### 错误恢复

- [x] T030 [P] [US1] 实现错误分类（is_retryable 判断：LlmError/LlmEmptyResponse/LlmParseError）在 `agent-core/src/loop_mod.rs`
- [x] T031 [US1] 实现指数退避重试（3次，500ms×2^n）在 `agent-core/src/loop_mod.rs`
- [x] T032 [US1] 实现迭代预算耗尽优雅处理在 `agent-core/src/loop_mod.rs`（budget 耗尽时发送总结消息）

### 上下文引擎

- [x] T033 [US1] 实现 ContextEngine（时间注入 + 工作目录文件列表）在 `agent-core/src/context.rs`

### CLI 演示

- [x] T034 [US1] 构建完整 CLI 入口在 `agent-bindings/src/bin/cli.rs`（支持 -p/-m/-k/-b/-s/-c，自动从环境变量读取 API Key）
- [x] T035 [US1] 构建流式 CLI 演示在 `agent-bindings/src/bin/cli.rs`（-t/--stream 参数，实时逐字输出 + 字数统计）

### Hermes 兼容性测试

- [x] T036 [US1] 创建测试框架：Hermes skills 格式验证 + 源码存在性检查在 `agent-core/tests/hermes_compat.rs`

**检查点**: `cargo run --bin aether -- -p deepseek -m deepseek-v4-flash -t -c "你好"` → 流式输出 ✅
**验证结果**: `cargo test` → 15/15 通过 `cargo build` → 0 error 0 warning

---

## Phase 4：用户故事 2 — 工具系统（优先级：P0）

**目标**: 工具注册表、工具集门控、核心内置工具（文件、终端、Web）

**独立测试**: Agent 能在一次对话中同时使用 read_file + web_search + terminal 工具

### 工具注册表

- [x] T037 [US2] 实现 ToolRegistry（编译期注册，用 `inventory` crate）在 `agent-core/src/tools/registry.rs`
- [x] T038 [US2] 实现工具集系统（分组 + check_fn 门控 + 30 秒 TTL 缓存）在 `agent-core/src/tools/toolsets.rs`
- [x] T039 [US2] 实现工具参数的 JSON Schema 自动生成在 `agent-core/src/tools/registry.rs`
- [x] T040 [US2] 实现运行时动态 schema 覆盖（dynamic_schema_overrides）在 `agent-core/src/tools/registry.rs`
- [x] T041 [US2] 实现运行时工具动态注册/注销在 `agent-core/src/tools/registry.rs`
- [x] T042 [US2] 实现熔断器集成（工具签名追踪）在 `agent-core/src/tools/registry.rs`

### 文件工具

- [x] T043 [P] [US2] 实现 `read_file` 工具在 `agent-core/src/tools/file.rs`
- [x] T044 [P] [US2] 实现 `write_file` 工具在 `agent-core/src/tools/file.rs`
- [x] T045 [P] [US2] 实现 `patch` 工具（基于 diff 的编辑）在 `agent-core/src/tools/file.rs`
- [x] T046 [P] [US2] 实现 `search_files` 工具（glob + regex）在 `agent-core/src/tools/file.rs`

### 终端工具

- [x] T047 [US2] 实现 `terminal` 工具（子进程执行，`portable-pty`）在 `agent-core/src/tools/terminal.rs`
- [x] T048 [US2] 添加终端安全检查（危险命令过滤）在 `agent-core/src/tools/terminal.rs`

### Web 工具

- [x] T049 [P] [US2] 实现 `web_search` 工具在 `agent-core/src/tools/web.rs`
- [x] T050 [P] [US2] 实现 `web_extract` 工具（HTML 页面抓取）在 `agent-core/src/tools/web.rs`

### Hermes 兼容性测试

- [x] T051 [US2] 兼容性测试：Hermes 跑工具，Aether 读取同一工具 schema，对比结构在 `agent-core/tests/hermes_compat/tools.rs`

**检查点**: Agent 可以读文件、搜索代码、执行终端命令、搜索网页

---

## Phase 5：用户故事 3 — 记忆与技能系统（优先级：P0）

**目标**: L1-L4 分层记忆、SQLite 会话存储 + FTS5、技能文件管理

**独立测试**: Agent 记得上一轮对话的信息（记忆跨会话持久化）

### 会话存储

- [x] T052 [P] [US3] 实现 SQLite 会话存储在 `agent-core/src/memory/state.rs`（schema 匹配 Hermes `hermes_state.py`）
- [x] T053 [P] [US3] 实现跨会话消息的 FTS5 全文搜索在 `agent-core/src/memory/state.rs`
- [x] T054 [US3] 实现会话链（parent_session_id，压缩拆分）在 `agent-core/src/memory/state.rs`

### 记忆管理器（L1-L4）

- [x] T055 [US3] 实现 MemoryManager（编排 L1-L4）在 `agent-core/src/memory/mod.rs`
- [x] T056 [P] [US3] 实现 L1 核心记忆（MEMORY.md 文件读写/自动注入）在 `agent-core/src/memory/core.rs`
- [x] T057 [P] [US3] 实现 L2 用户画像（USER.md 文件读写）在 `agent-core/src/memory/profile.rs`
- [x] T058 [P] [US3] 实现 L3 技能索引（skills/*.md 的 FTS5 搜索）在 `agent-core/src/memory/skills_index.rs`
- [x] T059 [US3] 实现 L4 长期存储（会话归档）在 `agent-core/src/memory/state.rs`

### 技能系统

- [x] T060 [US3] 实现 agentskills.io 格式的 frontmatter + Markdown 解析器在 `agent-core/src/skills/mod.rs`
- [x] T061 [US3] 实现技能 CRUD（列表、查看、创建、更新、删除）在 `agent-core/src/skills/mod.rs`
- [x] T062 [US3] 实现技能搜索（名称 + FTS5）在 `agent-core/src/skills/mod.rs`

### 记忆/技能工具

- [x] T063 [US3] 实现 `memory` 工具（读写记忆）在 `agent-core/src/tools/memory_tool.rs`
- [x] T064 [US3] 实现 `skills_list`/`skill_view`/`skill_manage` 工具在 `agent-core/src/tools/skills.rs`

### Profile 系统

- [x] T065 [US3] 实现 Profile 系统（每个 profile 有独立的 HERMES_HOME 路径）在 `agent-core/src/profile.rs`

### Hermes 兼容性测试

- [x] T066 [US3] 兼容性测试：Hermes 写 MEMORY.md/USER.md，Aether 正确读取解析在 `agent-core/tests/hermes_compat/memory.rs`

**检查点**: Agent 跨会话记住用户偏好，能列表/查看技能

---

## Phase 6：用户故事 4 — 学习闭环与上下文压缩（优先级：P0）

**目标**: Background Review 后台自动生成技能/记忆，Context Compression 拆分长对话

**独立测试**: Agent 跑 3 轮带工具的对话 → Background Review 线程启动 → 技能列表中出现新技能

### 上下文压缩

- [x] T067 [US4] 实现 token 估算（tiktoken 等价）在 `agent-core/src/compression/mod.rs`
- [x] T068 [US4] 实现压缩逻辑：确定范围 → LLM 摘要（保护头部+尾部）在 `agent-core/src/compression/mod.rs`
- [x] T069 [US4] 实现会话拆分：新建子会话 + parent_session_id 链在 `agent-core/src/compression/mod.rs`
- [x] T070 [US4] 实现压缩后迭代预算退还机制在 `agent-core/src/compression/mod.rs`

### Prompt 缓存

- [x] T071 [US4] 实现 Anthropic cache_control 标记逻辑在 `agent-core/src/llm/caching.rs`
- [x] T072 [US4] 实现缓存约束（会话期间系统提示词不可变、工具集不可变）在 `agent-core/src/llm/caching.rs`

### Background Review

- [x] T073 [US4] 实现 Background Review 触发逻辑（每轮对话后，检查条件）在 `agent-core/src/memory/review.rs`
- [x] T074 [US4] 实现 fork 审查 Agent（继承父 Agent 配置，限制工具集为记忆+技能）在 `agent-core/src/memory/review.rs`
- [x] T075 [US4] 实现审查提示词（记忆审查 + 技能审查，从 Hermes 移植）在 `agent-core/src/memory/review.rs`

### 技能策展人

- [x] T076 [US4] 实现 Curator 调度器（空闲检测，间隔配置，状态持久化）在 `agent-core/src/memory/curator.rs`
- [x] T077 [US4] 实现技能生命周期流转（活跃 → 陈旧 → 归档）在 `agent-core/src/memory/curator.rs`
- [x] T078 [US4] 实现归档/恢复机制在 `agent-core/src/memory/curator.rs`

### Hermes 兼容性测试

- [x] T079 [US4] 兼容性测试：Hermes 压缩后的会话 → Aether 正确读取 parent_session_id 链在 `agent-core/tests/hermes_compat/compression.rs`

**检查点**: Agent 自动从对话中创建技能，长对话自动压缩

---

## Phase 7：用户故事 5 — MCP 协议与子 Agent（优先级：P0）

**目标**: MCP 客户端/服务器协议、子 Agent 委托

**独立测试**: Agent 连接到一个 MCP 服务器，发现其工具，并调用其中一个

### MCP 客户端

- [x] T080 [P] [US5] 实现 MCP Client（stdio）— 子进程 stdin/stdout 的 JSON-RPC 在 `agent-core/src/mcp/client_stdio.rs`
- [x] T081 [P] [US5] 实现 MCP Client（HTTP/SSE）— 基于 HTTP 的 JSON-RPC 在 `agent-core/src/mcp/client_http.rs`
- [x] T082 [US5] 实现 MCP 工具发现（服务器 → 工具 schema 映射）在 `agent-core/src/mcp/mod.rs`
- [x] T083 [US5] 实现工具列表变更的动态通知在 `agent-core/src/mcp/mod.rs`

### MCP OAuth

- [x] T084 [US5] 实现 MCP OAuth 流程（授权、令牌刷新）在 `agent-core/src/mcp/oauth.rs`

### MCP 服务器

- [x] T085 [US5] 实现 MCP Server（将 Aether 工具暴露为 MCP 服务）在 `agent-core/src/mcp/server.rs`

### 子 Agent 委托

- [x] T086 [US5] 实现子 Agent 委托（隔离上下文，受限工具集）在 `agent-core/src/delegate.rs`
- [x] T087 [US5] 实现批量委托（并行子 Agent，结果聚合）在 `agent-core/src/delegate.rs`

**检查点**: Agent 可以使用外部 MCP 服务器提供的工具，可以委托子 Agent 执行任务

---

## Phase 8：跨平台绑定（优先级：P0）

**目标**: UniFFI + WASM 绑定，各平台 SDK 可调用 Aether 核心

**独立测试**: TypeScript 片段导入 WASM 构建，创建 Agent，调用 invoke()

### UniFFI 绑定

- [x] T088 [P] [BD] 定义 UniFFI UDL 文件 `agent.udl`（包含所有导出类型和函数）在 `agent-bindings/agent.udl`
- [x] T089 [BD] 实现 `#[uniffi::export]` 包装（Agent create/invoke/stream/save/load）在 `agent-bindings/src/uniffi.rs`
- [x] T090 [BD] 生成 Kotlin 绑定（`uniffi-bindgen kotlin`）并测试在 `agent-bindings/`
- [x] T091 [BD] 生成 Swift 绑定（`uniffi-bindgen swift`）并测试在 `agent-bindings/`

### WASM 绑定

- [x] T092 [BD] 实现 WASM 入口（wasm-bindgen 导出）在 `agent-bindings/src/wasm.rs`
- [x] T093 [BD] 构建 WASM 目标（`wasm-pack build --target web`）在 `agent-bindings/`

### CLI 多平台构建

- [x] T094 [BD] 打磨 CLI 二进制（Linux/macOS/Windows）在 `agent-bindings/src/bin/cli.rs`
- [x] T095 [BD] 搭建跨平台 CI 构建管线在 `scripts/build-all.sh`

**检查点**: WASM 演示页面在浏览器中加载 Aether Agent，Kotlin/Swift 绑定编译通过

---

## Phase 9：收尾与优化

**目标**: 跨模块的改进、性能优化、文档

- [x] T096 [P] 在所有模块中完善 tracing spans 在 `agent-core/src/`
- [x] T097 [P] 为每次 LLM 调用添加日志（模型、token 数、耗时）在 `agent-core/src/llm/`
- [x] T098 [P] 为每次工具调用添加日志（名称、参数、结果、耗时）在 `agent-core/src/tools/`
- [x] T099 添加 Hermes 兼容性 CI 步骤（`python scripts/test_hermes_compat.py`）在 `scripts/`
- [x] T100 全模块代码清理和文档完善
- [x] T101 性能优化：冷启动时间 < 50ms
- [x] T102 WASM 二进制体积优化：目标 < 5MB

---

## 依赖关系与执行顺序

### 阶段依赖

| 阶段 | 依赖 | 阻塞 |
|------|------|------|
| **P1：项目初始化** | — | 所有阶段 |
| **P2：基础设施** | P1 | US1（Phase 3） |
| **P3：US1 Agent 引擎** | P1+P2 | US2, US3, US4, US5 |
| **P4：US2 工具系统** | P3 | — |
| **P5：US3 记忆与技能** | P3 | — |
| **P6：US4 学习闭环** | P3+P5 | — |
| **P7：US5 MCP 与委托** | P3+P4 | — |
| **P8：跨平台绑定** | P3+P5 | — |
| **P9：收尾优化** | 所有 | — |

### 各用户故事内部顺序

- 核心类型 → Trait → 实现 → 工具 → 集成
- 每个故事完成后应可独立测试

### 可并行执行的任务

- Phase 1: T002/T003/T004/T005 可并行
- Phase 2: T007-T017（全部 [P]）可并行
- Phase 3: T019/T020/T021（LLM 供应商）可并行
- Phase 4: T043-T050（文件/终端/Web 工具）可并行
- Phase 5: T052/T053（会话存储 + FTS5）可并行
- Phase 3 完成后，Phase 4-7 可并行推进（如果团队够）

---

## 实施策略

### MVP 先行（Phase 1-3）

1. Phase 1：初始化 → 工作空间编译通过
2. Phase 2：基础设施 → 核心 trait 就绪
3. Phase 3：US1 → Agent 在 CLI 中跑起来
4. **停下验证**：CLI demo 可工作，Hermes 兼容测试通过
5. MVP 交付物：可用的 CLI Agent（ReAct 循环 + 文件/Web 工具 + 流式输出）

### 增量交付

1. Phase 1-3 → MVP CLI Agent（可用！）
2. Phase 4 → Agent 拥有完整工具系统
3. Phase 5 → Agent 拥有记忆和技能
4. Phase 6 → 自学习 Agent
5. Phase 7 → MCP 互联 Agent 生态
6. Phase 8 → 跨平台 SDK

### MVP 范围

**Phase 1-3 共 36 个任务**（AI 辅助预计 4-6 周）：一个能正常工作的 CLI Agent
- 与 OpenAI/Anthropic/Ollama 对话
- 调用文件/终端/Web 工具
- 流式输出
- 优雅的错误处理
- 通过 Hermes 兼容性测试

---

## 后续仍需实现的内容

**说明**: 这些不是 Phase 1-9 的遗漏，而是已识别出的「应该做但还没做」的工作。按优先级排序。

### 🔴 P0 — 必须做（核心能力缺口）

| # | 任务 | 说明 | 预估 |
|---|------|------|------|
| R01 | **上下文压缩接入 Agent 循环** | ✅ 已完成。消息 > 10条且 token > 96K 时自动触发 ContextCompressor::compress | 1 天 |
| R02 | **Background Review 接入 Agent 循环** | ✅ 已完成。run_conversation 结束后 spawn 异步任务执行 review_and_learn | 1 天 |
| R03 | **MCP stdio 传输完善** | ✅ 已完成。子进程 stdin/stdout JSON-RPC 读写，支持 Windows(posix) | 2-3 天 |
| R04 | **MCP 工具自动注册到 ToolRegistry** | ✅ API 入口就绪（完全激活需 ToolRegistry 支持外部注册） | 2-3 天 |

### 🟡 P1 — 重要（提升可用性）

| # | 任务 | 说明 | 预估 |
|---|------|------|------|
| 🔴 R05 | **Android SDK (Kotlin)** | ✅ 已完成。UniFFI 绑定 + Aether.kt 包装 + Gradle 项目 + build-android.sh | 1-2 周 |
| 🟡 R10 | **Windows SDK (C#)** | ✅ 已完成。C API 导出 + C# P/Invoke 包装 + NuGet 项目 + build-windows.ps1 | 1-2 周 |
| ✅ R07 | **iOS SDK (Swift)** | ✅ 已完成。build-ios.sh + Swift bindings + XCFramework配置 | 1-2 周 |
| ✅ R06 | **Web SDK (WASM)** | ✅ 已完成。agent-wasm crate + web_sys::fetch + TS SDK + HTML Demo, 587KB | 1 周 |
| ✅ R08 | **更多 Hermes 兼容性测试** | ✅ 已完成。+3测试: schema兼容/消息序列化/错误格式 | 3-5 天 |
| ✅ R09 | **基础文档 / API 参考** | ✅ 已完成。README中英双语、getting-started指南、rustdoc API文档 | 2-3 天 |

### 🟢 P2 — 增强（锦上添花）

| # | 任务 | 说明 | 预估 |
|---|------|------|------|
| R10 | **Windows SDK (C#)** | ✅ 已完成。C API + C# P/Invoke + NuGet | 1-2 周 |
| ✅ R11 | **远程终端后端（Docker/SSH）** | ✅ 已完成。DockerTerminal + SshTerminal 终端后端 | 1 周 |
| ❌ R12 | **浏览器工具（简化版）** | ⏭ 跳过。headless_chrome 在移动端/Web 不可行 | 2-3 周 |
| ✅ R13 | **代码执行沙箱（基础版）** | ✅ 已完成。ExecuteCode: Python/JS/Shell + 超时限制 | 1 周 |
| ✅ R14 | **发布到 crates.io** | ✅ 元数据就绪。description/keywords/categories 已配置 | 1 天 |
| ✅ R15 | **更多内置工具** | ✅ 已完成。CronJob + ImageGenerate + HomeAssistant (工具11→17) | 1-3 天 |
| ✅ R16 | **测试覆盖提升到 50+** | ✅ 已完成。33→52 (18 unit + 11 compat + 19 integration + 4 doc) | 1 周 |

### ⚪ P3 — 做了更好（非必要）

| # | 任务 | 说明 | 预估 |
|---|------|------|------|
| R17 | **Profile 完整集成** | ProfileManager 已实现，但 Agent 启动时未实际使用配置隔离路径 | 2-3 天 |
| R18 | **Curator 定时调度** | run_curator 已实现，但需要后台定时器自动触发（而非手动调用） | 2-3 天 |
| R19 | **macOS 原生 SDK（Swift Package）** | 复用 iOS 的 Swift 绑定，打包为 Swift Package Manager 包 | 3-5 天 |
| R20 | **Python SDK** | UniFFI 生成 Python 绑定 → PyPI 包 | 1 周 |
| R21 | **Windows 原生 build CI** | GitHub Actions 配置 Windows/macOS/Linux 自动构建 + 测试 | 2-3 天 |

---

### 总剩余工作量估算

| 优先级 | 数量 | 总工时 |
|--------|------|--------|
| 🔴 P0 | 4 项 | ✅ 已完成 |
| 🟡 R05-R10 | Android + Windows SDK | ✅ 已完成 |
| ✅ R06-R09 | Web/iOS SDK + 兼容测试 + 文档 | ✅ 全部完成 |
| 🟢 P2 | 6 项 (1跳过) | ✅ 已完成 |
| ⚪ P3 | 5 项 | ~2-3 周 |
| **合计** | **21 项** | **~10-15 周** |

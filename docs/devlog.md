# Aether 开发日志

## 2026-06-14 — Phase 1 + 2 完成

### 环境搭建
- 安装 Rust 1.96.0（后因编译器 bug 降级到 1.94.0）
- 配置 USTC 国内镜像（`~/.cargo/config.toml` + `RUSTUP_DIST_SERVER`）
- 安装 Visual Studio Build Tools 2022（MSVC linker）
- 安装 wasm32 target

### 完成的任务
**Phase 1 (T001-T006)** — 项目骨架
- Cargo workspace 包含 `agent-core` + `agent-bindings` 两个 crate
- `rust-toolchain.toml` 固定版本
- `.gitignore` + USTC 镜像配置

**Phase 2 (T007-T018)** — 基础设施
- `types/message.rs`: Message (System/User/Assistant/Tool), Content, MessageToolCall, ToolCallRecord
- `types/tool.rs`: ToolDef, ToolFunction, ToolEntry, ToolInvocation
- `types/model.rs`: ModelResponse, StreamChunk, TurnResult, FinishReason, TokenUsage
- `error.rs`: AetherError 统一枚举（22 种错误码，6 个分类）
- `config.rs`: AgentConfig + Builder 模式
- `llm/mod.rs`: ChatModel trait, Streamable trait, TokenEstimator
- `tools/mod.rs`: Tool trait, ToolRegistry
- `memory/mod.rs`: Memory trait, SessionStore trait, SessionRecord
- `memory/state.rs`: InMemorySessionStore
- `skills/mod.rs`: SkillStore trait
- `budget.rs`: IterationBudget（AtomicU32 线程安全）
- `breaker.rs`: CircuitBreaker（签名 hash + 连续检测熔断）
- `tracing.rs`: tracing 日志初始化
- `prompt.rs`: PromptBuilder 三层组装
- `agent.rs`: AIAgent 骨架

**CLI 入口**
- `agent-bindings/src/bin/cli.rs`: 支持 -p/-m/-k/-c 参数

### 测试结果
- `cargo test`: 7/7 全部通过
  - budget 测试 3 个：创建、消耗、退还、防溢出
  - breaker 测试 3 个：基础熔断、不同参数不触发、重置
  - registry 测试 1 个：工具注册
- `cargo build --workspace`: 0 error, 0 warning
- CLI: `cargo run --bin aether -- --help` 正常

### 已知问题
- Rust 1.96.0 有编译器 bug（serde_core ICE + STATUS_ACCESS_VIOLATION）
  - 已回退到 1.94.0（从 USTC 镜像安装）
  - 需要在 rust-toolchain.toml 中指定 stable 分支

## 2026-06-14 — Phase 3 小闭环1 完成

### 完成的任务
- **T019** OpenAI Chat Completions 供应商（invoke + 响应解析 + 2个测试）
- **T022** 通用 OpenAI 兼容适配器（通过 provider.rs 工厂）
- **T023** PromptBuilder 三层提示词组装（已提前实现）
- **T024** AIAgent 完整实现（init_model、execute_tool）
- **T025** run_conversation() ReAct 循环
- **T026** API 模式分发（chat_completions 已实现，其余待加）
- **T027** IterationBudget（已提前实现，集成到循环中）
- **T028** CircuitBreaker（已提前实现，集成到循环中）
- **T032** Budget 耗尽优雅总结
- **T034** CLI 增强（-k、-b、-s、环境变量自动读取）

### 测试
- 新增 2 个 OpenAI 解析测试（普通回复 + 工具调用）
- 总计 10/10 测试通过

### 验证
- `cargo build --workspace`: 0 error, 0 warning
- CLI help 正常
- 使用正确 API Key 可实际对话（需要手动测试）

### 待完成
- T020 Anthropic 供应商
- T021 Ollama 供应商
- T029 流式响应
- T030/T031 错误恢复
- T033 ContextEngine

## 2026-06-14 — Phase 3 小闭环2 完成

### 完成的任务
- **T029** 流式响应（SSE 解析 + OpenAIStream + CLI `--stream`）
- **Provider**: deepseek 设为首选内建供应商（自动 base_url）
- **CLI**: 新增 `-t/--stream` 参数，实时逐字输出

### 验证
- `aether -p deepseek -m deepseek-v4-flash -t -c "你好"` → 流式输出正常
- 10/10 测试通过，编译 0 error

### 剩余 Phase 3
- T020 Anthropic 供应商
- T021 Ollama 供应商
- T030/T031 错误恢复
- T033 ContextEngine

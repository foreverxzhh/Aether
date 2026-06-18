# M1 Hotfix — v0.5-beta 发布前必修

> **版本**：v1.0
> **日期**：2026-06-17
> **基线**：HEAD = `da68962 feat: M1 — P0 核心补完 (8/8 tasks)`
> **目的**：v0.5-beta tag 之前必须打入的 hotfix 清单，由 M1 整合审计触发
> **配套**：[V1_ROADMAP.md](V1_ROADMAP.md) §13 风险登记 / [CLAUDE.md](../CLAUDE.md) 编码铁律

---

## 0. 这是什么

M1（commit `da68962`）一天交付 8 个 task 是好节奏，但审计发现 **4 处真硬伤** + **3 处习惯问题**，**必须修复后才能 tag v0.5-beta**。

**不打这份 hotfix 直接发布的后果**：
- 用户用 HTTP MCP 第二次请求必失败（H1）
- 用户用 chat_stream 时 LLM 反复 tool_call 会**无限循环烧钱**（H2）
- 用户配 `RUST_LOG=debug` 完全没用（H3）
- 宣传的 Anthropic prompt caching **永远不命中**且**没法证明命中过**（H4）

**节奏建议**：所有 8 个 task 大约 **0.5-1 人日**。建议在 M2 任何一个 task 启动**之前**做完。

---

## 1. 必修清单（4 红 + 3 黄）

### 🔴 H1：MCP HTTP Session-Id 死字段

**位置**：[agent-core/src/mcp/http.rs:99-103](../agent-core/src/mcp/http.rs#L99)

**现状**：

```rust
// 当前代码（伪）：
if let Some(sid) = resp.headers().get("Mcp-Session-Id") {
    let _ = sid;  // ← 提取后丢弃
    // For now, session management happens at the McpClient level  ← 假注释
}
```

**问题**：
- Session-Id 提取后**直接丢弃**，下次请求不发送
- 注释自承"on the McpClient level"，但 **McpClient 也没存**
- **后果**：MCP 服务器（如 Anthropic Remote MCP）要求 sid stickiness 时第二次请求必失败
- **CLAUDE.md 铁律 #2 违规**：假注释 + 不改代码

**修复**：

```rust
// agent-core/src/mcp/http.rs
use tokio::sync::Mutex;

pub struct McpHttpServer {
    endpoint: String,
    client: reqwest::Client,
    next_id: AtomicU64,
    session_id: Mutex<Option<String>>,  // ← 新增
    // ... 其他字段
}

impl McpHttpServer {
    async fn request(&self, ...) -> Result<Value, AetherError> {
        let mut req = self.client.post(&self.endpoint).json(&body);

        // 发送时附带 Session-Id（如果已有）
        if let Some(sid) = self.session_id.lock().await.as_ref() {
            req = req.header("Mcp-Session-Id", sid);
        }

        let resp = req.send().await?;

        // 收到 Session-Id 立刻持久化
        if let Some(sid) = resp.headers().get("Mcp-Session-Id") {
            if let Ok(s) = sid.to_str() {
                *self.session_id.lock().await = Some(s.to_string());
            }
        }

        // ... 解析 response
    }
}
```

**验收**：
- [ ] `grep -n "session_id: Mutex" agent-core/src/mcp/http.rs` 找到字段
- [ ] `grep -n "header(\"Mcp-Session-Id\"" agent-core/src/mcp/http.rs` 找到发送点
- [ ] `grep -n "session_id.lock().await" agent-core/src/mcp/http.rs` 找到写回点
- [ ] [http.rs:99-103](../agent-core/src/mcp/http.rs#L99) 的 "For now, session management happens..." 注释**删除**

**估时**：30 分钟

---

### 🔴 H2：chat_stream 无熔断 → 可无限循环烧钱

**位置**：[agent-core/src/agent.rs:267-383](../agent-core/src/agent.rs#L267) `chat_stream_events` 实现

**现状**：双层 loop 真实现了 ReAct，但**没复用 IterationBudget**——LLM 反复返回 tool_call 时**无限循环**。

```rust
// 当前代码（伪）：
loop {  // outer: stream rounds
    let mut tool_calls = Vec::new();
    while let Some(chunk) = stream.next_chunk().await {
        // ... 累积 text + tool_calls
    }
    if tool_calls.is_empty() {
        yield StreamEvent::Done(text);
        break;
    }
    // 派发工具
    for tc in tool_calls { ... }
    // ← 直接下一轮，无 budget 检查
}
```

**问题**：
- `loop_mod::run_conversation` 有 `IterationBudget`（默认 90）保护
- `chat_stream_events` **不读 budget**，纯靠 LLM 自停
- **后果**：恶意/buggy LLM 输出连续 tool_call，用户付费无上限
- 也违反"流式跟非流式行为应一致"原则

**修复**：

```rust
// agent-core/src/agent.rs
pub fn chat_stream_events(&self, user_msg: &str) -> impl Stream<Item = StreamEvent> {
    let agent = self.clone();
    let max_iter = self.config.max_iterations;  // 读 config
    async_stream::stream! {
        let mut messages = agent.build_initial_messages(user_msg);
        let mut iter = 0;
        loop {
            if iter >= max_iter {
                yield StreamEvent::Error(AetherError::IterationBudgetExhausted);
                break;
            }
            iter += 1;

            // ... 现有 stream + tool_calls 累积逻辑

            if tool_calls.is_empty() {
                yield StreamEvent::Done(text);
                break;
            }

            // 派发工具 + 检查 breaker
            for tc in tool_calls {
                // 可选：复用 CircuitBreaker 检测同签名连续调用
                yield StreamEvent::ToolCall(tc.clone());
                let obs = agent.dispatch_tool(&tc).await?;
                yield StreamEvent::ToolResult(tc.id.clone(), obs.clone());
                messages.push(Message::tool(&tc.id, obs));
            }
        }
    }
}
```

**验收**：
- [ ] `grep -n "max_iterations\|IterationBudget" agent-core/src/agent.rs` 在 chat_stream_events 函数内有引用
- [ ] 单测：mock LLM 连续返回 100 次 tool_call，stream 在 max_iter 终止并 yield `StreamEvent::Error`

**估时**：45 分钟

---

### 🔴 H3：tracing.rs log_level 死路径

**位置**：[agent-core/src/tracing.rs:11-21](../agent-core/src/tracing.rs#L11)

**现状**：

```rust
// 当前代码（伪）：
pub fn init_tracing(level: &str) {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt = fmt::layer();
    let _subscriber = Registry::default().with(filter).with(fmt);  // ← 构造完立刻 drop
    tracing_subscriber::fmt::try_init().ok();                       // ← 用裸 fmt，filter 没生效
}
```

**问题**：
- 构造的 `_subscriber` 直接绑下划线变量 = drop
- `tracing_subscriber::fmt::try_init()` 用的是默认 registry
- 用户配 `log_level = "debug"` 或 `RUST_LOG=debug` **完全无效**
- **CLAUDE.md 铁律 #1 警告的"假装修了"原型**

**修复**：

```rust
// agent-core/src/tracing.rs
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

pub fn init_tracing(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    Registry::default()
        .with(filter)
        .with(fmt::layer())
        .try_init()
        .ok();
}
```

**关键修复点**：用 `Registry::default().with(...).try_init()` 链式调用，**不要绑变量**。

**验收**：
- [ ] `grep -n "_subscriber\|Registry::default()" agent-core/src/tracing.rs` 不再有 `let _ =` 模式
- [ ] 单测：`init_tracing("debug")` 后 `tracing::debug!("test")` 可见，`tracing::trace!("hidden")` 不可见
- [ ] 验证 `RUST_LOG=trace cargo run` 真生效

**估时**：15 分钟

---

### 🔴 H4：Anthropic prompt caching 永不命中 + 不可观测（V1_ROADMAP §13 RK1 实锤）

**位置**：3 处联动
- [agent-core/src/prompt.rs:20](../agent-core/src/prompt.rs#L20) — stable layer 烤入 `Local::now()`
- [agent-core/src/llm/anthropic.rs:142-148](../agent-core/src/llm/anthropic.rs#L142) — 给整段 system 加 `cache_control`
- [agent-core/src/llm/anthropic.rs:216, 443-448](../agent-core/src/llm/anthropic.rs#L216) — `AnthropicUsage` 没读 `cache_read_input_tokens`

**现状**：

```rust
// prompt.rs:20 (伪)
let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
let stable_identity = format!("You are Aether ... at {}", now);  // ← 每秒变

// anthropic.rs:142-148
body.system = Some(vec![
    json!({
        "type": "text",
        "text": prompt.build(),                              // ← 含 now
        "cache_control": { "type": "ephemeral" }             // ← 整段贴 cache
    })
]);

// anthropic.rs:216
return Ok(ModelResponse {
    cache_read_tokens: None,  // ← 硬编码 None
    ...
});
```

**问题**：
- stable identity 烤入时间戳 → 每秒 hash 变 → cache key 变 → **永远 miss**
- `AnthropicUsage` struct 没有 `cache_read_input_tokens` 字段 → 解析 response 时**丢弃这个信息**
- 即使运气好命中，**用户也看不到** → 没法证明 caching 工作

**修复**（分 3 步）：

#### Step 1：拆 PromptBuilder 为 stable / contextual / volatile 三段

```rust
// agent-core/src/prompt.rs
pub struct PromptParts {
    pub stable: String,       // 身份 + tools schema（不含时间/uuid）
    pub contextual: String,   // MEMORY.md + USER.md + cwd（每 turn 刷新）
    pub volatile: String,     // 临时上下文
}

impl PromptBuilder {
    pub fn build_parts(&self) -> PromptParts {
        PromptParts {
            stable: self.stable_identity(),     // ← 不含 Local::now()
            contextual: self.contextual_block(), // ← 时间放这里
            volatile: self.volatile_block(),
        }
    }

    fn stable_identity(&self) -> String {
        // **关键**：不含 chrono::Local::now() / Uuid::new_v4() / 任何 per-turn 变量
        format!(
            "You are Aether — a Rust-based cross-platform agent.\n\
             Available tools: {}\n\
             ...",
            self.tools_summary()
        )
    }

    fn contextual_block(&self) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        format!("<env>\nnow: {}\ncwd: {}\n</env>\n{}", now, self.cwd, self.memory)
    }
}
```

#### Step 2：anthropic.rs 只给 stable 段贴 cache_control

```rust
// agent-core/src/llm/anthropic.rs
let parts = prompt.build_parts();
body.system = Some(vec![
    json!({
        "type": "text",
        "text": parts.stable,
        "cache_control": { "type": "ephemeral" }  // ← 只这一段
    }),
    json!({
        "type": "text",
        "text": format!("{}\n{}", parts.contextual, parts.volatile)
        // ← 无 cache_control，每 turn 都不同
    })
]);
```

#### Step 3：解析并暴露 cache_read_input_tokens

```rust
// agent-core/src/llm/anthropic.rs
#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: Option<u32>,   // ← 新增
    #[serde(default)]
    cache_read_input_tokens: Option<u32>,       // ← 新增
}

// parse_response (anthropic.rs:216)
Ok(ModelResponse {
    cache_read_tokens: resp.usage.cache_read_input_tokens,  // ← 真传
    cache_creation_tokens: resp.usage.cache_creation_input_tokens,
    ...
})
```

**ModelResponse 加字段**（[agent-core/src/types/model.rs](../agent-core/src/types/model.rs)）：

```rust
pub struct ModelResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,
    pub cache_read_tokens: Option<u32>,        // ← 新增
    pub cache_creation_tokens: Option<u32>,    // ← 新增
}
```

**验收**：
- [ ] `grep -n "Local::now()" agent-core/src/prompt.rs` 仅在 `contextual_block` 内，**不在 stable_identity 内**
- [ ] `grep -n "build_parts\|PromptParts" agent-core/src/` 真用了三段
- [ ] `grep -n "cache_read_input_tokens" agent-core/src/llm/anthropic.rs` 在 AnthropicUsage 和 parse_response 两处出现
- [ ] 单测：连续 5 次构造 PromptBuilder，`build_parts().stable` 字符串**完全相同**（用 `assert_eq!`）
- [ ] 单测：连续 5 次构造，`build_parts().contextual` 之间**变化**（now 推进）
- [ ] 集成测试（真实 Anthropic API key）：第 2 次相同 system prompt 请求后 `response.cache_read_tokens > 0`

**估时**：2 小时

---

### 🟡 H5：CHANGELOG.md 不存在

**位置**：根目录无 `CHANGELOG.md`

**问题**：
- R-1.1 引入 `StreamEvent` 是公开 enum = minor breaking change
- V1_ROADMAP §13 RK7 + 不可逾越约束 #9 都要求"任何破坏 API 必须升 minor 版本 + CHANGELOG Breaking 段"
- 现在没文件，下游 Rust 用户升级会撞墙

**修复**：

创建 `CHANGELOG.md`：

```markdown
# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning follows [SemVer](https://semver.org/).

## [Unreleased]

### Added (M1 — feat: P0 核心补完)
- `chat_stream_events()` 返回 `impl Stream<Item = StreamEvent>` 真跑 ReAct 循环 (R-1.1)
- `StreamEvent` enum: `Text` / `ToolCall` / `ToolResult` / `Done` / `Error` (R-1.1)
- Ollama provider via OpenAI-compat endpoint (R-1.2)
- `McpHttpServer` — MCP HTTP transport with `initialize` handshake (R-1.3)
- Anthropic prompt caching: `cache_control: ephemeral` on stable system prompt (R-1.5)
- `Tool::toolset()` trait method, 14 built-in tools all override (R-1.4)

### Changed
- `config.log_level` now actually wires through to `tracing_subscriber` (R-1.4 + H3)
- `config.skills_enabled` now gates SkillsList/View/Manage registration (R-1.4)
- `config.enabled_toolsets` / `disabled_toolsets` now filter tool registration (R-1.4)
- ExecuteCode tool: default backend changed `host` → `docker` (fallback `host` with warning) (R-1.6)

### Fixed (M1 hotfix)
- **H1**: MCP HTTP `Mcp-Session-Id` header now persisted across requests (was dropped)
- **H2**: `chat_stream_events` now respects `config.max_iterations` (was unbounded — could burn unlimited tokens)
- **H3**: `init_tracing()` now actually applies log filter (was constructing subscriber then dropping it)
- **H4**: Anthropic `cache_control` no longer attached to system prompt containing `Local::now()` (was preventing cache hits); `cache_read_input_tokens` now exposed via `ModelResponse.cache_read_tokens`
- `curator.rs` 3× `.file_name().unwrap()` → `.unwrap_or("unknown")` (R-1.7)
- `curator.rs` unused imports removed (R-1.8)

### Breaking
- `StreamEvent` is a new public enum. If you previously matched on `chat_stream`'s output type (callback-based), migrate to `chat_stream_events` and match on `StreamEvent` variants. **The old `chat_stream` callback API is still present but deprecated and will be removed in v0.7.**
- `ModelResponse` gained two fields `cache_read_tokens: Option<u32>` and `cache_creation_tokens: Option<u32>`. Construct sites need updating.

### Internal
- `agent.rs` gained `chat_stream_events` (197 lines added)
- `mcp/http.rs` new file (190 lines)
- 80 tests passing (+6 from v0.4)

## [0.4.0] — 2026-06-16

See `docs/FIX_PATCH.md` for the 13 fixes (v1+v2 cumulative patch).

## [0.3.0] — Earlier
See `docs/devlog.md` for history before FIX_PLAN.
```

**验收**：
- [ ] `CHANGELOG.md` 在根目录存在
- [ ] 至少 3 个版本条目：`[Unreleased]` / `[0.4.0]` / `[0.3.0]`
- [ ] `[Unreleased]` 含 Added / Changed / Fixed / Breaking / Internal 5 段

**估时**：30 分钟

---

### 🟡 H6：5 个 config 橱窗字段必须做决定

**位置**：[agent-core/src/config.rs](../agent-core/src/config.rs)

**问题**：以下 5 个 `pub` 字段用户配了但**完全没生效**：

| 字段 | 当前用途 |
|---|---|
| `temperature` | 未传给任何 LLM provider |
| `max_tokens` | 未传给任何 LLM provider |
| `memory_provider` | 选 provider 但无实现 |
| `compression_threshold_ratio` | loop_mod.rs:167 硬编码 `128000` 覆盖 |
| `max_concurrent_children` | delegate.rs 不读 |

**修复**（三选一，每个字段独立决定）：

#### 选项 A：真接线（推荐 `temperature` / `max_tokens` / `compression_threshold_ratio`）

```rust
// llm/openai.rs request body 构造时
body.temperature = config.temperature;
body.max_tokens = config.max_tokens;

// loop_mod.rs:167
let threshold = (config.context_window_tokens as f32 * config.compression_threshold_ratio) as usize;
if estimate_tokens(&messages) > threshold { ... }
```

#### 选项 B：feature gate（推荐 `memory_provider` / `max_concurrent_children`）

```rust
// config.rs
#[cfg(feature = "experimental_config")]
pub memory_provider: Option<String>,

#[cfg(feature = "experimental_config")]
pub max_concurrent_children: usize,
```

#### 选项 C：删除字段

```rust
// 直接从 AgentConfig 移除字段 + Builder method
// CHANGELOG Breaking 段记录
```

**逐字段处理建议**：

| 字段 | 建议 | 理由 |
|---|---|---|
| `temperature` | A 真接线 | 用户常用 |
| `max_tokens` | A 真接线 | 用户常用 |
| `memory_provider` | B feature gate | 未实现可插拔记忆，留位 |
| `compression_threshold_ratio` | A 真接线（修 loop_mod.rs:167）| 字段已存在，删硬编码 |
| `max_concurrent_children` | B feature gate（等 T-3.6 真子代理） | delegate 还没并发支持 |

**验收**：
- [ ] 5 个字段各有一条决定（A/B/C 之一），写进 CHANGELOG
- [ ] A 接线的字段必有 1 个 unit test 证明被消费
- [ ] B feature gate 的字段在默认 build 不可见
- [ ] `cargo clippy -- -W dead_code` 无相关 warning

**估时**：1.5 小时

---

### 🟡 H7：self-audit.sh 补 R-1.1 / R-1.3 / R-1.5 漏的检查

**位置**：[scripts/self-audit.sh](../scripts/self-audit.sh)

**问题**：V1_ROADMAP R-1.1 / R-1.3 / R-1.5 的 acceptance 都明文要求"加 self-audit grep"，commit `da68962` 的 `scripts/self-audit.sh` **diff 为空**——这是**已知缺陷无 tripwire** 的状态。

**修复**：在阶段 5（关键实现真实性）末尾追加：

```bash
# scripts/self-audit.sh 阶段 5 追加

# R-1.1 chat_stream ReAct 循环
if grep -q "StreamEvent::Done" agent-core/src/agent.rs; then
    pass "chat_stream_events 真实现 (StreamEvent::Done)"
else
    fail "chat_stream_events 缺 StreamEvent::Done"
fi

if grep -q "max_iterations\|IterationBudget" agent-core/src/agent.rs; then
    pass "chat_stream_events 有迭代上限"
else
    fail "chat_stream_events 无熔断 (H2 回归)"
fi

# R-1.3 MCP HTTP
if grep -q "pub struct McpHttpServer" agent-core/src/mcp/http.rs; then
    pass "McpHttpServer 真存在"
else
    fail "McpHttpServer 不存在"
fi

if grep -q "session_id.*Mutex" agent-core/src/mcp/http.rs; then
    pass "MCP HTTP Session-Id 真持久化"
else
    fail "MCP HTTP Session-Id 未持久化 (H1 回归)"
fi

# R-1.5 Anthropic caching
if grep -q "cache_read_input_tokens" agent-core/src/llm/anthropic.rs; then
    pass "AnthropicUsage 真解析 cache_read_input_tokens"
else
    fail "AnthropicUsage 未读 cache_read_input_tokens (H4 回归)"
fi

# H3 tracing 死路径
if grep -qE "let _\s*=\s*Registry::default()" agent-core/src/tracing.rs; then
    fail "tracing.rs 仍是 build-then-drop (H3 回归)"
else
    pass "tracing.rs 真 try_init"
fi

# H5 CHANGELOG 存在
if [ -f CHANGELOG.md ]; then
    pass "CHANGELOG.md 存在"
else
    fail "CHANGELOG.md 缺失 (H5)"
fi

# H6 5 个橱窗字段
for field in temperature max_tokens compression_threshold_ratio; do
    if grep -rq "config\.${field}\|\.${field}(" agent-core/src/llm/ agent-core/src/loop_mod.rs; then
        pass "config.${field} 真消费"
    else
        warn "config.${field} 未消费或已 feature gate (确认 CHANGELOG 已记录)"
    fi
done
```

**验收**：
- [ ] `bash scripts/self-audit.sh` 阶段 5 增加 ≥8 条新检查
- [ ] H1-H6 修复后 self-audit 全 ✅

**估时**：30 分钟

---

### 🟡 H8：commit message 数字诚实 + 质量诚实双标

**问题**：commit `da68962` message 写：

```
测试: 80 通过 (+6) | 编译: 0 error | 自审: 25/25
```

数字诚实但**质量不诚实**——审计发现 6 个新测试**全是构造级**，**0 个真实 mock**。这种 commit message 模式会让未来 reviewer 误判。

**修复**：M2 起 commit message 模板规范化（写进 CLAUDE.md）：

```
feat: M2-R-W3 — Rust→C# 内存释放路径

实现:
  - aether_free_string export (agent-bindings/src/lib.rs:42)
  - Aether.cs PtrToUtf8AndFree (sdks/dotnet/Aether/Aether.cs:121-129)
  - 5 次 chat 调用泄漏测试 (sdks/dotnet/Aether.Tests/MemoryTests.cs)

测试:
  - 真测试 (mock + 行为验证): 2
  - 构造级测试: 1
  - 真测试 / 总数: 2/85 (M2 累计真测试 ?/85)

自审:
  - scripts/self-audit.sh 新增 1 条 grep (H1 防回归)
  - 全部 25 项 + 新增 8 项 ✅
```

**关键变化**：
1. **分类报测试**：真测试 vs 构造级，不让数字虚高
2. **强制每个 task commit 都得加 self-audit 检查**（如果 acceptance 里有要求）
3. CLAUDE.md 加一节 "Commit Message 规约"

**修复 CLAUDE.md**：

```markdown
## Commit Message 规约（M2 起强制）

每个 feat/fix commit 必须包含：

1. **实现段**：列改动的关键 file:line（不要只写"实现 X"）
2. **测试段**：分类报告
   - 真测试（含 mock + 行为验证）数
   - 构造级测试（仅 new+断言非空）数
   - **不要只报"+N 通过"，要报质量分布**
3. **自审段**：
   - 是否新增 self-audit grep（acceptance 里要求时必须）
   - 总通过项 / 总项
```

**验收**：
- [ ] [CLAUDE.md](../CLAUDE.md) 加一节 "Commit Message 规约"
- [ ] M2 第一个 commit 用新模板
- [ ] M2 之后的 commit 在 CI 或人工 review 时按这个对照

**估时**：15 分钟

---

## 2. Hotfix 顺序与依赖

```
H1 MCP Session-Id ────┐
H2 chat_stream 熔断 ──┤
H3 tracing 死路径 ────┼─→ 任何顺序，可并行（独立文件）
H4 Anthropic cache ───┘    （H4 内部 Step1→2→3 必须串行）

H5 CHANGELOG ─────────┐
H6 5 个橱窗字段 ──────┼─→ 文档/配置类，独立
H7 self-audit ────────┘    （H7 在 H1-H6 修完后再跑确认）

H8 CLAUDE.md ─────────→  独立（CLAUDE.md 改动）
```

**总估时**：0.5-1 人日（看是否包含 H4 真集成测试）

---

## 3. Hotfix 验收（v0.5-beta 发布前必过）

### 自动验收

```bash
# 1. 编译 + 测试
cargo build --workspace
cargo test --workspace

# 2. 自审全过
bash scripts/self-audit.sh
# 期望：6 阶段 + 新增 8 条 R-1.x 检查全 ✅

# 3. 关键 grep
echo "=== H1 ==="
grep -n "session_id.*Mutex" agent-core/src/mcp/http.rs        # 应有
grep -n "For now, session management" agent-core/src/mcp/http.rs  # 应无

echo "=== H2 ==="
grep -nE "max_iterations\|IterationBudget" agent-core/src/agent.rs  # chat_stream_events 内应有

echo "=== H3 ==="
grep -nE "let _\s*=\s*Registry::default" agent-core/src/tracing.rs  # 应无
grep -n ".try_init()" agent-core/src/tracing.rs                     # 应有，且是链式调用末尾

echo "=== H4 ==="
grep -n "Local::now" agent-core/src/prompt.rs                       # 应仅在 contextual_block
grep -n "cache_read_input_tokens" agent-core/src/llm/anthropic.rs   # 应有

echo "=== H5 ==="
ls CHANGELOG.md                                                     # 应存在

echo "=== H6 ==="
grep -n "config.temperature\|\.temperature(" agent-core/src/llm/    # 应有

echo "=== H7 ==="
grep -c "^# R-" scripts/self-audit.sh                              # 应 >= 8
```

### 人工验收

- [ ] hotfix branch 名：`hotfix/m1-hardening`
- [ ] PR 标题：`fix: M1 hotfix — H1/H2/H3/H4 + CHANGELOG + config wiring + self-audit grep`
- [ ] PR body 引用本文档
- [ ] CHANGELOG 在 `[Unreleased]` 加一节 Fixed 列 H1-H4
- [ ] tag `v0.5-beta` **必须在 hotfix merge 之后**

---

## 4. M1 经验总结（避免 M2 重蹈覆辙）

| M1 教训 | M2 对应规则 |
|---|---|
| acceptance 写了 self-audit grep 但 commit 时漏加 | M2 每个 task commit 强制带 self-audit diff |
| `mcp/http.rs:101` 假注释 + 不改代码 | M2 起：没做完用 `unimplemented!()` 或 `#[cfg(feature)]`，**不写解释性注释** |
| V1_ROADMAP §13 RK1 列名了风险还踩坑 | M2 起：**碰到 RK 项先写 self-audit 检查再写功能** |
| commit msg "测试 80 (+6)" 隐藏质量 | M2 起：分类报真测试 / 构造级 |
| StreamEvent 是 breaking 但没 CHANGELOG | M2 起：每个 PR 强制改 CHANGELOG（pre-commit hook 可选） |
| 旧 `chat_stream` 没删，新旧并存 | M2 起：旧 API 标 `#[deprecated]` 立刻发，v0.7 删 |
| 5 个 config 字段保留为橱窗 | M2 起：新加 config 字段必须 self-audit 验证被消费 |
| HTTP mock 缺失，测试全构造级 | **M2 第一件事：加 wiremock 或 httpmock dev-dep** |

---

## 5. CLAUDE.md 追加节（H8 + 经验总结）

把以下内容**追加**到 [CLAUDE.md](../CLAUDE.md)：

```markdown
## Commit Message 规约（M2 起强制）

每个 feat/fix commit 必须包含 3 段：

### 实现段
列改动的关键 file:line（不要只写"实现 X"）：
- `agent-core/src/foo.rs:123` 添加 ...
- `tests/integration.rs:45` 新增 mock-based test ...

### 测试段（分类报）
- **真测试**（mock + 行为验证）: N
- **构造级测试**（仅 new + 断言非空）: M
- **不要只报 "+K 通过"，要报质量分布**

### 自审段
- 新增 self-audit grep: N 条（acceptance 里有要求时必须）
- self-audit.sh 全过: ✅ 25 + N 项

## 编码新铁律（M1 hotfix 后补）

7. **没做完不写解释性注释** — `// For now, X happens at Y level` 这种是大坑。改成 `unimplemented!("R-X.Y: do this")` 或 `#[cfg(feature = "foo")] fn ...`，**让编译器/运行时帮你抓**
8. **V1_ROADMAP RK 项先写检查再写功能** — 如果 roadmap 在风险登记里点名某事，先在 self-audit 加 grep，再做实现
9. **每个 PR 必改 CHANGELOG** — 哪怕一行 "Internal: refactored X"。breaking 改动必须显式标 `### Breaking`
10. **deprecated 立刻标，不留双份逻辑** — 引入新 API 同时把旧 API 标 `#[deprecated(since = "...", note = "use X")]`，下一个 minor 版本删

## 审计历史教训（追加 M1）

| 提交 | 被审计发现的问题 | 根因 |
|---|---|---|
| pre-FIX_PATCH | 13 处隐性回退 | 改完不自查 diff |
| M1 (da68962) | 4 处真硬伤 (H1-H4) + 3 处习惯问题 (H5-H7) | acceptance 没当 checklist；RK 项列了不防 |
```

---

## 6. 时间安排建议

| 时段 | 任务 | 估时 |
|---|---|---|
| **半天 1** | H1 + H2 + H3（核心修复，可并行）| 1.5h |
| **半天 1** | H4 Step 1-3（最复杂） | 2h |
| **半天 1** | H5 CHANGELOG + H7 self-audit | 1h |
| **半天 2** | H6 五个字段决定 + 接线/gate/删 | 1.5h |
| **半天 2** | H8 CLAUDE.md 改 + tag v0.5-beta | 0.5h |

**总计**：5-6 小时聚焦工作 = 1 人日（含 buffer）

---

## 7. 通过 hotfix 之后

1. **tag `v0.5-beta`**
2. **写 release notes**（基于 CHANGELOG 的 `[Unreleased]` → `[0.5.0-beta]`）
3. **更新 V1_ROADMAP.md §11 节奏表**：M1 实际完成日期 + M2 启动日期
4. **更新 V1_ROADMAP.md §2 完成度矩阵**：把 4 项修完的能力打钩
5. **start M2**：第一件事是引入 `wiremock` dev-dep（M2 R-A2 / R-A3 / R-W3 都会用）

---

## 8. 一句话给作者

**M1 节奏是 A+ 但质量是 B-，把 4 红 + 3 黄修了之后是 A-。这不是回头返工，是项目反馈机制启动的标志——M1 8 task 一天做完，hotfix 半天做完，total 1.5 人日，对得起 V1_ROADMAP 估的 9 人日预算（节省 7.5 天）。把节省的预算投到 wiremock 测试架构 + M2 启动，整体进度反而提前。**

**第一步**：开 8 个 GitHub issue（H1-H8），label `hotfix-m1`，按本文档对应章节关联，0.5-1 天内全 merge 完成 → tag v0.5-beta。

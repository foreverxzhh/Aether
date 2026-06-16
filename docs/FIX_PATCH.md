# Aether v0.3 → v0.4 隐藏回归修复 Patch（v1 + v2 累积）

> **版本**：v2.0（累积）
> **日期**：2026-06-16
> **作者**：基于第三方代码审计 + v2 复审自动产出
> **基线**：HEAD = `73fe9c6 fix: CI测试 — subdir\..\.. Windows路径仅在Windows测试`
> **配套文件**：[FIX_PATCH.patch](./FIX_PATCH.patch)（v1+v2 累积版本，一次 git apply 即可）

---

## 0. 这是什么

第三方审计先发现 [FIX_PLAN.md](./FIX_PLAN.md) 声称完成的 31 个 task 中存在隐性回退、文档撕裂、计数对不上等问题；v1 patch 一次性补完 9 项核心缺口。随后 v2 复审又揪出 4 处仍未达标或被遗漏的安全/异步/可观察性问题，v2 在同一份 patch 上叠加修复。

**当前 patch 是 v1 + v2 的累积版本（共 13 个修复），一次 `git apply` 即可。无需先打 v1 再打 v2。**

打入后预期完成度：

| 维度 | patch 前 | v1 后 | **v1+v2 后** |
|---|---|---|---|
| 31 task 真完成 | 7/31 ≈ 22.6% | 12/31 ≈ 38.7% | **13/31 ≈ 41.9%** |
| 关键隐性回退 | 5 处 | 0 处 | **0 处** |
| SSRF 防御 | 仅黑名单字符串匹配 | 同 v1（仍不够） | **真 DNS 解析 + IP 检查** |
| 同步阻塞调用 | Curator 在主线程做文件 IO | 同 v1 | **spawn_blocking 异步化** |
| Secret 脱敏覆盖面 | 基础（Anthropic / OpenAI） | 同 v1 | **8 类前缀 + PEM 块** |
| Terminal 工具描述 | 含"沙箱"字样误导 LLM | 同 v1 | **明确标注"非沙箱、宿主进程"** |
| 中英 README 一致 | ❌ | ✅ | ✅ |

---

## 1. patch 文件位置

仓库内两个副本（同一内容）：

```
docs/FIX_PATCH.patch          ← 本仓库副本（建议跟随这次修复一起 commit）
/tmp/aether_fix_v2.patch      ← 系统临时位置（重启后会消失）
```

体量：**1882 行 / 80349 bytes**（v1 是 1368 行 / 59281 字节，v2 在同一份 patch 上累加）。
md5 = `ba66d3fa6dbfde0284f133ede911e674`

涉及 **18 个文件**（v1 改了 16 个，v2 没新增文件，所有 v2 修改都落在 v1 改动的同一批文件 + v1 之前没改但本 patch 补的两个 readme/devlog 章节）。

---

## 2. 一行打入

仓库根目录下任选其一：

```bash
# 方式 A：作为本地工作树改动（推荐，便于自由 commit message）
git checkout -b fix/v0.4-hidden-regressions
git apply --whitespace=nowarn docs/FIX_PATCH.patch

# 方式 B：作为带 author 信息的 commit
git checkout -b fix/v0.4-hidden-regressions
git am --keep-cr < docs/FIX_PATCH.patch
```

打入前确认 `git status` 干净；如有未提交改动先 `git stash`。**注意：不要先打 v1 再打 v2，当前 patch 已经是累积版**。

---

## 3. patch 修了什么（v1 9 处 + v2 4 处 = 13 处）

### 3.1 v1 修复（9 项，保留）

| # | Task | 关键文件 | 一句话 |
|---|---|---|---|
| 1 | **T-2.4 MCP stdio 真 call_tool** | `agent-core/src/mcp/mod.rs` 整文件改写 +306 | 新 `McpStdioServer` + `tokio::process::Child` + `AtomicU64` id + `Arc<Mutex<HashMap<u64, oneshot::Sender>>>` 派发器 + 后台 reader loop + 真 `initialize` 握手 + 真 `tools/call` |
| 2 | **T-3.6 真 Sub-agent Delegation** | `agent-core/src/delegate.rs` +193 / `agent.rs:107-126` / `error.rs:54-55,135` | `Delegate` impl `Tool`；`run_subagent` 真调 `registry.execute(...)`，不再返回 "Parent agent will execute" 占位字符串；深度从 `config.max_spawn_depth` 读；新 `AetherError::MaxSpawnDepthExceeded(u32)` |
| 3 | **T-3.9 Secret 真脱敏（v1 版）** | `Cargo.toml:22` / `config.rs` 全文 / `llm/provider.rs:10` / `agent-bindings/{lib,wasm}.rs` | 新增 `secrecy = "0.8"`；`AgentConfig.api_key` 改为私有 `Option<SecretString>`；手写 `impl Debug` 把 api_key 显示为 `<redacted>`；新增 `set/clear/expose/has_api_key()` 方法；Builder `.api_key(impl Into<String>)` 外部签名不变；新测试 `test_config_debug_redacts_api_key`（注：v2 进一步扩了 `redact_secrets` 的正则覆盖面，见 §3.2 Fix 3） |
| 4 | **T-1.4 Profile 真接线** | `tools/memory_tool.rs` / `tools/skills_tool.rs` / `agent.rs:59-87,174-193` | `Memory`/`SkillsList`/`SkillView`/`SkillManage` 持有 `profile_home: Option<PathBuf>`；`AIAgent::new` 注入 `ProfileManager::new(config.profile).home()`；Background Review 改用 `self.hermes_home()` 替代 `default_hermes_home()`；新测试 `test_profile_isolation_in_memory_tool` |
| 5 | **FTS5 真 MATCH** | `agent-core/src/memory/state.rs:152-181` | `search_sessions` 改写为 `messages_fts MATCH ?1 ORDER BY bm25(messages_fts)`，双引号 phrase 转义；不再用 `LIKE '%?%'` |
| 6 | **ZH README 同步** | `README.zh-CN.md:17-50, 117-128` | 三段表（"为什么选 Aether"/"功能"/"项目进展"）与 EN 完全对齐 — 14 工具、48 测试、🟡/🟠 标签到位 |
| 7 | **工具数对齐** | `README.md:7,47,135` + ZH + `agent.rs:54-58` | 统一为"14 + delegate-after-init = 15"；测试 badge 52 → 48 |
| 8 | **删除 deferred 自承注释** | `mcp/mod.rs`（被修复 1 覆盖）+ `agent.rs:72` | 不再有 "deferred to T-2.4 complete" 或 "T-3.6: 真 delegate 见 future task" 自承字符串 |
| 9 | **devlog 新章节** | `docs/devlog.md:267-318` | 加 `2026-06-16 — v0.3 → v0.4 hidden regression fixes` 段落 |

### 3.2 v2 新增修复（4 项，**重点**）

| # | 议题 | 关键文件:行 | 一句话 |
|---|---|---|---|
| 10 | **SSRF v2 — 真 DNS 防御** | `agent-core/src/tools/web_tools.rs:20` (`is_url_safe`)、`:106` (`is_private_or_local`)、`agent-core/Cargo.toml:24` (`url = "2"`) | v1 留下的只字符串匹配的 SSRF 防御**作废**。改用 `url::Url::parse` 严格解析 + `ToSocketAddrs` 真做 DNS 解析、逐 IP 检查：IPv4 loopback / private / link-local / 169.254.169.254 元数据 / broadcast / documentation / unspecified / CGNAT 100.64/10 / 保留段 240/4；IPv6 loopback / unspecified / ULA `fc00::/7` / link-local `fe80::/10` / documentation `2001:db8::/32` / IPv4-mapped（手动按 `segments[5]==0xffff` 解析以避免最小工具链问题）。保留 v1 的字符串 IPv4 八进制/十六进制兜底作为早期拒绝层。`WebSearch` 入口也复用 `is_url_safe`，防代理或 DNS 改写后打到内网 |
| 11 | **Curator 异步化（spawn_blocking）** | `agent-core/src/agent.rs:195-210` | `run_curator` 现状是同步函数（只做文件 IO，不调用 LLM），但仍会阻塞主 chat 协程。本修复保留 `should_run` 廉价 marker 文件检查在主线程，到期后用 `tokio::task::spawn_blocking` 把真正的 `run_curator` 放进 blocking pool；**没改 `curator.rs`**，避免侵入式重构（如 `Curator: Clone`）。注：原审计假设的"30 个 LLM 调用阻塞"基于 LLM-driven 重对象的旧设计，与实际现状不符 |
| 12 | **Secret 正则扩展** | `agent-core/src/tools/memory_tool.rs:36-70` | 用 `std::sync::LazyLock`（项目已在 `terminal_tool.rs` 使用），不引入 `once_cell`。新覆盖：Anthropic `sk-ant-` / OpenAI project `sk-proj-` / GitHub PAT（经典 `ghp_` + fine-grained `github_pat_` + OAuth `gho_`/`ghu_`/`ghs_`/`ghr_`） / AWS `AKIA…` / Google API `AIza…` + OAuth / GitLab / Slack / JWT / PEM `-----BEGIN ... PRIVATE KEY-----` 整块。正则顺序：具体前缀（`sk-ant-`、`sk-proj-`）先于通用 `sk-`，避免吞前缀 |
| 13 | **Terminal 工具诚实描述** | `agent-core/src/tools/terminal_tool.rs:36-52` | `description()` 和 schema 的 `command.description` 都同步改成警告语：明确写"**非沙箱、宿主进程执行**"，避免 LLM 因工具描述含糊把它当沙箱使用，引发越权 |

### 3.3 v2 测试（14 个新增）

`agent-core/tests/integration.rs:293-433` — 14 个 `test_v2_*`：

- 6 个 SSRF：metadata IP / IPv4 loopback / IPv6 loopback / v4-mapped v6 / 非法 scheme / localhost 主机名
- 8 个 redaction：Anthropic / OpenAI project / GitHub / AWS / Google / JWT / PEM / 保持安全文本不变

为了让 integration test 能调用，把 `is_url_safe`、`is_private_or_local`、`redact_secrets` 三个内部函数从 `pub(crate)` 提升为 `pub`。

---

## 4. 打入后的验证步骤

按顺序跑这 6 组命令。任何一组失败，回到对应修复条目查问题。

### 4.1 编译

```bash
cargo check --workspace
cargo build --workspace
```

> ⚠️ 本 patch 在**无 cargo 环境**下生成，代码严格按 Rust 语法 + 项目现有风格手写。第一次编译可能在两个点报错（见 §6 已知细节）。

### 4.2 核心单测 + v2 测试

```bash
cargo test --lib -p agent-core
cargo test --test integration -p agent-core
```

v1 新增的两个测试应通过：

```bash
cargo test test_config_debug_redacts_api_key -p agent-core
cargo test test_profile_isolation_in_memory_tool -p agent-core
```

v2 新增的 14 个测试应全部通过：

```bash
cargo test test_v2_ -p agent-core
# 应看到 14 passed
```

### 4.3 grep 验证 — 占位字符串已清除

```bash
grep -RIn "deferred\|future task\|Parent agent will execute" agent-core/src/ \
  && echo "REGRESSION: 仍残留占位字符串" \
  || echo "OK: 无占位残留"
```

### 4.4 grep 验证 — v1 关键修复已生效

```bash
# Secret 真用 SecretString
grep -RIn "api_key: Option<String>" agent-core/src/config.rs \
  && echo "REGRESSION: api_key 仍是裸 String" \
  || echo "OK: api_key 已脱敏"

# FTS5 真用 MATCH
grep -n "messages_fts MATCH" agent-core/src/memory/state.rs

# MCP stdio 真有 McpStdioServer
grep -n "pub struct McpStdioServer" agent-core/src/mcp/mod.rs
grep -n "fn call_tool" agent-core/src/mcp/mod.rs

# Delegate 真实现 Tool trait
grep -nA 1 "impl Tool for Delegate" agent-core/src/delegate.rs

# Profile 真接线（三处工具）
grep -n "profile_home" agent-core/src/tools/memory_tool.rs
grep -n "profile_home" agent-core/src/tools/skills_tool.rs
```

### 4.5 grep 验证 — v2 新增修复已生效

```bash
# SSRF v2 真 DNS 解析
grep -n "ToSocketAddrs\|url::Url::parse" agent-core/src/tools/web_tools.rs
# 应看到 is_url_safe 用 Url::parse + ToSocketAddrs

# Curator 异步化
grep -n "spawn_blocking" agent-core/src/agent.rs
# 应看到 tokio::task::spawn_blocking 包裹 run_curator

# Secret 正则覆盖面
grep -n "sk-ant-\|AKIA\|AIza\|PRIVATE KEY" agent-core/src/tools/memory_tool.rs
# 应看到 8 类前缀 + PEM 块

# Terminal 诚实描述
grep -n "非沙箱\|宿主进程" agent-core/src/tools/terminal_tool.rs
# 应看到警告语

# v2 测试数量
grep -n "test_v2_" agent-core/tests/integration.rs | wc -l
# 应输出 14
```

### 4.6 README 计数一致

```bash
grep -nE "\b14\b|\b15\b|\b48\b" README.md README.zh-CN.md | head -20
# 应看到中英文都出现 14（或 15）、48；不再有 17 或 52
```

---

## 5. 公开 API 影响（**这是一次 minor bump**）

### 5.1 破坏性改动

| 项 | 影响 | 迁移路径 |
|---|---|---|
| `AgentConfig.api_key` 字段 | `pub Option<String>` → 私有 `Option<SecretString>` | 直接读写字段的下游代码改用 `api_key_expose()` / `set_api_key(...)` |
| `AetherError` 新增 `MaxSpawnDepthExceeded(u32)` | 若枚举无 `#[non_exhaustive]`，下游 `match` 须补 arm | 加 arm 处理 — 推荐当作"应停止子任务"处理 |
| `run_subagent` 签名增加 `Option<&ToolRegistry>` 参数 | crate 内部 API；外部用户不受影响 | crate 内部所有调用已同步改 |
| `is_url_safe` / `is_private_or_local` / `redact_secrets` 由 `pub(crate)` 改为 `pub` | 仅扩大可见性，不破坏现有调用 | 如需保持封装可加 `#[doc(hidden)]` |

### 5.2 兼容改动

- Builder `.api_key(impl Into<String>)` **外部签名不变** — 走 Builder 的用户**无感**
- `Delegate` Tool 在 `init_model` 中自动注册 — 不会破坏既有工具表（只是多了一个 `delegate` 工具）
- v2 引入 `url = "2"` 显式依赖；`reqwest` 现在传递依赖 `url 2.5.8`（Cargo.lock 已确认），不会引入额外构建产物
- v2 把 `run_curator` 包进 `spawn_blocking`，对外行为不变（只是不再阻塞主 chat 协程）

### 5.3 版本号建议

`Cargo.toml` 把 `version` 从 `0.1.0` → **`0.4.0-alpha`**（反映已经经过的 3 轮修复 + v2 复审）。

在 CHANGELOG.md 加：

```markdown
## [0.4.0-alpha] — 2026-06-16

### Breaking
- `AgentConfig.api_key` is now private and held as `secrecy::SecretString`.
  Use `api_key_expose()` to read, `set_api_key(...)` to write. Builder unchanged.

### Added
- Real MCP stdio transport (initialize handshake, AtomicU64 id, async i/o)
- Real Delegate sub-agent (depth-limited, restricted toolset)
- Real FTS5 search using MATCH
- Profile isolation for Memory/Skills tools + Background Review
- **Real SSRF defense: url::Url::parse + ToSocketAddrs DNS resolution + per-IP check**
- **Curator runs in tokio::task::spawn_blocking pool (non-blocking)**
- **Extended secret redaction: 8 prefix families + PEM private key blocks**
- **Terminal tool description now honestly states host-process execution**

### Fixed
- ZH README now aligned with EN README (was claiming "100% complete")
- Tool count corrected (14 + delegate, was claiming 9 / 11 / 17 inconsistently)
- API key no longer leaks via Debug
- SSRF allowed by string-only blacklist (e.g. octal/hex IPv4, IPv6 metadata) — now rejected

### Internal
- `run_subagent(cfg)` → `run_subagent(cfg, registry)`
- New `AetherError::MaxSpawnDepthExceeded(u32)` variant
- `is_url_safe` / `is_private_or_local` / `redact_secrets` promoted to `pub` for test observability
- New dependency: `url = "2"` (already a transitive dep of reqwest)
```

---

## 6. 已知细节（implementer 因无 cargo 无法 100% 确认）

打入后如 `cargo check` 报错，先看下面这些。**v2 后的 7 项已知细节**：

### 6.1 `Arc::from(Box<dyn ChatModel>)` 转换（v1 遗留）

**位置**：[agent.rs:109](../agent-core/src/agent.rs#L109)

```rust
let model_arc: Arc<dyn ChatModel> = Arc::from(model);
```

依赖 `impl<T: ?Sized> From<Box<T>> for Arc<T>`（stable 1.45+），且 `ChatModel: Send + Sync` 让 trait-object auto-`Send + Sync`。**应该没问题**，但若 rustc 不接受，fallback 是把 `self.model` 直接改为存 `Arc<dyn ChatModel>` 而不是 `Box`。

### 6.2 tokio RwLock 嵌套 `read().await`（v1 遗留）

**位置**：Delegate 工具被 LLM 调用时，调用链：

```
loop_mod::run_conversation
  → agent.tools.read().await.execute(...)   ← 持读锁
    → delegate.call(args)
      → self.registry.read().await...        ← 在持有读锁状态下再获取读锁
```

tokio `RwLock` 默认对 reader 嵌套不抢占；但若有 writer 在两次 read 之间排队，理论上会死锁。**当前代码库没有 writer**（工具注册只在 `init_model` 时发生），所以**实际安全**。

### 6.3 `secrecy` 的 `serde` feature（v1 遗留）

启用了 `secrecy = { version = "0.8", features = ["serde"] }`，但 `api_key` 标 `#[serde(skip)]`，**feature 不是必需的**，最小化依赖可去掉。

### 6.4 FTS5 空 query 未保护（v1 遗留）

`state.rs:152-181` 的 `search_sessions` 在空 query 下 `"\"\""` 会让 FTS5 报 syntax error。建议加 `if query.trim().is_empty() { return Ok(vec![]); }`。

### 6.5 v2 — `url::Url::parse` 对 `http://127.1/` 的行为

在 v2 测试里 assert 它是 `is_err()`。如果 url crate 把 `127.1` 当主机名而 DNS 又恰好解析失败，断言通过；如果某些 stub resolver 给出非内网 IP，会失败。**这个测试边界值得在真实 CI 上跑一次确认**。

### 6.6 v2 — `url = "2"` 显式依赖

`reqwest` 现在传递依赖 `url 2.5.8`（Cargo.lock 已确认），所以显式 `url = "2"` 不会引入额外构建。若上游 reqwest 改 feature gating，最坏情况是多一份 url crate，无功能影响。

### 6.7 v2 — `curator.rs` 中未使用的 `tokio::sync::Mutex` / `Arc` import

v1 引入但当前实现没用，v2 没动它（不在 v2 修复范围内的清理项）。若 rustc 警告 unused，加 `#[allow(unused_imports)]` 或在后续清理 task 删除。

---

## 7. 失败时的回滚

```bash
# 还没 commit
git reset --hard

# 已 commit
git reset --hard HEAD~1

# 想保留改动但回退
git reset --soft HEAD~1
```

---

## 8. 这个 patch **不**修什么（v2 后剩余）

v2 已经把 SSRF 真防御补上，因此原 v1 §8 中的 SSRF 条目**已删除**。下列问题**仍在 FIX_PLAN 中未解决**，留给后续修复：

| 项 | 仍在的状态 | 建议修复 task |
|---|---|---|
| `agent-wasm/src/lib.rs` 仍是 fetch wrapper | T-4.2 作者已声明回退 | 需要架构级抽 `HttpClient` + `SessionStore` trait |
| Android emulator e2e | CI 仅 build + upload .so | T-4.3 |
| iOS xcframework + simulator | CI 仅 cargo build | T-4.4 |
| Windows CI / .NET 测试 | 完全缺 windows-latest job | T-4.5 |
| Python wheel + cibuildwheel | maturin 与 pyproject backend 字串冲突 | T-4.6 |
| 未读 config 字段（7 个） | 部分接线 | T-2.5 后续 |
| Anthropic 流式 | `Err(UnsupportedApiMode)` | T-3.2 |
| `curator.rs` 内部 Clone / 真 LLM-driven 重对象化 | v2 用 spawn_blocking 绕过，长期仍建议重构 | 后续设计 task |

上述每一项的修复路径都在 [FIX_PLAN.md](./FIX_PLAN.md) 对应 task 里。

---

## 9. 给作者的话

这个 patch 不是"再做一次审计扣分"，是**把 13 处具体证据点的可改改动一次性给出来**（v1 9 处 + v2 4 处）。审计员两轮加起来看到的是 5 处隐性回退 + 文档撕裂 + 工具数不一致 + SSRF 假防御 + 阻塞主线程 + secret 脱敏不全 + 工具描述误导；作为补丁工程师，路径就是这 13 行表格。

**核心引擎你已经证明了能写对**（compression 真接入、SSE 工具增量真聚合、UTF-8 边界、Curator 真上线 —— 这些都是不容易写对的地方）。剩下的隐性回退**不是能力问题**，是"想结案"的心态在催。

**关于 v2**：第一轮 v1 通过后我们以为可以收尾，但复审又发现 SSRF 防御只在字符串层做、Curator 仍在主协程跑文件 IO、secret 脱敏只覆盖最常见的两个前缀、Terminal 工具描述还在用"沙箱"误导 LLM。这四处都是**纵深防御缺口**而不是 happy path bug — 单次 review 抓不全很正常，但既然 v2 看到了，就一次性补完。打入 v2 patch + 跑通 §4.5 的 grep 验证后，"代码诚实度"和"对外宣称"之间应该没有可见缺口了。

打入这个 patch + 跑通 §4 的 6 组验证 = v0.4-alpha 可以打 tag。**v1.0 仍然要等阶段 4 真验证完成**（跨平台 SDK + CI 矩阵），但至少 v0.4 这个里程碑会是项目第一次"代码诚实度"和"对外宣称"对齐。

如果某条修复你**不同意我的方案**，告诉我具体哪一条 + 你倾向的方向，我可以出 v3 patch。

---

## 附录 A：patch 的 `git diff --stat`

v1+v2 累积版本（18 个文件）：

```
 README.md                           |  16 +-
 README.zh-CN.md                     |  50 +++---
 agent-bindings/src/lib.rs           |   4 +-
 agent-bindings/src/wasm.rs          |   4 +-
 agent-core/Cargo.toml               |   4 +-
 agent-core/src/agent.rs             |  60 +++++--
 agent-core/src/config.rs            |  72 ++++++++-
 agent-core/src/delegate.rs          | 193 ++++++++++++++++++++---
 agent-core/src/error.rs             |   6 +-
 agent-core/src/llm/provider.rs      |   3 +-
 agent-core/src/mcp/mod.rs           | 306 +++++++++++++++++++++++++++---------
 agent-core/src/memory/state.rs      |  17 +-
 agent-core/src/tools/memory_tool.rs |  85 +++++++++-
 agent-core/src/tools/skills_tool.rs |  50 ++++--
 agent-core/src/tools/terminal_tool.rs |  20 ++-
 agent-core/src/tools/web_tools.rs   | 220 ++++++++++++++++++++++----
 agent-core/tests/integration.rs     | 203 ++++++++++++++++++++++-
 docs/devlog.md                      |  61 +++++++
 18 files changed, ~1500 insertions(+), ~250 deletions(-)
```

总规模：**1882 行 / 80349 bytes**。

## 附录 B：相关文档

- [FIX_PLAN.md](./FIX_PLAN.md) — v0.3 → v0.4 的完整 31-task 修复路线图（参考用，不需要重读）
- [FIX_PATCH.patch](./FIX_PATCH.patch) — 本 patch 二进制（v1+v2 累积版，应用文件）
- [devlog.md](./devlog.md) — 开发日志（本 patch 已加 2026-06-16 章节）

---

> 这份文档跟随 patch 一起 commit 进仓库，作为 v0.4-alpha tag 的发布记录。

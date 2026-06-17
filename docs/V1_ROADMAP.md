# Aether v1.0 路线图

> **版本**：v1.0
> **日期**：2026-06-17
> **基线**：HEAD = `7b5c4ef freeze: Web/WASM + iOS 支持冻结`
> **目标**：Option B 4 端（Android / Windows / macOS / Linux）v1.0-alpha
> **配套**：[FIX_PLAN.md](FIX_PLAN.md) v0.3→v0.4 修复路线（已完成）+ [FIX_PATCH.md](FIX_PATCH.md) v1+v2 patch（已 commit）+ [CLAUDE.md](../CLAUDE.md) 编码铁律

---

## 0. 导读

### 这份文档是什么

继 [FIX_PLAN.md](FIX_PLAN.md)（v0.3→v0.4 修复）之后的**下一段路线图**：从当前 v0.4 → v1.0-alpha 的详细 work-breakdown。**31 个 task / 6 周 / 4 个里程碑**。

### 给谁看

- **维护者**：决定每周做什么 + 防止 scope creep 的依据
- **贡献者**：每个 PR 对应一个 task 编号（R1-R31）
- **用户**：理解项目下一阶段会交付什么 + 不会交付什么

### 怎么用

1. 先读 §1-§3（约 10 分钟）理解项目当前位置
2. 读 §6 看总策略，§7-§10 看 4 个里程碑细节
3. 为 31 个 task 在 GitHub 开 31 个 issue，label `R-N`
4. 按 §11 节奏推进；每周末跑 [self-audit.sh](../scripts/self-audit.sh) 核对

---

# Part I — 现状审计

## 1. 一句话总评

**v0.4 已落地（5887 行 Rust / 70 个测试 / 13 处隐性回退已修 / 4 端聚焦 / self-audit 防回归）。距 v1.0-alpha 还差 6 周聚焦工作，主要 gap 在：① chat_stream 流式工具循环、② 4 端 binding 真验证、③ MCP server + OAuth 完整性、④ 真 LLM e2e 测试。**

## 2. 完成度矩阵（按 Option B 4 端目标重算）

| 能力桶 | 完成度 | 阻塞点 | 到 100% 估时 |
|---|---|---|---|
| **核心引擎（ReAct sync）** | 85% | unwrap 在 [curator.rs:94-102](../agent-core/src/memory/curator.rs#L94) 等 3 处可移除 | 0.5d |
| **流式 ReAct（chat_stream）** | 30% | [agent.rs:217-225](../agent-core/src/agent.rs#L217) 仍是单次调用 + 工具循环未做 | 2d |
| **LLM provider** | 50% | OpenAI 完整；Anthropic 流式实现；Ollama 只 3 行；无 prompt caching；无 codex_responses | 4d |
| **Tools（14 个）** | 75% | 9 真用 / 3 为冻结桩（已不注册）/ ExecuteCode 默认 host | 1d |
| **Memory L1-L4** | 80% | profile_home 全接线 / FTS5 真 MATCH / SQLite session_id 接线缺 | 1d |
| **MCP** | 35% | stdio client 完整；HTTP / Server / OAuth 全无 | 9d |
| **Compression** | 70% | maybe_compress 真接入 loop；token 估算粗（`len/2`）| 1d |
| **Skills + Curator + Review** | 75% | Curator spawn_blocking 已修；Review skill 命名仍粗 | 1d |
| **Profile** | 80% | 3 处 helper（memory_tool/skills_tool/agent.rs review）已接 profile_home | 0.5d |
| **Security** | 80% | SSRF / Secret / SecurePath / SecretString 都已修；ExecuteCode 默认未切 Docker | 1d |
| **Linux 端** | 70% | `cargo install --path agent-bindings --features cli` 直接可用；缺 `cargo install aether` 发布 + apt/pacman 包 | 2d |
| **macOS 端** | 25% | 没 macOS CI job；universal `.dylib` 脚本未写；macOS 真机/CI 无产物 | 4d |
| **Windows 端** | 20% | **无 windows-latest CI job**；无 `Aether.Tests` 项目；`*mut c_char` 释放路径未做 | 7d |
| **Android 端** | 30% | sdks/android/build.gradle.kts 存在但 jniLibs 不在仓；UniFFI Kotlin 生成路径未走通；emulator e2e 缺 | 10d |
| **测试覆盖** | 65% | **70 个测试**（35 integration + 11 hermes_compat + 24 src 内）；缺真 LLM e2e + 流式 e2e | 3d |
| **CI** | 45% | Linux build + `cargo test -p agent-core --lib` 跑通；windows/macos/android emulator/python wheel 缺 | 4d |
| **文档** | 85% | README × 2 + FIX_PLAN + FIX_PATCH + CLAUDE.md + devlog 一致；缺 CHANGELOG.md + 公开 API docs | 1d |
| **安全审计** | 70% | 主要洞已修；ExecuteCode 默认 host + secure_path canonicalize fallback 边界仍敞 | 2d |

### 加权总分

加权公式：核心引擎 25% + LLM 15% + Tools+Memory 15% + MCP 10% + 4 端 20% + 测试 CI 10% + 文档+安全 5%

**当前 = 53%**（patch 前 22%；v1+v2 patch 后 42%；Option B freeze 后 53%——分母从 6 端缩到 4 端 + 隐性回退清零）

## 3. 核心能力健康表（按真实代码读出）

| 能力 | 状态 | 关键证据 |
|---|---|---|
| ReAct loop (sync) | ✅ 健康 | [loop_mod.rs:59-207](../agent-core/src/loop_mod.rs#L59) tokio::spawn + join_all 并行；breaker hash 真接；compression 真消费 |
| ReAct loop (stream) | 🟠 缺失 | [agent.rs:217-225](../agent-core/src/agent.rs#L217) chat_stream 仍单次调用 |
| OpenAI streaming + tool_calls | ✅ 真聚合 | [openai.rs:282-389](../agent-core/src/llm/openai.rs#L282) `pending_calls: HashMap<u32, _>` 按 index 累积 |
| Anthropic streaming | ✅ 真实现 | [anthropic.rs:257+](../agent-core/src/llm/anthropic.rs#L257) 真发 SSE 请求；之前 UnsupportedApiMode 已删 |
| Ollama provider | 🟠 stub | 仅 3 行；需复用 OpenAI 兼容端点 |
| MCP stdio client | ✅ 真实 | [mcp/mod.rs:9-348](../agent-core/src/mcp/mod.rs) `McpStdioServer` + `AtomicU64` + `oneshot::Sender` 派发 |
| MCP HTTP / Server / OAuth | ❌ 全无 | 仓库内无对应模块 |
| Delegate sub-agent | ✅ 真接 | [delegate.rs:30-104](../agent-core/src/delegate.rs#L30) `run_subagent` 真调 `registry.execute()` |
| Compression | ✅ 真接 | [loop_mod.rs:183](../agent-core/src/loop_mod.rs#L183) `messages = result.compressed_messages` |
| Background Review | ✅ 真 spawn | [review.rs](../agent-core/src/memory/review.rs) 通过 self-audit grep MCP `McpStdioServer` 类似验证 |
| Curator | ✅ 异步化 | [agent.rs:195-210](../agent-core/src/agent.rs#L195) `spawn_blocking` |
| Profile 隔离 | 🟡 部分 | 3 处工具接线 OK；CLI `--profile` 切换需端到端测试 |
| Secret 脱敏 | ✅ 真实 | [config.rs:32](../agent-core/src/config.rs#L32) `SecretString` + [memory_tool.rs:36-70](../agent-core/src/tools/memory_tool.rs#L36) 8 类前缀 + PEM |
| SSRF | ✅ 真 DNS | [web_tools.rs:5-106](../agent-core/src/tools/web_tools.rs#L5) `url::Url::parse` + `ToSocketAddrs` |
| secure_path | 🟡 OK | [file_tools.rs:7-28](../agent-core/src/tools/file_tools.rs#L7) canonicalize；fallback corner case 见 §5 |
| FTS5 search | ✅ 真 MATCH | [state.rs:152-181](../agent-core/src/memory/state.rs#L152) `messages_fts MATCH ?1 ORDER BY bm25` |

## 4. 4 端就绪度对照

| 端 | 当前状态 | hello world 命令 | 阻塞点 |
|---|---|---|---|
| **Linux** | 🟢 80% | `cargo build --release && ./target/release/aether -h` | 缺 `cargo install aether` 发布到 crates.io；缺 apt/pacman 包 |
| **macOS** | 🟡 40% | `cargo build --release --target aarch64-apple-darwin` | 缺 macos-latest CI；缺 universal `.dylib` lipo 脚本；example/macos-demo 缺 |
| **Windows** | 🟠 25% | 理论：`cargo build --target x86_64-pc-windows-msvc` | **CI 完全没 windows-latest job**；`Aether.Tests` 项目不存在；`*mut c_char` 释放路径无 `aether_free_string` 函数；UTF-16 路径 corner case 未测 |
| **Android** | 🟠 30% | 理论：`cargo ndk -t arm64-v8a build && cd sdks/android && ./gradlew assemble` | jniLibs 不在仓；UniFFI Kotlin binding 生成未自动化；android.yml 仅 build 不 emulator 测；examples/android-demo 状态未确认能跑 |

## 5. 隐藏债务清单（self-audit 未覆盖部分）

按严重度排序：

| ID | 债务 | 严重度 | file:line | 修复建议 |
|---|---|---|---|---|
| D1 | Rust `*mut c_char` → C# 内存释放路径未建 | H | sdks/dotnet/Aether/Aether.cs + agent-bindings 缺 `aether_free_string` 导出 | 见 R-W3 |
| D2 | secure_path canonicalize 失败时 fallback 到 absolute path | M | [file_tools.rs:21-22](../agent-core/src/tools/file_tools.rs#L21) | 改为 Err 而非 fallback；新文件场景另走 parent canonicalize |
| D3 | ExecuteCode 默认 backend = "host"（裸跑） | M | [terminal_backends.rs:118](../agent-core/src/tools/terminal_backends.rs#L118) | 见 R-T6 |
| D4 | 24 个 unwrap 在 src/（3 个非测试位置：curator.rs `file_name().unwrap()` × 3） | L | [curator.rs:94-102](../agent-core/src/memory/curator.rs#L94) | 改为 `unwrap_or_else` |
| D5 | 5-7 个 config 字段仍未读 | L | [config.rs](../agent-core/src/config.rs) `enabled_toolsets`/`disabled_toolsets`/`session_id` 等 | 见 R-C1 |
| D6 | Curator 未使用 import（v1 引入但未消费） | L | [curator.rs](../agent-core/src/memory/curator.rs) `tokio::sync::Mutex` / `Arc` | 删除或加 `#[allow(unused_imports)]` |
| D7 | tracing 配置存在但 log_level config 字段无关联 | M | tracing.rs vs config.rs | 见 R-C1 |
| D8 | examples/ 仅 android-demo；macOS/Windows/Linux 无 demo | M | examples/ | 见 R-X1~X3 |
| D9 | CHANGELOG.md 不存在；用 docs/devlog.md 代偿 | L | 根目录 | 见 R-D1 |
| D10 | crates.io 包名占位风险 | M | Cargo.toml `aether-agent-core` 已存在于 crates.io，需查证 | 见 R-D2 |
| D11 | `Aether.Tests` C# 项目完全不存在 | M | sdks/dotnet/ | 见 R-W4 |
| D12 | `cargo doc` 公开 API doc 缺失（无 `///`）| L | agent-core/src 公开 API | 见 R-D3 |

---

# Part II — v1.0 路线图

## 6. 总策略与里程碑

### 4 个里程碑

| 里程碑 | 主题 | 任务数 | 估时 | 目标产出 |
|---|---|---|---|---|
| **M1** | P0 核心补完 | 8 | 9d | v0.5-beta — 核心引擎完整 |
| **M2** | 4 端 binding | 12 | 18d | v0.7-beta — 4 端真验证 |
| **M3** | P2 功能补完 | 6 | 8d | v0.9-rc — 功能对标 |
| **M4** | release 收尾 | 5 | 5d | v1.0-alpha — 可发布 |

**31 个 task / 40 人日 / 单人 6-8 周**（按 8h/d 计算 / 含 30% buffer）

### 强制顺序

- **M1 必须先于 M2** —— chat_stream 不修，4 端流式都用不了
- **M2 内 Linux → macOS → Windows → Android** —— 难度递增，先攻易守的端
- **M3 可与 M2 并行**（不同人或同人切换 context）
- **M4 不能跳** —— 没发布脚本 = 没 v1.0

## 7. M1 — P0 核心补完（9 人日 / 第 1-2 周）

### R-1.1  chat_stream 真跑 ReAct 循环 [P0 / 2d / 高难度 / 无依赖]

**问题**：[agent.rs:217-225](../agent-core/src/agent.rs#L217) `chat_stream` 是单次 LLM 调用，工具调用增量被丢弃。

**改法**：引入 `StreamEvent` enum，在 chat_stream 内做循环：

```rust
pub enum StreamEvent {
    Text(String),
    ToolCall(ToolCall),
    ToolResult(String, String),
    Done(String),
    Error(AetherError),
}

pub fn chat_stream(&self, msg: &str) -> impl Stream<Item = StreamEvent> {
    // loop: stream LLM → accumulate text + tool_calls
    // → if tool_calls empty: Done
    // → else: dispatch tools → push observations → next iteration
}
```

**Acceptance**:
- [ ] [agent.rs:217+](../agent-core/src/agent.rs#L217) chat_stream 返回 `impl Stream<Item = StreamEvent>`
- [ ] mock LLM 测试：第 1 轮返回 tool_call → 流出 `ToolCall` → 派发 → 流出 `ToolResult` → 第 2 轮返回 text → 流出 `Text` → `Done`
- [ ] OpenAI + Anthropic 两个 provider 都通
- [ ] self-audit.sh 阶段 5 增加 `grep "StreamEvent::Done" agent-core/src/`

**风险 H**：StreamEvent 是新公开 enum → minor breaking change，要写 CHANGELOG。

---

### R-1.2  Ollama provider 真实现 [P0 / 1d / 低难度 / 无依赖]

**问题**：[ollama.rs](../agent-core/src/llm/ollama.rs) 仅 3 行。

**改法**：Ollama 支持 OpenAI 兼容端点 `/v1/chat/completions`。最简：复用 `OpenAiProvider` 改 base_url：

```rust
pub fn ollama(model: &str, base_url: Option<String>) -> OpenAiProvider {
    OpenAiProvider::new(
        "ollama-local",
        model,
        base_url.unwrap_or_else(|| "http://localhost:11434/v1".into()),
        None,  // no api key
    )
}
```

**Acceptance**:
- [ ] [ollama.rs](../agent-core/src/llm/ollama.rs) 行数 >50
- [ ] integration test：`Provider::ollama("llama3.2")` 能构造
- [ ] devlog 加一条 "Ollama via OpenAI-compat endpoint"

**风险 L**

---

### R-1.3  MCP HTTP transport [P0 / 2d / 中难度 / 依赖 R-1.1]

**问题**：[mcp/mod.rs](../agent-core/src/mcp/mod.rs) 只有 stdio；HTTP / Streamable HTTP 缺。

**改法**：新增 `mcp/http.rs`，实现 MCP Streamable HTTP (spec 2025-03-26)：
- POST JSON-RPC → 同步 response or SSE stream
- Session-Id header 管理
- 重连 + retry

**Acceptance**:
- [ ] `pub struct McpHttpServer` 与 `McpStdioServer` 镜像接口
- [ ] integration test：mock HTTP MCP server，list/call 一遍
- [ ] self-audit 阶段 5 加 `grep "pub struct McpHttpServer"`

**风险 M**：协议规范在演进，写完可能要追小调整

---

### R-1.4  Config 字段接线（5-7 个未读字段）[P0 / 1d / 低难度 / 无依赖]

**问题**：[config.rs](../agent-core/src/config.rs) `enabled_toolsets` / `disabled_toolsets` / `session_id` / `log_level` 等仍未读。

**改法**（逐个接线或删除）：
| 字段 | 接线点 |
|---|---|
| `enabled_toolsets`/`disabled_toolsets` | `agent.rs:60-79` 注册前 filter |
| `session_id` | `SessionStore::resume_or_create(session_id)` |
| `log_level` | `tracing.rs` 启动时读 |
| `delegation_enabled` | `agent.rs:107-126` delegate 注册 gate |

**Acceptance**:
- [ ] `cargo clippy -- -D dead_code` 通过
- [ ] 每个保留字段加一个 unit test 证明被消费
- [ ] config.rs 字段数 - dead_code 数 = 字段数

**风险 L**

---

### R-1.5  Anthropic prompt caching [P0 / 1d / 低难度 / 无依赖]

**改法**：[anthropic.rs](../agent-core/src/llm/anthropic.rs) 在 system / tools / 选中 messages 加 `cache_control: { type: "ephemeral" }`。**关键**：stable prompt 不能含 `Local::now()`（[prompt.rs](../agent-core/src/prompt.rs) 三层结构应已支持）。

**Acceptance**:
- [ ] 发请求 body 含 `cache_control: { type: "ephemeral" }`
- [ ] 真实 Anthropic API 调用后，第 2 次响应 `cache_read_input_tokens > 0`（需手动验证）

**风险 L**

---

### R-1.6  ExecuteCode 默认 backend → Docker [P0 / 0.5d / 低难度 / 无依赖]

**改法**：[terminal_backends.rs:118](../agent-core/src/tools/terminal_backends.rs#L118) 默认改 docker；检测 daemon 不可用时 fallback host + 警告日志。

**Acceptance**:
- [ ] `ExecuteCode` schema description 写 "default: docker; requires Docker daemon"
- [ ] Docker 不可用时返回 stderr 警告

**风险 L**

---

### R-1.7  3 个 unwrap → unwrap_or_else（D4）[P0 / 0.5d / 低难度 / 无依赖]

**问题**：[curator.rs:94-102](../agent-core/src/memory/curator.rs#L94) `path.file_name().unwrap()` × 3。

**改法**：改 `unwrap_or_else(|| OsStr::new(""))` + log warning。

**Acceptance**:
- [ ] `grep -n "file_name().unwrap()" agent-core/src/memory/curator.rs` 无结果

**风险 L**

---

### R-1.8  Curator 未用 import 清理（D6）[P0 / 5min / 低难度 / 无依赖]

**改法**：删除 `tokio::sync::Mutex` / `Arc` import。

**Acceptance**:
- [ ] `cargo build --workspace` 无 `unused_imports` warning

**风险 L**

---

### M1 验收（v0.5-beta）

- [ ] 全部 8 个 R-1.x task ✅
- [ ] `cargo test --workspace` 通过测试数从 70 → 80+
- [ ] `bash scripts/self-audit.sh` 全部 ✅
- [ ] README "✅ Functional" 数从 X 增加 3-4 项（chat_stream / Ollama / MCP HTTP / Anthropic caching）

---

## 8. M2 — 4 端 binding 真验证（18 人日 / 第 3-5 周）

### 顺序：Linux → macOS → Windows → Android

### R-L1  Linux: cargo install 发布准备 [P1 / 1d / 低难度 / 无依赖]

**改法**：
- agent-bindings/Cargo.toml `[[bin]] name = "aether"` 已存在；确认 `cli` feature 配置
- 加 README quickstart `cargo install --path agent-bindings --features cli`
- 写 docs/installation/linux.md

**Acceptance**:
- [ ] `cargo install --path agent-bindings --features cli` 在 ubuntu-22.04 fresh 容器跑通
- [ ] `aether --help` 输出 CLI 帮助

---

### R-L2  Linux: apt/snap 包脚本（可选）[P3 / 1d / 低难度 / 依赖 R-D2]

**改法**：用 `cargo-deb` 生成 `.deb`；用 `cargo-aur` 生成 PKGBUILD。

**Acceptance**:
- [ ] CI 产出 `aether_0.5.0-1_amd64.deb` artifact

---

### R-M1  macOS: universal .dylib 构建脚本 [P1 / 1.5d / 中难度 / 无依赖]

**改法**：写 `scripts/build-macos.sh`：

```bash
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create \
    target/aarch64-apple-darwin/release/libagent_bindings.dylib \
    target/x86_64-apple-darwin/release/libagent_bindings.dylib \
    -output target/universal/libaether.dylib
```

**Acceptance**:
- [ ] `bash scripts/build-macos.sh` 产出 `target/universal/libaether.dylib`
- [ ] `file libaether.dylib` 显示 universal binary

**风险 L**

---

### R-M2  macOS CI job [P1 / 1d / 中难度 / 依赖 R-M1]

**改法**：`.github/workflows/macos.yml` 跑 macos-latest runner + 上述 build 脚本。

**Acceptance**:
- [ ] `.github/workflows/macos.yml` 存在且 PR 触发
- [ ] artifact `libaether-universal.dylib` 可下载

**风险 M**：macos runner 贵 10x，CI 时间成本

---

### R-M3  examples/macos-demo（最小化） [P2 / 0.5d / 低难度 / 依赖 R-M1]

**改法**：写一个最小化 Swift CLI 或 Objective-C 程序 link `libaether.dylib`，调用 `aether_init` + `aether_chat` 一次。

**Acceptance**:
- [ ] `examples/macos-demo/` 包含 `main.swift` + `build.sh`
- [ ] `bash examples/macos-demo/build.sh && ./macos-demo` 输出 LLM 回复

---

### R-W1  Windows: windows-latest CI job [P1 / 1.5d / 中难度 / 无依赖]

**问题**：[.github/workflows/ci.yml](../.github/workflows/ci.yml) 仅 ubuntu。

**改法**：matrix 加 windows-latest，跑 `cargo build --workspace` + `cargo test --workspace`。

**Acceptance**:
- [ ] CI matrix 包含 windows-latest
- [ ] Windows job 通过 build + test

**风险 H**：第一次开会爆隐性问题（path 分隔符 / portable-pty ConPTY / dotnet 检测）

---

### R-W2  Windows: portable-pty ConPTY 修复 [P1 / 1d / 中难度 / 依赖 R-W1]

**改法**：[terminal_tool.rs](../agent-core/src/tools/terminal_tool.rs) 已 `cfg!(windows)` 派发；确认 `portable-pty` 在 windows-msvc 编译过。如失败，要 vendor patch 或换 `windows-pty`。

**Acceptance**:
- [ ] Windows CI 跑 `Terminal::call({"command": "echo hello"})` 测试通过

---

### R-W3  Windows: Rust → C# 内存释放（D1）[P1 / 1.5d / 高难度 / 依赖 R-W1]

**问题**：[Aether.cs](../sdks/dotnet/Aether/Aether.cs) 调 `aether_chat` 返回 `IntPtr`，但**没有 `aether_free_string` 调用**——每次调用泄漏。

**改法**：
1. agent-bindings/src/lib.rs 加 export：

```rust
#[no_mangle]
pub extern "C" fn aether_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { let _ = CString::from_raw(ptr); }
    }
}
```

2. Aether.cs 改 `PtrToUtf8` 后立刻调 `aether_free_string`：

```csharp
private static string PtrToUtf8AndFree(IntPtr p) {
    if (p == IntPtr.Zero) return null;
    var s = PtrToUtf8(p);
    aether_free_string(p);
    return s;
}
```

**Acceptance**:
- [ ] agent-bindings exports `aether_free_string`
- [ ] Aether.cs 所有 PtrToUtf8 调用后释放
- [ ] valgrind / dotnet-counters 显示 1000 次 chat 调用后内存稳定

**风险 H**：内存安全 critical；测试需要长跑

---

### R-W4  Windows: Aether.Tests C# 项目（D11）[P1 / 1d / 中难度 / 依赖 R-W3]

**改法**：新建 `sdks/dotnet/Aether.Tests/Aether.Tests.csproj`（NUnit / xUnit），加 5 个测试：构造 / chat / 中文 roundtrip / 释放 / 大文本。

**Acceptance**:
- [ ] `dotnet test sdks/dotnet/Aether.Tests/` 通过
- [ ] Windows CI 跑这些测试

---

### R-A1  Android: cargo-ndk 构建 + jniLibs [P1 / 2d / 中难度 / 无依赖]

**改法**：
- 写 `scripts/build-android.sh`：
  ```bash
  cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
      -o sdks/android/src/main/jniLibs \
      build --release -p agent-bindings
  ```
- 决策：`.so` 入仓 OR build 时产出
  - 入仓优点：用户 clone 即用，体积 ~15MB
  - 不入仓优点：仓库小，但需要本地 setup NDK
  - **建议**：入仓 `.so`（git-lfs 可选），README 说"如需重建跑 build-android.sh"

**Acceptance**:
- [ ] `bash scripts/build-android.sh` 在 ubuntu 跑通
- [ ] `sdks/android/src/main/jniLibs/arm64-v8a/libuniffi_agent.so` 存在（或 CI 产出）

**风险 M**：NDK × cargo-ndk × UniFFI 版本矩阵

---

### R-A2  Android: UniFFI Kotlin binding 生成 [P1 / 2d / 中难度 / 依赖 R-A1]

**改法**：
- 确认 agent-bindings 用 UniFFI procedural macro 还是 UDL
- 写 `scripts/gen-kotlin-binding.sh`：
  ```bash
  cargo run --features uniffi-bindgen --bin uniffi-bindgen generate \
      --library target/debug/libagent_bindings.so \
      --language kotlin \
      --out-dir sdks/android/src/main/kotlin
  ```
- AAR build：`cd sdks/android && ./gradlew assembleRelease`

**Acceptance**:
- [ ] `sdks/android/src/main/kotlin/uniffi/agent/agent.kt` 自动生成
- [ ] `./gradlew assembleRelease` 产出 `aether-release.aar`

**风险 M**

---

### R-A3  Android: emulator e2e CI [P1 / 2d / 中难度 / 依赖 R-A2]

**改法**：[.github/workflows/android.yml](../.github/workflows/android.yml) 已有 build job；新增 emulator job 用 `reactivecircus/android-emulator-runner@v2`。

**Acceptance**:
- [ ] emulator job 在 PR 触发（main branch only 可接受）
- [ ] examples/android-demo `./gradlew connectedAndroidTest` 通过

**风险 H**：emulator runner 在 ubuntu-latest 不稳；macos-latest runner 贵；总成本要权衡

---

### R-A4  examples/android-demo 真能跑 [P2 / 1d / 低难度 / 依赖 R-A2]

**改法**：检查 [examples/android-demo/](../examples/android-demo/) 现状；如果 broken 修，如果空白写最小 Activity 调 `Aether(provider="openai", ...).chat("hi")`。

**Acceptance**:
- [ ] `cd examples/android-demo && ./gradlew installDebug` 在真机或 emulator 跑通

---

### M2 验收（v0.7-beta）

- [ ] 4 端 CI 全绿（Linux + macOS + Windows + Android emulator）
- [ ] 4 端 hello world artifact 可下载（cli binary / dylib / dll+exe / AAR）
- [ ] README "Verified" 标签首次有 CI badge 链接
- [ ] self-audit.sh 加 4 条新检查：`grep "windows-latest" .github/workflows/ci.yml` 等

---

## 9. M3 — P2 功能补完（8 人日 / 第 6 周）

### R-3.1  MCP server 实现 [P2 / 3d / 中难度 / 依赖 R-1.3]

**改法**：新增 `mcp/server.rs`。镜像 stdio client：作为 stdio server 接受 host 调用，暴露 Aether 内置工具。

**架构决策**：
- 模式 A：`aether mcp-server` 子命令 binary
- 模式 B：library API 让 SDK 用户自己 spawn server
- **推荐 A**——给 Claude Desktop / Cursor 等 host 用

**Acceptance**:
- [ ] `aether mcp-server` 启动后能响应 `initialize` + `tools/list`
- [ ] host 调用 `tools/call` 真执行 Aether 工具

---

### R-3.2  MCP OAuth [P2 / 2d / 中难度 / 依赖 R-1.3]

**改法**：实现 OAuth 2.1 + PKCE。token 存储分平台：
- Linux/macOS：`keyring` crate（Secret Service / Keychain）
- Windows：`keyring` crate（Credential Manager）
- Android：开放 hook 让 host app 注入 Keystore

**Acceptance**:
- [ ] `McpHttpServer::with_oauth(...)` 接口
- [ ] integration test：mock OAuth server，PKCE 流程跑通

**风险 H**：架构分层活，停下来想 1d 再开写

---

### R-3.3  codex_responses (OpenAI Responses API) [P2 / 2d / 中难度 / 无依赖]

**改法**：新增 `llm/codex.rs` 或扩展 `openai.rs`。Responses API 是 stateful（`previous_response_id`）。

**Acceptance**:
- [ ] `Provider::codex(...)` 构造方式
- [ ] integration test：链式调用 3 次，`previous_response_id` 真衔接

**风险 M**：可能暴露 ChatModel trait 设计是否够灵活

---

### R-3.4  Skill 命名改进（Background Review 不再 auto-learned-skill） [P2 / 0.5d / 低难度 / 无依赖]

**改法**：[review.rs](../agent-core/src/memory/review.rs) 生成 skill 时用 `review-{YYYYMMDD}-{slug}`。

**Acceptance**:
- [ ] 多次 review 不产生同名 skill
- [ ] 测试：跑两次 review，文件名不同

---

### R-3.5  Compression token 估算改 CJK 友好 [P2 / 0.5d / 低难度 / 无依赖]

**改法**：[compression/mod.rs](../agent-core/src/compression/mod.rs) `text.len() / 2` → CJK 字符 1 token + ASCII 4字符 1 token，或集成 `tiktoken-rs`。

**Acceptance**:
- [ ] 单测：1000 个中文字符估算 ~1000 token（不是 ~500）

---

### R-3.6  secure_path Err（D2）[P2 / 0.5d / 低难度 / 无依赖]

**改法**：[file_tools.rs:21-22](../agent-core/src/tools/file_tools.rs#L21) canonicalize 失败时不 fallback，对新文件场景走 parent canonicalize：

```rust
let canonical = match std::fs::canonicalize(&absolute) {
    Ok(p) => p,
    Err(_) => {
        let parent = absolute.parent().ok_or(...)?;
        std::fs::canonicalize(parent)?.join(absolute.file_name().unwrap())
    }
};
```

**Acceptance**:
- [ ] 写新文件 `safe.txt` → 通过
- [ ] 写 symlink `evil.txt → /etc/passwd` → 被拒

---

### M3 验收（v0.9-rc）

- [ ] MCP 完整（client stdio+HTTP / server / OAuth）
- [ ] 3 API mode 全实现（chat_completions / anthropic_messages / codex_responses）
- [ ] 自审 5 阶段加新检查项

---

## 10. M4 — release 收尾（5 人日 / 第 7 周）

### R-D1  CHANGELOG.md（D9）[P1 / 0.5d / 低难度 / 无依赖]

**改法**：根目录 CHANGELOG.md，关联 `docs/changes/` 编号归档（如有）；遵循 Keep a Changelog 格式。

```markdown
## [0.5.0-beta] — 2026-XX-XX

### Added
- chat_stream now runs full ReAct loop (R-1.1)
- Ollama provider via OpenAI-compat (R-1.2)
- MCP HTTP transport (R-1.3)
- Anthropic prompt caching (R-1.5)

### Breaking
- `StreamEvent` enum exposed (R-1.1)

### Fixed
- 3 unwrap() in curator.rs (R-1.7)
```

**Acceptance**: ✅ CHANGELOG.md 存在，包含 v0.5/v0.7/v0.9/v1.0 4 个条目

---

### R-D2  crates.io publish prep（D10）[P1 / 1d / 中难度 / 依赖 R-D1]

**改法**：
- 检查 `aether-agent-core` / `aether` / `agent-core` 在 crates.io 占用
- 选定最终包名（可能要换）
- 写 `cargo publish --dry-run` 验证
- README 加发布 badge 占位

**Acceptance**:
- [ ] `cargo publish --dry-run` 通过
- [ ] crates.io 包名锁定

**风险 M**：名字可能被占

---

### R-D3  cargo doc 公开 API doc [P2 / 1d / 低难度 / 无依赖]

**改法**：[agent-core/src/lib.rs](../agent-core/src/lib.rs) 顶部 `//! ...` crate-level doc；公开 trait / struct 加 `///` 文档注释。

**Acceptance**:
- [ ] `cargo doc --no-deps --open` 显示完整 API 文档
- [ ] `#![deny(missing_docs)]` 在 lib.rs 加上不报错

---

### R-T1  真 LLM round-trip 集成测试 [P1 / 2d / 中难度 / 无依赖]

**改法**：写 mock LLM server（基于 `wiremock-rs`），覆盖：
- chat happy path（OpenAI/Anthropic）
- chat with tool calls
- stream + tool calls
- compression triggered
- error recovery

GitHub Actions secrets：把测试 API key 配进 main branch only job。

**Acceptance**:
- [ ] `tests/e2e_llm.rs` 新文件含 ≥10 个测试
- [ ] CI main branch job 跑通

**风险 M**：mock 跟真 API 漂移

---

### R-T2  self-audit.sh 新增 v1.0 检查 [P1 / 0.5d / 低难度 / 依赖 全部 R-*.*]

**改法**：[scripts/self-audit.sh](../scripts/self-audit.sh) 加 v1.0 检查：
- `grep "StreamEvent" agent-core/src/agent.rs`
- `grep "pub struct McpHttpServer" agent-core/src/mcp/`
- `grep "aether_free_string" agent-bindings/src/`
- `grep "windows-latest" .github/workflows/ci.yml`
- `cargo doc --no-deps` 成功
- `cargo publish --dry-run` 成功

**Acceptance**:
- [ ] self-audit.sh 阶段总数 7 → 8
- [ ] 所有阶段在 v1.0 仓库 ✅

---

### M4 验收（v1.0-alpha）

- [ ] CHANGELOG.md 完整 4 版本条目
- [ ] crates.io 名字锁定 + dry-run 通过
- [ ] `cargo doc --no-deps --open` 完整
- [ ] 真 LLM e2e 10 个测试通过
- [ ] self-audit.sh 全 ✅
- [ ] git tag `v1.0.0-alpha`
- [ ] GitHub Release 含 artifacts（cli binary / aar / dylib / dll）

---

# Part III — 执行指南

## 11. 6 周节奏表

| 周 | 重点 | 完成的 task | 周末里程碑 |
|---|---|---|---|
| **W1** | M1 前半 | R-1.1 chat_stream + R-1.2 Ollama + R-1.5 caching + R-1.6 docker + R-1.7 unwrap + R-1.8 cleanup | M1 6/8 |
| **W2** | M1 收尾 + M2 启动 | R-1.3 MCP HTTP + R-1.4 config + R-L1 Linux + R-M1 macOS .dylib | M1 ✅ / **v0.5-beta** |
| **W3** | M2 macOS + Windows 启动 | R-M2 macOS CI + R-M3 demo + R-W1 Windows CI + R-W2 ConPTY | M2 4/12 |
| **W4** | M2 Windows + Android 启动 | R-W3 内存释放 + R-W4 Aether.Tests + R-A1 cargo-ndk + R-A2 UniFFI | M2 8/12 |
| **W5** | M2 Android 收尾 + M3 并行 | R-A3 emulator CI + R-A4 demo + R-3.1 MCP server + R-3.5 token估算 + R-3.6 secure_path | M2 ✅ / M3 3/6 / **v0.7-beta** |
| **W6** | M3 收尾 + M4 | R-3.2 OAuth + R-3.3 codex + R-3.4 skill 命名 + R-D1 CHANGELOG + R-D2 crates + R-D3 docs + R-T1 e2e + R-T2 self-audit | M3 ✅ / M4 ✅ / **v1.0-alpha** |

**关键观察**：W6 任务密集（8 个），需要前几周 buffer 一些时间。

## 12. 关键依赖图

```
R-1.1 chat_stream ──┬─→ R-1.3 MCP HTTP ──→ R-3.1 MCP server
                    │                    └─→ R-3.2 OAuth
                    │
                    └─→ M2 4 端任务（streaming SDK API）

R-W1 Windows CI ──┬─→ R-W2 ConPTY
                  ├─→ R-W3 内存释放 ──→ R-W4 Aether.Tests
                  └─→ R-W5 CI 集成

R-A1 cargo-ndk ──→ R-A2 UniFFI Kotlin ──┬─→ R-A3 emulator CI
                                        └─→ R-A4 demo
```

**串行瓶颈**：M1 内 R-1.1 是后续 4 端流式的前置；M2 内 Windows 链路（R-W1→W2→W3→W4）必须串行。

## 13. 风险登记表

| ID | 风险 | 概率 | 影响 | 触发条件 | 缓解 |
|---|---|---|---|---|---|
| RK1 | R-1.1 StreamEvent 设计返工 | M | H | mock 不出真 SSE 时序 | W1 预算 1d buffer；先写 OpenAI 单 provider 验证 |
| RK2 | R-W1 Windows CI 第一次开爆 10+ 问题 | H | M | 没人在 Windows 上跑过 | W3 预留 1.5d buffer；先在本地 Windows VM 跑过 |
| RK3 | R-W3 内存释放 valgrind/memory profiler 难配 | M | H | 跨进程 .NET → Rust 内存追踪复杂 | 找有 .NET interop 经验的人 review；或用 dotnet-counters 做 1000-call 稳定性测试 |
| RK4 | R-A1 NDK 版本不兼容 | M | M | NDK r26 / r27 + cargo-ndk + UniFFI 3 维矩阵 | 锁版本 NDK r26d；rust-toolchain.toml 固定 1.94.0 |
| RK5 | R-A3 GitHub Actions emulator runner 不稳 | H | M | ubuntu-latest 启动 emulator 卡住 / timeout | 用 macos-latest runner（贵但稳）；emulator test 只 main branch 跑 |
| RK6 | R-3.2 MCP OAuth 跨平台 token 存储 | M | M | keyring crate Linux DBus 依赖 | Linux fallback file + chmod 600；文档说明 |
| RK7 | R-D2 crates.io 名字被占 | M | L | `aether-agent-core` 可能已注册 | W6 之前先查清楚；备选 `aether` / `aether-sdk` |
| RK8 | R-1.3 MCP Streamable HTTP spec 变动 | L | M | 协议规范在演进 | 实现时引 spec 版本号；测试用固定 mock |
| RK9 | macOS CI runner 时间预算超 | M | M | macos-latest 比 ubuntu 贵 10x | 仅 main branch + release tag 跑；feature PR 跳过 |
| RK10 | 单人 6 周时间不够 | M | H | task 密度高 + 隐性问题 | W3 末做中期 review，看是否要砍 M3 部分到 v1.1 |

## 14. acceptance 定义（什么算 v1.0-alpha ready）

**必须**：
1. ✅ `bash scripts/self-audit.sh` 全部 ✅
2. ✅ `cargo build --workspace --release` 在 Linux / macOS / Windows 三个平台通过（CI 验证）
3. ✅ `cargo test --workspace` ≥ 100 个测试全部通过
4. ✅ Android emulator CI job 通过 + AAR artifact 产出
5. ✅ macOS universal `.dylib` artifact 产出
6. ✅ Windows `.dll` artifact + `dotnet test` 通过 + 中文 roundtrip 通过
7. ✅ Linux `cargo install` 在 fresh Ubuntu 容器跑通
8. ✅ `cargo doc --no-deps` 无 warning
9. ✅ `cargo publish --dry-run` 通过
10. ✅ CHANGELOG.md 含 v0.5/v0.7/v0.9/v1.0 4 个完整 entry
11. ✅ README 不含任何"✅ Complete"对未真完成功能的声明
12. ✅ MCP client (stdio + HTTP) + MCP server + OAuth 全实现
13. ✅ chat_stream + tools 在所有 LLM provider 通过 e2e 测试
14. ✅ git tag `v1.0.0-alpha` + GitHub Release with artifacts

**不必须**（推迟到 v1.0-rc / v1.0 正式版）：
- 真 ExecuteCode 安全沙箱（Wasmtime/Firecracker）
- 浏览器 / iOS 端（已冻结）
- crates.io 真发布（dry-run 通过即可）
- 完整国际化（保持中英 README 同步即可）

---

# Part IV — 范围管理

## 15. v1.0 不包括什么（明确砍掉）

| 已砍 | 砍除原因 |
|---|---|
| Web / WASM | 已 FROZEN(2026-06-16)；与"端侧 SDK"定位不符；Vercel AI SDK 已是事实标准 |
| iOS Swift SDK | 已 FROZEN(2026-06-16)；Apple FM framework + ClaudeForFoundationModels 已收编 |
| 真 ExecuteCode 安全沙箱 | 跨平台真沙箱不存在；推荐用 host docker / Wasmtime 二选一让用户自配 |
| 浏览器工具（Playwright 等）| Rust 无成熟跨平台替代 |
| 18+ 消息平台适配器 | App 层 |
| Kanban / TUI / Desktop App | 非 SDK 范围 |
| ImageGenerate / TTS / STT | API wrapper，App 层做 |
| ACP 适配器（IDE 集成）| 非 SDK 范围 |

## 16. v1.1+ 路线建议

按用户反馈决定优先级：

| 候选 | 触发条件 |
|---|---|
| iOS 解冻 + Apple FM provider 适配 | iOS 用户问 ≥ 10 次 |
| Web/WASM 解冻 + HttpClient trait 抽象 | 浏览器 demo 需求 |
| Wasmtime ExecuteCode 沙箱 | 安全合规客户 |
| 真 Hermes SQLite 兼容回归测试 | Hermes 用户切换需求 |
| 第二批 LLM provider（Gemini / Bedrock）| 客户要求 |
| Browser tool（headless_chrome）| Desktop 客户要求 |

## 17. 不可逾越的约束

1. **核心代码 100% 跨 4 端** — agent-core 主路径不允许 `cfg(target_os)` 分支
2. **数据格式兼容 Hermes** — MEMORY.md / USER.md / skills/*.md / SQLite schema
3. **公开 trait 稳定** — ChatModel / Tool / Memory / SessionStore / Streamable 是 v1.0 公开 API
4. **每次 commit 前跑 self-audit.sh** — CLAUDE.md 铁律
5. **README 与代码一致** — 中英文 README 同步改
6. **任何 status 标签后必须列具体缺失项**
7. **不接受"我顺手改了别的"PR** — 每 task 独立可 revert
8. **新 feature 必须有对应测试**
9. **任何破坏 API 必须升 minor 版本 + CHANGELOG Breaking 段**
10. **iOS / Web 解冻必须经 review** — 不能自动恢复

---

# 附录

## 附录 A — 关键文件清单

agent-core/src/ 主要文件（按行数倒序）：

| 文件 | 行数 | 职责 |
|---|---|---|
| [llm/openai.rs](../agent-core/src/llm/openai.rs) | 564 | OpenAI provider + streaming + tool_calls |
| [llm/anthropic.rs](../agent-core/src/llm/anthropic.rs) | 473 | Anthropic provider + streaming |
| [mcp/mod.rs](../agent-core/src/mcp/mod.rs) | 348 | MCP stdio client + handshake |
| [tools/web_tools.rs](../agent-core/src/tools/web_tools.rs) | 331 | Web 工具 + SSRF DNS 防御 |
| [agent.rs](../agent-core/src/agent.rs) | 287 | AIAgent 构造 + 注册工具 + Curator spawn_blocking |
| [delegate.rs](../agent-core/src/delegate.rs) | 274 | 真 Sub-agent Delegation |
| [config.rs](../agent-core/src/config.rs) | 246 | SecretString api_key + 23 字段 |
| [loop_mod.rs](../agent-core/src/loop_mod.rs) | 227 | ReAct 主循环 + 工具并行 |
| [memory/state.rs](../agent-core/src/memory/state.rs) | 215 | SQLite + FTS5 MATCH |
| [tools/file_tools.rs](../agent-core/src/tools/file_tools.rs) | 192 | 文件工具 + secure_path canonicalize |
| [tools/skills_tool.rs](../agent-core/src/tools/skills_tool.rs) | 181 | skills 列表/查看/管理 |
| [tools/memory_tool.rs](../agent-core/src/tools/memory_tool.rs) | 180 | memory 工具 + 8 类 secret redact |
| [tools/terminal_backends.rs](../agent-core/src/tools/terminal_backends.rs) | 169 | ExecuteCode + docker backend |
| [types/message.rs](../agent-core/src/types/message.rs) | 160 | Message / Content / ToolCall |
| [memory/curator.rs](../agent-core/src/memory/curator.rs) | 148 | Skill 生命周期 |
| [error.rs](../agent-core/src/error.rs) | 147 | AetherError 24 变体五段分类 |
| [skills/mod.rs](../agent-core/src/skills/mod.rs) | 145 | agentskills.io frontmatter 解析 |
| [tools/terminal_tool.rs](../agent-core/src/tools/terminal_tool.rs) | 135 | Terminal 工具 + 跨平台 shell |
| [memory/review.rs](../agent-core/src/memory/review.rs) | 124 | Background Review |
| [budget.rs](../agent-core/src/budget.rs) | 114 | IterationBudget AtomicU32 |
| [compression/mod.rs](../agent-core/src/compression/mod.rs) | 113 | LLM 摘要压缩 |
| [tools/extra_tools.rs](../agent-core/src/tools/extra_tools.rs) | 107 | 实验性桩工具（已不注册） |
| [tools/registry.rs](../agent-core/src/tools/registry.rs) | 89 | ToolRegistry RwLock |
| [memory/core.rs](../agent-core/src/memory/core.rs) | 89 | MEMORY.md / USER.md L1+L2 |

**总计**：25 个文件 / **5,887 行 Rust**

## 附录 B — task 完整索引（31 个）

| ID | 任务 | M | 估时 | 风险 |
|---|---|---|---|---|
| R-1.1 | chat_stream ReAct 循环 | M1 | 2d | H |
| R-1.2 | Ollama provider | M1 | 1d | L |
| R-1.3 | MCP HTTP transport | M1 | 2d | M |
| R-1.4 | Config 字段接线 | M1 | 1d | L |
| R-1.5 | Anthropic prompt caching | M1 | 1d | L |
| R-1.6 | ExecuteCode 默认 Docker | M1 | 0.5d | L |
| R-1.7 | unwrap → unwrap_or_else | M1 | 0.5d | L |
| R-1.8 | Curator unused imports | M1 | 0.1d | L |
| R-L1 | Linux cargo install | M2 | 1d | L |
| R-L2 | Linux apt/snap 包（P3） | M2 | 1d | L |
| R-M1 | macOS universal .dylib 脚本 | M2 | 1.5d | L |
| R-M2 | macOS CI job | M2 | 1d | M |
| R-M3 | examples/macos-demo | M2 | 0.5d | L |
| R-W1 | Windows CI job | M2 | 1.5d | H |
| R-W2 | Windows portable-pty | M2 | 1d | M |
| R-W3 | Rust→C# 内存释放 | M2 | 1.5d | H |
| R-W4 | Aether.Tests C# 项目 | M2 | 1d | M |
| R-A1 | Android cargo-ndk + jniLibs | M2 | 2d | M |
| R-A2 | UniFFI Kotlin binding | M2 | 2d | M |
| R-A3 | Android emulator CI | M2 | 2d | H |
| R-A4 | examples/android-demo | M2 | 1d | L |
| R-3.1 | MCP server | M3 | 3d | M |
| R-3.2 | MCP OAuth | M3 | 2d | H |
| R-3.3 | codex_responses API mode | M3 | 2d | M |
| R-3.4 | Skill 命名改进 | M3 | 0.5d | L |
| R-3.5 | Compression CJK 估算 | M3 | 0.5d | L |
| R-3.6 | secure_path Err | M3 | 0.5d | L |
| R-D1 | CHANGELOG.md | M4 | 0.5d | L |
| R-D2 | crates.io publish prep | M4 | 1d | M |
| R-D3 | cargo doc 公开 API | M4 | 1d | L |
| R-T1 | 真 LLM e2e 测试 | M4 | 2d | M |
| R-T2 | self-audit.sh v1.0 检查 | M4 | 0.5d | L |

**总计 31 task / ~40 人日 / 单人 6-8 周（含 30% buffer）**

## 附录 C — 决策记录

| # | 决策 | 理由 |
|---|---|---|
| 1 | **Option B 4 端聚焦**（iOS/Web 冻结） | iOS 被 Apple FM 收编；Web 被 Vercel AI SDK 占住；4 端是 Rust+UniFFI 真正能立住的赛道 |
| 2 | **冻结而非删除** | `grep -r 'FROZEN(2026-06-16)'` 即可解冻；保留代码不破坏 git history |
| 3 | **6 周时间表 + 30% buffer** | 31 task × 平均 1.3 人日 = 40 人日；单人 8h/d × 6 周 = 48 人日；buffer 用于 RK2/RK3/RK5 等高风险 task |
| 4 | **顺序 Linux → macOS → Windows → Android** | 难度递增 + 经验复用；Linux 已 80%，Android 30% 最难 |
| 5 | **MCP server 用 binary 而非 library API** | 给 Claude Desktop / Cursor 等 host 用，stdio binary 是协议惯例 |
| 6 | **OAuth token 用 `keyring` crate** | 跨 Linux/macOS/Windows 一致 API；Android 让 host app 注入 |
| 7 | **真 LLM 测试用 mock 而非真 API** | 不依赖 API key 进 CI fork PR；mock 维护成本可接受 |
| 8 | **不做 crates.io 真发布** | dry-run 通过即可；正式发布留给 v1.0.0 正式版（非 alpha）|
| 9 | **Curator inline `spawn_blocking` 已选** | 不引入 LLM-driven 重对象重构；保持 minimal change |
| 10 | **self-audit.sh 持续扩展** | CLAUDE.md 铁律之一；每发现新债务就加新 grep |

---

## 给执行者的最终一句话

**这份路线图不是愿望清单，是 31 个 file:line 锚定的 atomic task。每个 task 完成的判据是 `bash scripts/self-audit.sh` 中能 grep 到对应实现。6 周后 v1.0-alpha 真能 ship，前提是不偏离 Option B 的 4 端聚焦 + CLAUDE.md 编码铁律。**

**第一步**：在 GitHub 开 31 个 issue，label `R-X.X`，关联本文档对应章节。每个 PR 关联 1 个 issue，merge 后勾选附录 B 的 acceptance checkbox。

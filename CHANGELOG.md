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
- `config.log_level` now wires through to `tracing_subscriber` (R-1.4 + H3)
- `config.skills_enabled` now gates SkillsList/View/Manage registration (R-1.4)
- `config.enabled_toolsets` / `disabled_toolsets` now filter tool registration (R-1.4)
- ExecuteCode tool: default backend changed `host` → `docker` (fallback `host` with warning) (R-1.6)
- `PromptBuilder` now exposes `build_parts()` with `PromptParts { stable, contextual, volatile }` (H4)

### Fixed (M1 hotfix)
- **H1**: MCP HTTP `Mcp-Session-Id` header now persisted across requests via `Mutex<Option<String>>` (was extracted then dropped)
- **H2**: `chat_stream_events` now respects `config.max_iterations` (was unbounded — could burn unlimited tokens)
- **H3**: `init_tracing()` now actually applies log filter via `Registry::default().with(filter).with(layer).try_init()` (was constructing subscriber then dropping it)
- **H4**: Anthropic `cache_control` now only on stable prompt segment (not on contextual segments containing `Local::now()`); `cache_read_input_tokens` now parsed from API response and exposed
- `curator.rs` 3× `.file_name().unwrap()` → `.unwrap_or("unknown")` (R-1.7)
- `curator.rs` unused imports removed (R-1.8)

### Breaking
- `StreamEvent` is a new public enum. Downstream code consuming `chat_stream`'s callback-based output should migrate to `chat_stream_events`. The old `chat_stream` callback API is present but will be deprecated.

### Internal
- `agent.rs` gained `chat_stream_events` (~130 lines) + `MpscStream` adapter
- `mcp/http.rs` new file (190 lines)
- `prompt.rs` gained `PromptParts` struct + 3 unit tests
- 83 tests passing (+9 from v0.4)

## [0.4.0] — 2026-06-16

### Added
- Real MCP stdio transport (initialize handshake, AtomicU64 id, async i/o)
- Real Delegate sub-agent (depth-limited, restricted toolset)
- Real FTS5 search using MATCH
- Profile isolation for Memory/Skills tools + Background Review
- Real SSRF defense: url::Url::parse + ToSocketAddrs DNS resolution + per-IP check
- Curator runs in tokio::task::spawn_blocking pool (non-blocking)
- Extended secret redaction: 8 prefix families + PEM private key blocks
- Terminal tool description honestly states host-process execution

### Breaking
- `AgentConfig.api_key` is now private and held as `secrecy::SecretString`. Use `api_key_expose()` to read, `set_api_key(...)` to write. Builder unchanged.

### Fixed
- ZH README now aligned with EN README
- Tool count corrected (14 + delegate)
- API key no longer leaks via Debug
- SSRF allowed by string-only blacklist now rejected

## [0.3.0] — Earlier

See `docs/devlog.md` for history before FIX_PLAN.

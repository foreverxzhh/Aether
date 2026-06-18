# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning follows [SemVer](https://semver.org/).

## [0.5.0-beta] — 2026-06-17

### Added (M1 — P0 核心补完)
- `chat_stream_events()` 返回 `impl Stream<Item = StreamEvent>` 真跑 ReAct 循环 (R-1.1)
- `StreamEvent` enum: `Text` / `ToolCall` / `ToolResult` / `Done` / `Error` (R-1.1)
- Ollama provider via OpenAI-compat endpoint (R-1.2)
- `McpHttpServer` — MCP HTTP transport with `initialize` handshake (R-1.3)
- Anthropic prompt caching: `cache_control: ephemeral` on stable system prompt (R-1.5)
- `Tool::toolset()` trait method, 15 built-in tools all override (R-1.4)
- `CodexProvider` — OpenAI Responses API support (R-3.3)
- `McpServer` — MCP stdio server, expose Aether tools to Claude Desktop/Cursor (R-3.1)
- `aether mcp-server` CLI subcommand (R-3.1)

### Changed (M2 — 4 端 binding)
- CI matrix: ubuntu-latest + windows-latest (R-W1)
- macOS universal dylib build (R-M1, R-M2)
- Android CI updated: .so → jniLibs artifact (R-A3)
- config.temperature / max_tokens wired to LLM providers (H6)
- compression_threshold_ratio replaces hardcoded 96000 (H6)
- CJK token estimation: ~1 token/char for Chinese (R-3.5)
- secure_path Err on canonicalize failure instead of fallback (R-3.6)

### Fixed (M1 hotfix)
- **H1**: MCP HTTP `Mcp-Session-Id` now persisted via `Mutex<Option<String>>`
- **H2**: `chat_stream_events` now respects `config.max_iterations`
- **H3**: `init_tracing()` chain-call `Registry::default().with(filter).try_init()`
- **H4**: Anthropic cache only on stable prompt segment; `cache_read_input_tokens` parsed
- `curator.rs` 3× `.file_name().unwrap()` → `.unwrap_or("unknown")` (R-1.7)
- Learning Loop skills now named `review-{YYYYMMDD_HHMMSS}` not `auto-learned-skill` (R-3.4)

### Breaking
- `StreamEvent` is a new public enum. Migrate callback-based `chat_stream` consumers to `chat_stream_events`.
- `memory_provider` and `max_concurrent_children` fields feature-gated (experimental_config).

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

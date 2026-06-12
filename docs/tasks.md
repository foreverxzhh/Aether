# Tasks: Aether Cross-platform Agent SDK (Phase 1 — Core Engine)

**Input**: `docs/requirements.md` (spec), `docs/implementation-plan.md` (plan)
**Prerequisites**: Rust toolchain, Hermes Agent v0.16.0 source at `../hermes/`
**Tests**: Hermes compatibility tests (Hermes generates test data, Rust parses and validates)

---

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Project initialization and basic structure

- [ ] T001 Create Aether project root with Cargo workspace in `Aether/Cargo.toml`
- [ ] T002 [P] Create `agent-core` crate with `Cargo.toml` (dependencies: tokio, reqwest, serde, serde_json, rusqlite, tracing, async-trait, uuid, thiserror)
- [ ] T003 [P] Create `agent-bindings` crate with `Cargo.toml` (dependencies: uniffi, wasm-bindgen)
- [ ] T004 [P] Configure Rust toolchain (`rust-toolchain.toml`) with nightly features
- [ ] T005 [P] Set up tracing subscriber (`tracing-subscriber`) in `agent-core/src/lib.rs`
- [ ] T006 Add `.gitignore` rules for Rust targets, IDE, OS files

**Checkpoint**: `cargo build --workspace` compiles successfully

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data types, error system, traits that ALL user stories depend on

### Core Data Types

- [ ] T007 [P] Define Message types (System/User/Assistant/Tool) in `agent-core/src/types/message.rs`
- [ ] T008 [P] Define ToolCall/ToolResult types in `agent-core/src/types/tool.rs`
- [ ] T009 [P] Define ModelResponse (text, tool_calls, finish_reason) in `agent-core/src/types/model.rs`
- [ ] T010 [P] Define AgentConfig struct (~60 fields, Builder pattern) in `agent-core/src/config.rs`

### Error System

- [ ] T011 [P] Define unified Error enum (`AetherError`) with error codes in `agent-core/src/error.rs`
- [ ] T012 [P] Implement `Display` and `From` conversions for AetherError

### Core Traits

- [ ] T013 [P] Define `ChatModel` trait (invoke, stream) in `agent-core/src/llm/mod.rs`
- [ ] T014 [P] Define `Tool` trait (name, description, parameters, call) in `agent-core/src/tools/mod.rs`
- [ ] T015 [P] Define `Memory` trait (add, get_context, clear) in `agent-core/src/memory/mod.rs`
- [ ] T016 [P] Define `SessionStore` trait (save, load, search, delete) in `agent-core/src/memory/state.rs`
- [ ] T017 [P] Define `SkillStore` trait (list, get, save, delete, search) in `agent-core/src/skills/mod.rs`

### Observability

- [ ] T018 Initialize tracing spans for agent lifecycle (agent_run, llm_call, tool_call) in `agent-core/src/tracing.rs`

**Checkpoint**: All core traits compile, AetherError usable across all modules

---

## Phase 3: User Story 1 — Agent Engine (Priority: P0) 🎯 MVP

**Goal**: Core ReAct loop that can converse with LLM, call tools, handle errors, stream responses

**Independent Test**: Hermes compatibility test: run same prompt through Hermes and Aether CLI, compare final response structure

### Implementation: LLM Providers

- [ ] T019 [P] [US1] Implement OpenAI Chat Completions provider in `agent-core/src/llm/openai.rs`
- [ ] T020 [P] [US1] Implement Anthropic Messages provider in `agent-core/src/llm/anthropic.rs`
- [ ] T021 [P] [US1] Implement Ollama provider (OpenAI-compatible) in `agent-core/src/llm/ollama.rs`
- [ ] T022 [US1] Implement generic OpenAI-compatible adapter in `agent-core/src/llm/provider.rs`

### Implementation: Agent Loop

- [ ] T023 [P] [US1] Build multi-layer system prompt assembler in `agent-core/src/prompt.rs`
- [ ] T024 [US1] Implement AIAgent struct with Builder pattern in `agent-core/src/agent.rs`
- [ ] T025 [US1] Implement `run_conversation()` ReAct loop in `agent-core/src/loop.rs`
- [ ] T026 [P] [US1] Implement 3 API mode dispatch (chat_completions/anthropic_messages/codex_responses) in `agent-core/src/loop.rs`
- [ ] T027 [P] [US1] Implement IterationBudget (AtomicUsize, refund logic) in `agent-core/src/budget.rs`
- [ ] T028 [P] [US1] Implement CircuitBreaker (tool signature hash + consecutive detection) in `agent-core/src/breaker.rs`
- [ ] T029 [US1] Implement streaming response (SSE parsing + interrupt) in `agent-core/src/loop.rs`

### Implementation: Error Recovery

- [ ] T030 [P] [US1] Implement error classification (empty response, truncation, invalid tool, provider error) in `agent-core/src/error.rs`
- [ ] T031 [US1] Implement retry logic with jittered backoff in `agent-core/src/loop.rs`
- [ ] T032 [US1] Implement graceful budget exhaustion handler in `agent-core/src/loop.rs`

### Implementation: Context Engine

- [ ] T033 [US1] Implement ContextEngine (inject workspace files, recent tool results) in `agent-core/src/context.rs`

### Implementation: CLI Demo

- [ ] T034 [US1] Build minimal CLI entry point in `agent-bindings/src/bin/cli.rs` (reads prompt from args, runs agent, prints response)
- [ ] T035 [US1] Build streaming CLI demo (stdin/stdout, real-time token output)

### Implementation: Hermes Compatibility Tests

- [ ] T036 [US1] Create test harness: run Hermes with test prompts, capture output, parse with Rust, compare results in `agent-core/tests/hermes_compat/mod.rs`

**Checkpoint**: `cargo run --bin cli -- "hello"` → agent responds. Streaming works. Error recovery works.

---

## Phase 4: User Story 2 — Tool System (Priority: P0)

**Goal**: Tool registry, toolset gating, core built-in tools (file, terminal, web)

**Independent Test**: Agent can use read_file + web_search + terminal tools in a single conversation

### Implementation: Tool Registry

- [ ] T037 [US2] Implement ToolRegistry (compile-time registration via `inventory` crate) in `agent-core/src/tools/registry.rs`
- [ ] T038 [US2] Implement Toolset system (grouping + check_fn gating + 30s TTL cache) in `agent-core/src/tools/toolsets.rs`
- [ ] T039 [US2] Implement JSON Schema auto-generation for tool parameters in `agent-core/src/tools/registry.rs`
- [ ] T040 [US2] Implement dynamic_schema_overrides (runtime description updates) in `agent-core/src/tools/registry.rs`
- [ ] T041 [US2] Implement runtime dynamic tool registration/deregistration in `agent-core/src/tools/registry.rs`
- [ ] T042 [US2] Implement CircuitBreaker integration (tool signature tracking) in `agent-core/src/tools/registry.rs`

### Implementation: File Tools

- [ ] T043 [P] [US2] Implement `read_file` tool in `agent-core/src/tools/file.rs`
- [ ] T044 [P] [US2] Implement `write_file` tool in `agent-core/src/tools/file.rs`
- [ ] T045 [P] [US2] Implement `patch` tool (diff-based editing) in `agent-core/src/tools/file.rs`
- [ ] T046 [P] [US2] Implement `search_files` tool (glob + regex) in `agent-core/src/tools/file.rs`

### Implementation: Terminal Tool

- [ ] T047 [US2] Implement `terminal` tool (subprocess execution with `portable-pty`) in `agent-core/src/tools/terminal.rs`
- [ ] T048 [US2] Add terminal safety checks (dangerous command filtering) in `agent-core/src/tools/terminal.rs`

### Implementation: Web Tools

- [ ] T049 [P] [US2] Implement `web_search` tool in `agent-core/src/tools/web.rs`
- [ ] T050 [P] [US2] Implement `web_extract` tool (HTML scraping) in `agent-core/src/tools/web.rs`

### Implementation: Hermes Compatibility Tests

- [ ] T051 [US2] Compatibility test: Hermes runs tools, Aether reads same tool schemas, compares structure in `agent-core/tests/hermes_compat/tools.rs`

**Checkpoint**: Agent can read files, search code, run terminal commands, search web

---

## Phase 5: User Story 3 — Memory & Skills (Priority: P0)

**Goal**: L1-L4 memory system, session storage (SQLite + FTS5), skill file management

**Independent Test**: Agent remembers information from previous conversation (memory persists across sessions)

### Implementation: Session Store

- [ ] T052 [P] [US3] Implement SQLite session store in `agent-core/src/memory/state.rs` (schema matching Hermes `hermes_state.py`)
- [ ] T053 [P] [US3] Implement FTS5 full-text search across session messages in `agent-core/src/memory/state.rs`
- [ ] T054 [US3] Implement session chain (parent_session_id, compression splits) in `agent-core/src/memory/state.rs`

### Implementation: Memory Manager (L1-L4)

- [ ] T055 [US3] Implement MemoryManager orchestrating L1-L4 in `agent-core/src/memory/mod.rs`
- [ ] T056 [P] [US3] Implement L1 Core Memory (MEMORY.md file read/write/auto-inject) in `agent-core/src/memory/core.rs`
- [ ] T057 [P] [US3] Implement L2 User Profile (USER.md file read/write) in `agent-core/src/memory/profile.rs`
- [ ] T058 [P] [US3] Implement L3 Skills index (FTS5 over skills/*.md) in `agent-core/src/memory/skills_index.rs`
- [ ] T059 [US3] Implement L4 long-term storage session archiving in `agent-core/src/memory/state.rs`

### Implementation: Skill System

- [ ] T060 [US3] Implement agentskills.io frontmatter + Markdown parser in `agent-core/src/skills/mod.rs`
- [ ] T061 [US3] Implement skill CRUD (list, view, create, update, delete) in `agent-core/src/skills/mod.rs`
- [ ] T062 [US3] Implement skill search (name + FTS5) in `agent-core/src/skills/mod.rs`

### Implementation: Memory/Skill Tools

- [ ] T063 [US3] Implement `memory` tool (read/write memory) in `agent-core/src/tools/memory_tool.rs`
- [ ] T064 [US3] Implement `skills_list`/`skill_view`/`skill_manage` tools in `agent-core/src/tools/skills.rs`

### Implementation: Profile System

- [ ] T065 [US3] Implement Profile system (isolated HERMES_HOME per profile) in `agent-core/src/profile.rs`

### Implementation: Hermes Compatibility Tests

- [ ] T066 [US3] Compatibility test: Hermes writes MEMORY.md/USER.md, Aether reads and parses correctly in `agent-core/tests/hermes_compat/memory.rs`

**Checkpoint**: Agent remembers user preferences across sessions, can list/read skills

---

## Phase 6: User Story 4 — Learning Loop & Compression (Priority: P0)

**Goal**: Background Review auto-generates skills/memory, Context Compression splits long conversations

**Independent Test**: Agent runs 3 turns with tool calls → Background Review thread fires → new skill appears in skills list

### Implementation: Context Compression

- [ ] T067 [US4] Implement token estimation (tiktoken equivalence) in `agent-core/src/compression/mod.rs`
- [ ] T068 [US4] Implement compression logic: identify range → LLM summary (protect head+tail) in `agent-core/src/compression/mod.rs`
- [ ] T069 [US4] Implement session splitting: new child session + parent_session_id chain in `agent-core/src/compression/mod.rs`
- [ ] T070 [US4] Implement iteration budget refund on compression in `agent-core/src/compression/mod.rs`

### Implementation: Prompt Caching

- [ ] T071 [US4] Implement Anthropic cache_control marker logic in `agent-core/src/llm/caching.rs`
- [ ] T072 [US4] Implement caching constraints (system prompt immutable mid-session, toolset frozen) in `agent-core/src/llm/caching.rs`

### Implementation: Background Review

- [ ] T073 [US4] Implement Background Review trigger logic (post-turn, check conditions) in `agent-core/src/memory/review.rs`
- [ ] T074 [US4] Implement forked review agent (inherit parent config, restrict toolset to memory+skills) in `agent-core/src/memory/review.rs`
- [ ] T075 [US4] Implement review prompts (memory review + skill review, ported from Hermes) in `agent-core/src/memory/review.rs`

### Implementation: Curator

- [ ] T076 [US4] Implement Curator scheduler (idle detection, interval config, state persistence) in `agent-core/src/memory/curator.rs`
- [ ] T077 [US4] Implement skill lifecycle transitions (active → stale → archived) in `agent-core/src/memory/curator.rs`
- [ ] T078 [US4] Implement archive/restore mechanism in `agent-core/src/memory/curator.rs`

### Implementation: Hermes Compatibility Tests

- [ ] T079 [US4] Compatibility test: Hermes compressed session → Aether reads parent_session_id chain correctly in `agent-core/tests/hermes_compat/compression.rs`

**Checkpoint**: Agent auto-creates skills from conversations, compresses long sessions

---

## Phase 7: User Story 5 — MCP & Delegate (Priority: P0)

**Goal**: MCP client/server protocol, sub-agent delegation

**Independent Test**: Agent connects to an MCP server, discovers its tools, and calls one

### Implementation: MCP Client

- [ ] T080 [P] [US5] Implement MCP Client (stdio) — JSON-RPC over subprocess stdin/stdout in `agent-core/src/mcp/client_stdio.rs`
- [ ] T081 [P] [US5] Implement MCP Client (HTTP/SSE) — JSON-RPC over HTTP in `agent-core/src/mcp/client_http.rs`
- [ ] T082 [US5] Implement MCP tool discovery (server → tool schema mapping) in `agent-core/src/mcp/mod.rs`
- [ ] T083 [US5] Implement dynamic tool list change notification in `agent-core/src/mcp/mod.rs`

### Implementation: MCP OAuth

- [ ] T084 [US5] Implement MCP OAuth flow (authorization, token refresh) in `agent-core/src/mcp/oauth.rs`

### Implementation: MCP Server

- [ ] T085 [US5] Implement MCP Server (expose Aether tools as MCP service) in `agent-core/src/mcp/server.rs`

### Implementation: Delegate

- [ ] T086 [US5] Implement sub-agent delegation (isolated context, restricted toolsets) in `agent-core/src/delegate.rs`
- [ ] T087 [US5] Implement batch delegation (parallel sub-agents, result aggregation) in `agent-core/src/delegate.rs`

**Checkpoint**: Agent can use MCP tools from external servers, delegate tasks to sub-agents

---

## Phase 8: Cross-Platform Bindings (Priority: P0)

**Goal**: UniFFI + WASM bindings so platform SDKs can call Aether core

**Independent Test**: TypeScript snippet imports WASM build, creates Agent, calls invoke()

### Implementation: UniFFI

- [ ] T088 [P] [BD] Define UniFFI UDL (`agent.udl`) with all exported types and functions in `agent-bindings/agent.udl`
- [ ] T089 [BD] Implement `#[uniffi::export]` wrappers for Agent create/invoke/stream/save/load in `agent-bindings/src/uniffi.rs`
- [ ] T090 [BD] Generate Kotlin bindings (`uniffi-bindgen kotlin`) and test in `agent-bindings/`
- [ ] T091 [BD] Generate Swift bindings (`uniffi-bindgen swift`) and test in `agent-bindings/`

### Implementation: WASM

- [ ] T092 [BD] Implement WASM entry point (wasm-bindgen exports) in `agent-bindings/src/wasm.rs`
- [ ] T093 [BD] Build WASM target (`wasm-pack build --target web`) in `agent-bindings/`

### Implementation: CLI as Native Linux Binary

- [ ] T094 [BD] Polish CLI binary for Linux/macOS in `agent-bindings/src/bin/cli.rs`
- [ ] T095 [BD] Build CI pipeline for cross-platform targets in `scripts/build-all.sh`

**Checkpoint**: WASM demo page loads Aether agent in browser. Kotlin/Swift bindings compile.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T096 [P] Add comprehensive tracing spans across all modules in `agent-core/src/`
- [ ] T097 [P] Add logging for each LLM call (model, tokens, duration) in `agent-core/src/llm/`
- [ ] T098 [P] Add logging for each tool call (name, params, result, duration) in `agent-core/src/tools/`
- [ ] T099 Add Hermes compatibility CI step (`python scripts/test_hermes_compat.py`) in `scripts/`
- [ ] T100 Code cleanup and documentation pass across all modules
- [ ] T101 Performance optimization: reduce cold start time < 50ms
- [ ] T102 WASM binary size optimization: target < 5MB

---

## Dependencies & Execution Order

### Phase Dependencies

| Phase | Depends On | Blocks |
|-------|-----------|--------|
| **P1: Setup** | — | All phases |
| **P2: Foundational** | P1 | US1 (Phase 3) |
| **P3: US1 Agent Engine** | P1+P2 | US2, US3, US4, US5 |
| **P4: US2 Tool System** | P3 | — |
| **P5: US3 Memory & Skills** | P3 | — |
| **P6: US4 Learning Loop** | P3+P5 | — |
| **P7: US5 MCP & Delegate** | P3+P4 | — |
| **P8: Cross-Platform** | P3+P5 | — |
| **P9: Polish** | All | — |

### Within Each User Story

- Core types → Traits → Implementation → Tools → Integration
- Each story should be independently testable after completion

### Parallel Opportunities

- Phase 1 T002/T003/T004/T005 (all [P]) can run in parallel
- Phase 2 T007-T017 (all [P]) can run in parallel
- Phase 3 LLM providers T019/T020/T021 can run in parallel
- Phase 4 T043-T050 (file/terminal/web tools) can run in parallel
- Phase 5 T052/T053 (state/FTS5) can run in parallel
- Phases 4-7 can technically run after Phase 3 completes (if team capacity)

---

## Implementation Strategy

### MVP First (Phase 1-3 Only)

1. Complete Phase 1: Setup → workspace compiles
2. Complete Phase 2: Foundational → core traits done
3. Complete Phase 3: US1 → Agent runs with CLI demo
4. **STOP and VALIDATE**: CLI demo works, Hermes compat test passes
5. MVP deliverable: working CLI agent with ReAct loop, file/web tools, basic streaming

### Incremental Delivery

1. Phase 1-3 → MVP CLI Agent (usable!)
2. Phase 4 → Agent with full tool system
3. Phase 5 → Agent with memory and skills
4. Phase 6 → Self-learning agent
5. Phase 7 → MCP-connected agent ecosystem
6. Phase 8 → Cross-platform SDK

### MVP Scope

**Phase 1-3 only** (≈ 4-6 weeks AI-assisted): A working CLI agent that can:
- Converse with OpenAI/Anthropic/Ollama
- Call file/terminal/web tools
- Stream responses
- Handle errors gracefully
- Run Hermes compatibility tests

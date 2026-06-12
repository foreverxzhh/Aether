# Aether — Cross-platform Agent SDK

> One core. Every platform. Hermes soul, Rust power.

Aether is a **cross-platform Agent SDK** that reimplements the [Hermes Agent](https://github.com/NousResearch/hermes-agent) architecture in Rust. It runs on Android, iOS, Windows, macOS, Linux, and Web through UniFFI and WASM bindings.

## Features

- **Agent Engine** — ReAct loop with 3 API modes (Chat Completions / Anthropic Messages / Codex Responses)
- **Learning Loop** — Background review auto-generates skills and updates memory
- **L1-L4 Memory** — Core memory, user profile, skills, long-term storage (SQLite + FTS5)
- **Skill System** — agentskills.io compatible, auto-generation, patching
- **MCP Protocol** — Full client & server support (stdio + HTTP)
- **Profile System** — Multi-instance isolation
- **Cross-platform** — One Rust core → Kotlin (Android), Swift (iOS/macOS), C# (Windows), WASM (Web)

## Project Status

🚧 **Pre-release — Architecture & planning phase**

- [x] Architecture design complete
- [x] Requirements documented
- [ ] Core Agent engine (in progress)
- [ ] Platform SDKs (planned)

## Architecture

```
Rust Core (agent-core crate)
    ↓ UniFFI + wasm-bindgen
Platform SDKs (Kotlin / Swift / C# / TypeScript)
    ↓
Native Apps (Android / iOS / Windows / macOS / Web)
```

See [docs/implementation-plan.md](docs/implementation-plan.md) for full details.

## License

MIT License. See [LICENSE](LICENSE).

Includes work derived from Hermes Agent by Nous Research (also MIT). See [NOTICE](NOTICE).

## Acknowledgments

Aether's architecture is inspired by [Hermes Agent](https://github.com/NousResearch/hermes-agent), an outstanding open-source AI agent by [Nous Research](https://nousresearch.com/). We are deeply grateful for their pioneering work.

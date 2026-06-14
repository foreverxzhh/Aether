# Aether SDK 快速入门

> 本文档帮助你快速了解如何编译、集成和使用 Aether SDK。

---

## 1. 环境要求

- **Rust** 1.94.0+（见 `rust-toolchain.toml`）
- **Windows**: Visual Studio Build Tools 2022（MSVC）
- **macOS / Linux**: GCC / Clang

```bash
# 确认 Rust 已安装
rustc --version   # ≥ 1.94.0
cargo --version

# 安装 wasm 编译目标（后续 Web SDK 需要）
rustup target add wasm32-unknown-unknown
```

---

## 2. 编译项目

```bash
# 克隆
git clone https://github.com/foreverxzhh/aether.git
cd aether

# 编译全部（库 + CLI）
cargo build --release

# 仅编译库
cargo build -p agent-core --release

# 运行测试
cargo test
```

编译产物：
```
target/release/
├── aether.exe     ← CLI 工具（Windows）
├── aether         ← CLI 工具（macOS/Linux）
└── deps/
    ├── libagent_core.rlib    ← Rust 静态库
    └── libagent_bindings.dll ← 动态库（Windows）
```

---

## 3. 使用 CLI 工具

### 3.1 设置 API Key

```bash
# PowerShell (Windows)
$env:DEEPSEEK_API_KEY = "sk-xxx"

# Git Bash / macOS / Linux
export DEEPSEEK_API_KEY="sk-xxx"
```

CLI 会自动读取 `{供应商大写}_API_KEY` 环境变量，不需要每次都传 `-k`。

### 3.2 对话

```bash
# 基本对话
aether -p deepseek -m deepseek-v4-flash -c "你好"

# 流式输出（逐字显示）
aether -p deepseek -m deepseek-v4-flash -t -c "讲个故事"

# 指定 API Key
aether -p openai -k sk-xxx -m gpt-4o -c "Hello"

# 自定义 API 地址（兼容 OpenAI 协议）
aether -p custom -b "https://your-api.com/v1" -k sk-xxx -c "你好"
```

### 3.3 CLI 参数说明

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-p` | LLM 供应商 | `openai` |
| `-m` | 模型名 | `gpt-4o` |
| `-k` | API Key | 环境变量 `XXX_API_KEY` |
| `-b` | API 地址 | 各供应商默认地址 |
| `-s` | 系统提示词 | — |
| `-c` | 用户输入 | **必填** |
| `-t` | 流式输出 | 关闭 |

---

## 4. 作为 Rust 库集成

### 4.1 添加依赖

```toml
[dependencies]
agent-core = { path = "/path/to/aether/agent-core" }
tokio = { version = "1", features = ["full"] }
```

### 4.2 基本用法

```rust
use agent_core::*;
use agent_core::config::AgentConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建 Agent 配置
    let config = AgentConfigBuilder::new()
        .provider("deepseek")
        .model("deepseek-v4-flash")
        .api_key(std::env::var("DEEPSEEK_API_KEY")?)
        .system_prompt("你是一个乐于助人的助手")
        .build();

    // 2. 初始化 Agent
    let mut agent = AIAgent::new(config);
    agent.init_model().await?;

    // 3. 对话
    let reply = agent.chat("你是谁？").await?;
    println!("{}", reply);

    Ok(())
}
```

### 4.3 流式对话

```rust
use std::io::Write;

agent.chat_stream("讲个长故事", |chunk| {
    print!("{}", chunk.delta);
    std::io::stdout().flush().ok();
}).await?;
```

### 4.4 带工具的 Agent

工具会自动注册到 AIAgent。Agent 会在需要时调用它们：

```rust
// Agent 会自动使用以下工具：
// - read_file / write_file / patch / search_files
// - terminal
// - web_search / web_extract
// - memory / skills_list / skill_view / skill_manage
```

### 4.5 错误处理

```rust
match agent.chat("你好").await {
    Ok(reply) => println!("{}", reply),
    Err(AetherError::LlmError(msg)) => eprintln!("LLM 错误: {}", msg),
    Err(AetherError::BudgetExhausted) => eprintln!("预算耗尽"),
    Err(AetherError::ToolExecutionError(msg)) => eprintln!("工具错误: {}", msg),
    Err(e) => eprintln!("其他错误: {}", e),
}
```

---

## 5. 支持的非功能性供应商

| 供应商 | `-p` 参数 | 默认模型 | API Key 环境变量 |
|--------|-----------|----------|-----------------|
| OpenAI | `openai` | `gpt-4o` | `OPENAI_API_KEY` |
| Anthropic | `anthropic` | `claude-sonnet-4-6` | `ANTHROPIC_API_KEY` |
| DeepSeek | `deepseek` | `deepseek-v4-flash` | `DEEPSEEK_API_KEY` |
| Ollama（本地） | `ollama` | `llama3` | 无需 Key |

---

## 6. 项目结构

```
Aether/
├── Cargo.toml              ← 工作空间配置
├── agent-core/             ← SDK 核心库（所有业务逻辑）
│   ├── src/
│   │   ├── lib.rs          ← 库入口，公开 API
│   │   ├── agent.rs        ← AIAgent 主类
│   │   ├── loop_mod.rs     ← ReAct 循环
│   │   ├── llm/            ← LLM 供应商
│   │   ├── tools/          ← 工具系统
│   │   ├── memory/         ← 记忆 + 会话存储
│   │   ├── skills/         ← 技能系统
│   │   ├── mcp/            ← MCP 协议
│   │   └── ...
│   └── tests/
├── agent-bindings/         ← CLI 工具 + 平台绑定
│   └── src/bin/cli.rs      ← CLI 入口
└── docs/                   ← 文档
```

---

## 7. 常见问题

**Q: 提示「不支持的供应商」？**
A: 确认 `-p` 参数为 `openai` / `anthropic` / `deepseek` / `ollama`。其他兼容 OpenAI 协议的 API 用 `-p custom -b "https://..."`。

**Q: API Key 怎么设置？**
A: 环境变量 `{大写供应商名}_API_KEY`，如 `DEEPSEEK_API_KEY`、`OPENAI_API_KEY`。或者用 `-k` 参数。

**Q: 编译报错找不到链接器？**
A: Windows 需要安装 Visual Studio Build Tools（勾选 MSVC 工具集）。macOS 需要 Xcode Command Line Tools。

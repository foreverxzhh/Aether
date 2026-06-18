# Linux 安装指南

## 前置条件

- Rust 1.94+（通过 [rustup](https://rustup.rs) 安装）
- 可选：Docker（ExecuteCode 沙箱后端）

## 方式 1：cargo install（推荐）

```bash
# 安装 CLI 工具
cargo install --path agent-bindings --features cli

# 验证
aether --help
```

## 方式 2：从源码构建

```bash
git clone https://github.com/foreverxzhh/Aether.git
cd Aether
cargo build --release --features cli
./target/release/aether --help
```

## 方式 3：作为 Rust 库

```toml
[dependencies]
agent-core = { path = "..." }  # 或从 crates.io（发布后）
```

```rust
use agent_core::*;
use agent_core::config::AgentConfigBuilder;

#[tokio::main]
async fn main() {
    let mut agent = AIAgent::new(
        AgentConfigBuilder::new()
            .provider("deepseek")
            .model("deepseek-v4-flash")
            .api_key(std::env::var("DEEPSEEK_API_KEY").unwrap())
            .build()
    );
    agent.init_model().await.unwrap();
    let reply = agent.chat("Hello!").await.unwrap();
    println!("{}", reply);
}
```

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `DEEPSEEK_API_KEY` | DeepSeek API 密钥 | — |
| `OPENAI_API_KEY` | OpenAI API 密钥 | — |
| `ANTHROPIC_API_KEY` | Anthropic API 密钥 | — |
| `RUST_LOG` | 日志级别（trace/debug/info/warn/error） | info |

## 验证

```bash
# 同步对话
export DEEPSEEK_API_KEY="sk-xxx"
aether -p deepseek -m deepseek-v4-flash -c "你好"

# 流式对话
aether -p deepseek -m deepseek-v4-flash -t -c "讲个故事"

# 指定 profile（隔离记忆/技能）
aether -p deepseek -m deepseek-v4-flash --profile my-project -c "你好"
```

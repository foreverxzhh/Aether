use agent_core::config::AgentConfigBuilder;
use agent_core::tracing::init_tracing;
use clap::{Parser, Subcommand};

/// Aether — 跨平台 Agent SDK
#[derive(Parser, Debug)]
#[command(name = "aether")]
#[command(version = "0.5.0-beta")]
#[command(about = "跨平台 Agent SDK CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// LLM 供应商
    #[arg(short = 'p', long, default_value = "openai")]
    provider: String,

    /// 模型名称
    #[arg(short = 'm', long, default_value = "gpt-4o")]
    model: String,

    /// API Key
    #[arg(short = 'k', long)]
    api_key: Option<String>,

    /// API Base URL（OpenAI 兼容 API）
    #[arg(short = 'b', long)]
    base_url: Option<String>,

    /// 系统提示词
    #[arg(short = 's', long)]
    system: Option<String>,

    /// 用户输入（chat 模式）
    #[arg(short = 'c', long)]
    prompt: Option<String>,

    /// 流式输出
    #[arg(short = 't', long)]
    stream: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// R-3.1: 启动 MCP server（stdio JSON-RPC）
    McpServer {
        #[arg(short = 'p', long, default_value = "openai")]
        provider: String,
        #[arg(short = 'm', long, default_value = "gpt-4o")]
        model: String,
        #[arg(short = 'k', long)]
        api_key: Option<String>,
        #[arg(short = 'b', long)]
        base_url: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::McpServer { provider, model, api_key, base_url }) => {
            init_tracing("warn"); // MCP server 模式下减少日志噪音
            eprintln!("🔌 Aether MCP Server 启动中...");

            let mut builder = AgentConfigBuilder::new()
                .provider(provider)
                .model(model);
            if let Some(ref k) = api_key {
                builder = builder.api_key(k);
            }
            if let Some(ref url) = base_url {
                builder = builder.base_url(url);
            }

            let agent = agent_core::AIAgent::new(builder.build());
            let tools = agent.tools().clone(); // clone Arc<RwLock<ToolRegistry>>

            let server = agent_core::mcp::server::McpServer::new(tools);
            eprintln!("   ✓ MCP Server 就绪，等待 host 连接...");
            if let Err(e) = server.run() {
                eprintln!("❌ MCP Server 错误: {}", e);
                std::process::exit(1);
            }
            return;
        }
        None => {}
    }

    // ── 默认：chat 模式 ──
    init_tracing("info");

    let prompt = cli.prompt.as_deref().unwrap_or("你好");

    let api_key = cli.api_key.or_else(|| {
        let var = format!("{}_API_KEY", cli.provider.to_uppercase());
        std::env::var(&var).ok()
    });

    let mut builder = AgentConfigBuilder::new()
        .provider(&cli.provider)
        .model(&cli.model);

    if let Some(ref key) = api_key {
        builder = builder.api_key(key.as_str());
    }
    if let Some(ref url) = cli.base_url {
        builder = builder.base_url(url.as_str());
    }
    if let Some(ref sys) = cli.system {
        builder = builder.system_prompt(sys.as_str());
    }

    let config = builder.build();
    let mut agent = agent_core::AIAgent::new(config);

    eprintln!("🚀 初始化 {} / {} ...", cli.provider, cli.model);
    if let Err(e) = agent.init_model().await {
        eprintln!("❌ 初始化失败: {}", e);
        std::process::exit(1);
    }
    eprintln!("   ✓ 就绪\n");

    eprintln!("🧑 用户: {}", prompt);
    eprintln!("🤖 Aether:\n");

    if cli.stream {
        match agent
            .chat_stream(prompt, |chunk| {
                print!("{}", chunk.delta);
                use std::io::Write;
                std::io::stdout().flush().ok();
            })
            .await
        {
            Ok(full) => {
                println!("\n");
                let words = full.split_whitespace().count();
                eprintln!("  (共 {} 字, {} 词)", full.chars().count(), words);
            }
            Err(e) => {
                eprintln!("\n❌ 错误: {}", e);
            }
        }
    } else {
        match agent.chat(prompt).await {
            Ok(response) => {
                println!("{}\n", response);
            }
            Err(e) => {
                eprintln!("❌ 错误: {}", e);
            }
        }
    }
}

use agent_core::config::AgentConfigBuilder;
use agent_core::tracing::init_tracing;
use clap::Parser;

/// Aether — 跨平台 Agent SDK
#[derive(Parser, Debug)]
#[command(name = "aether")]
#[command(version = "0.1.0")]
#[command(about = "跨平台 Agent SDK CLI")]
struct Cli {
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

    /// 用户输入
    #[arg(short = 'c', long)]
    prompt: String,

    /// 流式输出
    #[arg(short = 't', long)]
    stream: bool,
}

#[tokio::main]
async fn main() {
    init_tracing("info");

    let cli = Cli::parse();

    // 从环境变量获取 API Key
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

    // 初始化模型
    eprintln!("🚀 初始化 {} / {} ...", cli.provider, cli.model);
    if let Err(e) = agent.init_model().await {
        eprintln!("❌ 初始化失败: {}", e);
        std::process::exit(1);
    }
    eprintln!("   ✓ 就绪\n");

    // 用户输入
    eprintln!("🧑 用户: {}", cli.prompt);
    eprintln!("🤖 Aether:\n");

    // 运行对话（流式或非流式）
    if cli.stream {
        match agent
            .chat_stream(&cli.prompt, |chunk| {
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
        match agent.chat(&cli.prompt).await {
            Ok(response) => {
                println!("{}\n", response);
            }
            Err(e) => {
                eprintln!("❌ 错误: {}", e);
            }
        }
    }
}

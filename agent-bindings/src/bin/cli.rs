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

    /// 用户输入
    #[arg(short = 'c', long)]
    prompt: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    println!("🧪 Aether v{}", env!("CARGO_PKG_VERSION"));
    println!("   供应商: {}", cli.provider);
    println!("   模型: {}", cli.model);
    println!();

    let agent = agent_bindings::create_agent(
        &cli.provider,
        &cli.model,
        cli.api_key.as_deref(),
    );

    match agent.chat(&cli.prompt).await {
        Ok(response) => println!("🤖 {}\n", response),
        Err(e) => eprintln!("❌ 错误: {}", e),
    }
}

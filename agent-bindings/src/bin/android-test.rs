/// Android 真机测试
use agent_core::AIAgent;
use agent_core::config::AgentConfigBuilder;
use std::io::Write;

fn main() {
    let api_key = "REDACTED_API_KEY";
    println!("Aether Android Test");
    println!("Provider: deepseek, Model: deepseek-v4-flash");
    std::io::stdout().flush().ok();

    let mut agent = AIAgent::new(
        AgentConfigBuilder::new()
            .provider("deepseek")
            .model("deepseek-v4-flash")
            .api_key(api_key)
            .build()
    );

    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => { println!("Runtime error: {}", e); std::io::stdout().flush().ok(); return; }
    };

    rt.block_on(async {
        match agent.init_model().await {
            Ok(_) => { println!("Model init OK"); std::io::stdout().flush().ok(); }
            Err(e) => { println!("Init error: {:?}", e); std::io::stdout().flush().ok(); return; }
        }
        match agent.chat("hi, introduce yourself").await {
            Ok(reply) => { println!("SUCCESS: {}", reply); std::io::stdout().flush().ok(); }
            Err(e) => { println!("Chat error: {:?}", e); std::io::stdout().flush().ok(); }
        }
    });
    std::io::stdout().flush().ok();
}

use agent_core::config::AgentConfigBuilder;
/// Aether 集成测试 — 覆盖核心路径
use agent_core::*;

#[test]
fn test_config_builder() {
    let config = AgentConfigBuilder::new()
        .provider("deepseek")
        .model("deepseek-v4-flash")
        .api_key("test-key")
        .system_prompt("test prompt")
        .max_iterations(50)
        .session_id("test-session")
        .build();

    assert_eq!(config.provider, "deepseek");
    assert_eq!(config.model, "deepseek-v4-flash");
    assert_eq!(config.api_key, Some("test-key".into()));
    assert_eq!(config.max_iterations, 50);
    assert_eq!(config.session_id, Some("test-session".into()));
}

#[test]
fn test_config_defaults() {
    let config = AgentConfigBuilder::new()
        .provider("openai")
        .model("gpt-4o")
        .build();

    assert_eq!(config.max_iterations, 90);
    assert!(config.memory_enabled);
    assert!(config.compression_enabled);
}

#[test]
fn test_message_types() {
    use agent_core::types::message::*;

    let sys = Message::system("System prompt");
    assert!(matches!(sys.role, MessageRole::System));
    if let Some(Content::Text(t)) = sys.content {
        assert_eq!(t, "System prompt");
    } else {
        panic!("Expected text content");
    }

    let user = Message::user("Hello");
    assert!(matches!(user.role, MessageRole::User));

    let tool = Message::tool_result("call_1", "result");
    assert_eq!(tool.tool_call_id, Some("call_1".into()));
}

#[test]
fn test_error_display() {
    let e = AetherError::ToolNotFound("test".into());
    assert!(e.to_string().contains("TE001"));
    assert!(e.to_string().contains("test"));

    let e = AetherError::BudgetExhausted;
    assert!(e.to_string().contains("AE102"));

    let e = AetherError::LlmError("API error".into());
    assert_eq!(e.category(), "LLM");
}

#[test]
fn test_error_io_from() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let aether_err: AetherError = io_err.into();
    assert!(matches!(aether_err, AetherError::IoError(_)));
}

#[test]
fn test_model_response_types() {
    use agent_core::types::model::*;

    let resp = ModelResponse {
        content: Some("Hello".into()),
        tool_calls: None,
        finish_reason: FinishReason::Stop,
        usage: Some(TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        }),
    };
    assert_eq!(resp.content, Some("Hello".into()));
    assert_eq!(resp.finish_reason, FinishReason::Stop);
}

#[test]
fn test_finish_reason_variants() {
    use agent_core::types::model::FinishReason;
    assert_eq!(FinishReason::from_str("stop"), FinishReason::Stop);
    assert_eq!(FinishReason::from_str("length"), FinishReason::Length);
    assert_eq!(
        FinishReason::from_str("tool_calls"),
        FinishReason::ToolCalls
    );
}

#[test]
fn test_budget_creation() {
    use agent_core::budget::IterationBudget;
    let b = IterationBudget::new(10);
    assert_eq!(b.remaining(), 10);
    for _ in 0..10 {
        assert!(b.consume());
    }
    assert!(!b.consume());
}

#[test]
fn test_breaker_creation() {
    use agent_core::breaker::CircuitBreaker;
    let b = CircuitBreaker::new(3);
    let args = serde_json::json!({"x": 1});
    assert!(!b.check("tool1", &args));
    assert!(!b.check("tool1", &args));
    // 3rd call with same signature should trigger
    assert!(b.check("tool1", &args));
    b.reset();
    assert!(!b.check("tool1", &args));
}

#[test]
fn test_prompt_builder() {
    use agent_core::prompt::PromptBuilder;
    let msg = PromptBuilder::build_system_message(
        Some("You are a test agent"),
        Some("Context: testing"),
        Some("Time: now"),
    );
    let text = match msg.content {
        Some(agent_core::types::message::Content::Text(t)) => t,
        _ => String::new(),
    };
    assert!(text.contains("test agent"));
    assert!(text.contains("Context"));
    assert!(text.contains("Time"));
}

#[test]
fn test_ctx_engine() {
    use agent_core::context::ContextEngine;
    let ctx = ContextEngine::collect_context(None);
    assert!(ctx.contains("当前时间"));
}

#[test]
fn test_tool_def_creation() {
    use agent_core::types::tool::ToolDef;
    let def = ToolDef::new(
        "test_tool",
        "Test description",
        serde_json::json!({"type":"object"}),
    );
    assert_eq!(def.def_type, "function");
    assert_eq!(def.function.name, "test_tool");
}

#[test]
fn test_simple_token_estimator() {
    use agent_core::llm::SimpleTokenEstimator;
    use agent_core::llm::TokenEstimator;
    let messages = vec![agent_core::types::message::Message::user("Hello world!")];
    let tokens = SimpleTokenEstimator.estimate_messages_tokens(&messages);
    assert!(tokens > 0, "应返回非零 token 估算值");
}

#[test]
fn test_config_serialization_skips_api_key() {
    let config = AgentConfigBuilder::new()
        .provider("openai")
        .model("gpt-4o")
        .api_key("secret")
        .build();
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.contains("secret"), "api_key 不应出现在序列化输出中");
}

#[test]
fn test_compression_estimator() {
    use agent_core::compression::ContextCompressor;
    use agent_core::types::message::Message;
    use std::time::Duration;

    let msgs = vec![
        Message::system("Long system prompt"),
        Message::user("Hello world"),
    ];
    let tokens = ContextCompressor::estimate_tokens(&msgs);
    assert!(tokens > 10);
}

#[test]
fn test_profile_manager() {
    use agent_core::profile::ProfileManager;
    let pm = ProfileManager::new(None);
    let profiles = pm.list_profiles().unwrap();
    assert!(profiles.contains(&"default".to_string()));
}

#[test]
fn test_cache_tracker() {
    use agent_core::llm::caching::CacheTracker;
    let mut ct = CacheTracker::new();
    assert!(ct.needs_breakpoint(1500));
    ct.mark_breakpoint(1500);
    assert!(!ct.needs_breakpoint(2000));
    assert!(CacheTracker::would_break_cache("reload_memory"));
}

#[test]
fn test_simple_token_est_total() {
    use agent_core::llm::{SimpleTokenEstimator, TokenEstimator};
    use agent_core::types::message::Message;
    let msgs = vec![Message::user("Hi")];
    let total = SimpleTokenEstimator.estimate_total_tokens(&msgs, &[]);
    assert!(total > 0);
}

#[test]
fn test_finish_reason_equality() {
    use agent_core::types::model::FinishReason;
    assert_eq!(FinishReason::Stop, FinishReason::Stop);
    assert_ne!(FinishReason::Stop, FinishReason::Length);
}

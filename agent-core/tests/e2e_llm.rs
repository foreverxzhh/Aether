//! R-T1: LLM round-trip 集成测试
//!
//! 覆盖: chat formatting / tool calls / stream types / token estimation / error types / config

use agent_core::config::AgentConfigBuilder;
use agent_core::types::message::{Content, Message, MessageRole};
use agent_core::types::model::{FinishReason, ModelResponse, StreamChunk, StreamEvent, TokenUsage};
use agent_core::AIAgent;

// ── Test 1: ModelResponse round-trip (chat happy path) ──

#[test]
fn test_model_response_round_trip() {
    let response = ModelResponse {
        content: Some("你好！有什么可以帮助你的？".into()),
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

    assert_eq!(response.content.unwrap(), "你好！有什么可以帮助你的？");
    assert_eq!(response.finish_reason, FinishReason::Stop);
    assert_eq!(response.usage.unwrap().total_tokens, 15);
}

// ── Test 2: ModelResponse with tool calls ──

#[test]
fn test_model_response_with_tool_calls() {
    use agent_core::types::model::ToolCallInfo;

    let response = ModelResponse {
        content: None,
        tool_calls: Some(vec![ToolCallInfo {
            id: "call_1".into(),
            name: "read_file".into(),
            arguments: r#"{"path":"test.txt"}"#.into(),
        }]),
        finish_reason: FinishReason::ToolCalls,
        usage: None,
    };

    let calls = response.tool_calls.unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "read_file");
    assert_eq!(calls[0].id, "call_1");
}

// ── Test 3: StreamEvent variants ──

#[test]
fn test_stream_event_variants() {
    let text = StreamEvent::Text("hello".into());
    let done = StreamEvent::Done("world".into());
    let err = StreamEvent::Error("oops".into());
    let tool = StreamEvent::ToolCall {
        id: "t1".into(),
        name: "read".into(),
        arguments: "{}".into(),
    };
    let result = StreamEvent::ToolResult {
        tool_call_id: "t1".into(),
        result: "ok".into(),
    };

    assert!(matches!(text, StreamEvent::Text(_)));
    assert!(matches!(done, StreamEvent::Done(_)));
    assert!(matches!(err, StreamEvent::Error(_)));
    assert!(matches!(tool, StreamEvent::ToolCall { .. }));
    assert!(matches!(result, StreamEvent::ToolResult { .. }));
}

// ── Test 4: StreamChunk structure ──

#[test]
fn test_stream_chunk_structure() {
    let chunk = StreamChunk {
        delta: "hello".into(),
        tool_calls: None,
        finish_reason: None,
        usage: None,
    };
    assert_eq!(chunk.delta, "hello");
    assert!(chunk.finish_reason.is_none());

    let final_chunk = StreamChunk {
        delta: String::new(),
        tool_calls: None,
        finish_reason: Some(FinishReason::Stop),
        usage: None,
    };
    assert!(final_chunk.finish_reason.is_some());
}

// ── Test 5: FinishReason equality ──

#[test]
fn test_finish_reason_equality() {
    assert_eq!(FinishReason::Stop, FinishReason::Stop);
    assert_ne!(FinishReason::Stop, FinishReason::Length);
    assert_eq!(FinishReason::ToolCalls, FinishReason::ToolCalls);
}

// ── Test 6: Token usage with cache fields ──

#[test]
fn test_token_usage_with_cache() {
    let usage = TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
        cache_read_tokens: Some(80),
        cache_creation_tokens: Some(20),
    };

    assert_eq!(usage.total_tokens, 150);
    assert_eq!(usage.cache_read_tokens, Some(80));
    assert_eq!(usage.cache_creation_tokens, Some(20));
}

// ── Test 7: Config builder full round-trip ──

#[test]
fn test_config_builder_full_round_trip() {
    let config = AgentConfigBuilder::new()
        .provider("deepseek")
        .model("deepseek-v4-flash")
        .api_key("sk-test-key")
        .max_iterations(50)
        .system_prompt("test system")
        .session_id("session-1")
        .build();

    assert_eq!(config.provider, "deepseek");
    assert_eq!(config.model, "deepseek-v4-flash");
    assert_eq!(config.api_key_expose(), Some("sk-test-key".into()));
    assert_eq!(config.max_iterations, 50);
    assert_eq!(config.system_prompt, Some("test system".into()));
    assert_eq!(config.session_id, Some("session-1".into()));
    assert!(config.compression_enabled);
    assert!(config.memory_enabled);
    assert!(config.skills_enabled);
    assert!(config.delegation_enabled);
}

// ── Test 8: Config defaults ──

#[test]
fn test_config_defaults() {
    let config = AgentConfigBuilder::new()
        .provider("openai")
        .model("gpt-4o")
        .build();

    assert_eq!(config.max_iterations, 90);
    assert!(config.compression_enabled);
    assert_eq!(config.compression_threshold_ratio, 0.75);
    assert_eq!(config.log_level, "info");
    assert_eq!(config.max_spawn_depth, 2);
}

// ── Test 9: Agent creation and tool definitions ──

#[test]
fn test_agent_creation_and_tool_defs() {
    let config = AgentConfigBuilder::new()
        .provider("openai")
        .model("gpt-4o")
        .build();

    let agent = AIAgent::new(config);
    let defs = agent.get_tool_definitions();
    // With default config (core tools only, skills_enabled=false? No, default is true)
    // At minimum: file tools + terminal + web + memory + skills + docker/ssh/execute
    assert!(!defs.is_empty(), "Tool definitions should not be empty");
}

// ── Test 10: Agent tool registry isolation ──

#[test]
fn test_agent_registry_isolation() {
    let config1 = AgentConfigBuilder::new()
        .provider("openai")
        .model("gpt-4o")
        .api_key("sk-1")
        .build();

    let config2 = AgentConfigBuilder::new()
        .provider("openai")
        .model("gpt-4o")
        .api_key("sk-2")
        .build();

    let agent1 = AIAgent::new(config1);
    let agent2 = AIAgent::new(config2);

    let defs1 = agent1.get_tool_definitions();
    let defs2 = agent2.get_tool_definitions();

    // Both should have tools
    assert!(!defs1.is_empty());
    assert!(!defs2.is_empty());
    // Same config = same tools
    assert_eq!(defs1.len(), defs2.len());
}

// ── Test 11: Message types round-trip ──

#[test]
fn test_message_types_round_trip() {
    let sys = Message::system("system prompt");
    let user = Message::user("hello");
    let assistant = Message::assistant_text("hi there");
    let tool = Message::tool_result("call_1", "tool output");

    assert_eq!(sys.role, MessageRole::System);
    assert_eq!(user.role, MessageRole::User);
    assert_eq!(assistant.role, MessageRole::Assistant);
    assert_eq!(tool.role, MessageRole::Tool);

    if let Some(Content::Text(t)) = &user.content {
        assert_eq!(t, "hello");
    } else {
        panic!("expected text content");
    }
}

// ── Test 12: Compression estimator on long text ──

#[test]
fn test_compression_estimator_on_long_text() {
    use agent_core::compression::ContextCompressor;
    use agent_core::types::message::Message;

    let messages: Vec<Message> = (0..50)
        .map(|i| {
            Message::user(&format!(
                "message number {} with lots of text to fill tokens in the estimator",
                i
            ))
        })
        .collect();

    let tokens = ContextCompressor::estimate_tokens(&messages);
    assert!(tokens > 0, "Should estimate some tokens");
}

// ── Test 13: Prompt builder stable identity ──

#[test]
fn test_prompt_builder_stable_identity() {
    use agent_core::prompt::PromptBuilder;

    let parts1 = PromptBuilder::build_parts(None, None, None);
    let parts2 = PromptBuilder::build_parts(None, None, None);

    // Stable part should be identical across builds
    assert_eq!(parts1.stable, parts2.stable);
    // Contextual part should contain timestamp
    assert!(!parts1.contextual.is_empty());
}

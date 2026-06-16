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
    // T-3.9: api_key 是 SecretString，需要通过 api_key_expose() 读取
    assert_eq!(config.api_key_expose(), Some("test-key".to_string()));
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
fn test_config_debug_redacts_api_key() {
    let config = AgentConfigBuilder::new()
        .provider("openai")
        .model("gpt-4o")
        .api_key("sk-very-secret-value-123456")
        .build();
    let dbg = format!("{:?}", config);
    assert!(
        !dbg.contains("sk-very-secret-value-123456"),
        "Debug 输出不应包含 api_key 明文，得到: {}",
        dbg
    );
    assert!(
        dbg.contains("<redacted>"),
        "Debug 输出应出现 <redacted> 标记"
    );
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
fn test_profile_isolation_in_memory_tool() {
    // T-1.4: 验证 MemoryTool 真正按 profile_home 隔离，不再绕回
    // default_hermes_home()。
    use agent_core::tools::Tool;
    use agent_core::tools::memory_tool::Memory;
    use tempfile::TempDir;

    let dir_a = TempDir::new().unwrap();
    let dir_b = TempDir::new().unwrap();
    let mem_a = Memory::new(Some(dir_a.path().to_path_buf()));
    let mem_b = Memory::new(Some(dir_b.path().to_path_buf()));

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        mem_a
            .call(serde_json::json!({"action":"write","key":"memory","value":"alpha-only"}))
            .await
            .unwrap();
        let read_b = mem_b
            .call(serde_json::json!({"action":"read","key":"memory"}))
            .await
            .unwrap();
        assert!(
            !read_b.contains("alpha-only"),
            "profile B 不应看到 profile A 的记忆: {}",
            read_b
        );
        let read_a = mem_a
            .call(serde_json::json!({"action":"read","key":"memory"}))
            .await
            .unwrap();
        assert!(
            read_a.contains("alpha-only"),
            "profile A 应能读到自己写入的记忆: {}",
            read_a
        );
    });
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

// ─────────────────────────────────────────────────────────────────────────────
// v2 测试：SSRF 真防御 + 扩展 secret 脱敏
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_v2_ssrf_blocks_aws_metadata_ip_literal() {
    use agent_core::tools::web_tools::{is_private_or_local, is_url_safe};
    use std::net::{IpAddr, Ipv4Addr};
    let metadata = IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254));
    assert!(
        is_private_or_local(&metadata),
        "169.254.169.254 必须被识别为 link-local"
    );
    assert!(
        is_url_safe("http://169.254.169.254/latest/meta-data/").is_err(),
        "直接打 AWS metadata IP 必须被拦截"
    );
}

#[test]
fn test_v2_ssrf_blocks_ipv4_loopback_literal() {
    use agent_core::tools::web_tools::is_url_safe;
    assert!(is_url_safe("http://127.0.0.1/admin").is_err());
    assert!(is_url_safe("http://10.0.0.1/").is_err());
    assert!(is_url_safe("http://192.168.1.1/").is_err());
    assert!(is_url_safe("http://172.16.0.1/").is_err());
}

#[test]
fn test_v2_ssrf_blocks_ipv6_loopback() {
    use agent_core::tools::web_tools::{is_private_or_local, is_url_safe};
    use std::net::{IpAddr, Ipv6Addr};
    assert!(is_private_or_local(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
    assert!(
        is_url_safe("http://[::1]/admin").is_err(),
        "[::1] 必须被识别并拒绝"
    );
}

#[test]
fn test_v2_ssrf_blocks_ipv4_mapped_v6_private() {
    use agent_core::tools::web_tools::is_private_or_local;
    use std::net::{IpAddr, Ipv6Addr};
    // ::ffff:127.0.0.1
    let mapped = Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x7f00, 0x0001);
    assert!(
        is_private_or_local(&IpAddr::V6(mapped)),
        "IPv4-mapped 回环必须被识别"
    );
}

#[test]
fn test_v2_ssrf_blocks_unsupported_scheme() {
    use agent_core::tools::web_tools::is_url_safe;
    assert!(is_url_safe("file:///etc/passwd").is_err());
    assert!(is_url_safe("gopher://example.com/").is_err());
    assert!(is_url_safe("ftp://example.com/").is_err());
}

#[test]
fn test_v2_ssrf_blocks_localhost_hostname() {
    use agent_core::tools::web_tools::is_url_safe;
    assert!(is_url_safe("http://localhost/").is_err());
    assert!(is_url_safe("http://metadata.google.internal/").is_err());
    assert!(is_url_safe("http://foo.internal/").is_err());
    assert!(is_url_safe("http://bar.local/").is_err());
}

#[test]
fn test_v2_redact_anthropic_key() {
    use agent_core::tools::memory_tool::redact_secrets;
    let input = "Anthropic key: sk-ant-api03-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789";
    let out = redact_secrets(input);
    assert!(
        out.contains("<redacted-secret>"),
        "Anthropic 密钥未被脱敏: {}",
        out
    );
    assert!(!out.contains("AbCdEfGh"), "原始密钥内容仍出现: {}", out);
}

#[test]
fn test_v2_redact_openai_project_key() {
    use agent_core::tools::memory_tool::redact_secrets;
    let input = "OpenAI: sk-proj-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789";
    let out = redact_secrets(input);
    assert!(out.contains("<redacted-secret>"));
}

#[test]
fn test_v2_redact_github_pat() {
    use agent_core::tools::memory_tool::redact_secrets;
    let input = "GH: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
    let out = redact_secrets(input);
    assert!(out.contains("<redacted-secret>"));
}

#[test]
fn test_v2_redact_aws_access_key() {
    use agent_core::tools::memory_tool::redact_secrets;
    let input = "AWS: AKIAIOSFODNN7EXAMPLE";
    let out = redact_secrets(input);
    assert!(out.contains("<redacted-secret>"), "AWS key 未脱敏: {}", out);
    assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
}

#[test]
fn test_v2_redact_google_api_key() {
    use agent_core::tools::memory_tool::redact_secrets;
    let input = "Google: AIzaSyA-1234567890abcdefghijklmnopqrstuv";
    let out = redact_secrets(input);
    assert!(out.contains("<redacted-secret>"));
}

#[test]
fn test_v2_redact_jwt() {
    use agent_core::tools::memory_tool::redact_secrets;
    let input = "Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let out = redact_secrets(input);
    assert!(out.contains("<redacted-secret>"), "JWT 未脱敏: {}", out);
}

#[test]
fn test_v2_redact_private_key_block() {
    use agent_core::tools::memory_tool::redact_secrets;
    let pem = "Before\n-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEAxxxxxxxxxxxxxxx\nyyyyyyyyyyyyyyyyyyyyyyyyy\n-----END RSA PRIVATE KEY-----\nAfter";
    let out = redact_secrets(pem);
    assert!(
        out.contains("<redacted-private-key>"),
        "PEM 块未脱敏: {}",
        out
    );
    assert!(!out.contains("MIIEpAI"), "PEM 原内容仍出现: {}", out);
    assert!(out.contains("Before") && out.contains("After"));
}

#[test]
fn test_v2_redact_preserves_safe_text() {
    use agent_core::tools::memory_tool::redact_secrets;
    let safe = "用户偏好：中文回答，喜欢简洁。";
    assert_eq!(redact_secrets(safe), safe);
}


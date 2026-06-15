/// Hermes 兼容性测试
///
/// 测试策略：Hermes 生成测试数据（技能文件、记忆文件、会话SQLite），
/// Aether 读取并验证解析结果一致。
///
/// 需要 Hermes Agent 的安装路径，默认在 ../hermes/
use std::path::Path;

const HERMES_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../hermes");

// ── 基础存在性测试 ──

#[test]
fn test_hermes_source_exists() {
    let hermes = Path::new(HERMES_ROOT);
    if !hermes.exists() {
        eprintln!("Hermes dir not found, skipping (CI env)");
        return;
    }
    assert!(hermes.join("run_agent.py").exists());
    assert!(hermes.join("model_tools.py").exists());
}

// ── 技能格式兼容性 ──

#[test]
fn test_skill_format_compat() {
    let hermes_skills = Path::new(HERMES_ROOT).join("skills");
    if !hermes_skills.exists() {
        eprintln!("Hermes skills 目录不存在，跳过测试");
        return;
    }
    let skill_files = walkdir(&hermes_skills, 3);
    assert!(!skill_files.is_empty(), "Hermes skills 目录为空");

    // 检查至少有一个 SKILL.md 文件
    let skill_mds: Vec<_> = skill_files
        .iter()
        .filter(|p| p.file_name().map(|n| n == "SKILL.md").unwrap_or(false))
        .collect();
    assert!(!skill_mds.is_empty(), "未找到 SKILL.md 文件");

    // 每个 SKILL.md 必须有 frontmatter
    for path in &skill_mds {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            content.starts_with("---"),
            "SKILL.md {} 缺少 frontmatter",
            path.display()
        );
    }
}

#[test]
fn test_skill_parse_with_aether() {
    let hermes_skills = Path::new(HERMES_ROOT).join("skills");
    if !hermes_skills.exists() {
        return;
    }

    use agent_core::skills::FileSkillStore;
    for entry in walkdir(&hermes_skills, 3) {
        if entry.extension().and_then(|e| e.to_str()) == Some("md") {
            // Aether 应能解析 Hermes 的技能文件
            if let Ok(skill) = FileSkillStore::parse_skill_file(&entry) {
                assert!(!skill.name.is_empty(), "技能名称为空: {:?}", entry);
                assert!(
                    !skill.description.is_empty() || !skill.content.is_empty(),
                    "技能描述和内容同时为空: {:?}",
                    entry
                );
            }
        }
    }
}

// ── 消息格式兼容性 ──

#[test]
fn test_message_format_compat() {
    // OpenAI 格式的消息 → 应能正确解析
    let json = r#"{
        "role": "assistant",
        "content": "Hello!",
        "tool_calls": [{
            "id": "call_1",
            "type": "function",
            "function": {
                "name": "web_search",
                "arguments": "{\"query\":\"test\"}"
            }
        }]
    }"#;

    let msg: agent_core::types::message::Message =
        serde_json::from_str(json).expect("应能解析 OpenAI 格式的消息");

    assert_eq!(role_name(&msg.role), "assistant");
    assert!(msg.tool_calls.is_some());
    assert_eq!(msg.tool_calls.as_ref().unwrap().len(), 1);
}

#[test]
fn test_conversation_context_format() {
    // 验证 Hermes 风格的会话消息列表可以在 Aether 中解析
    let messages_json = r#"[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "Hello!"},
        {"role": "assistant", "content": "Hi! How can I help?"},
        {"role": "user", "content": "Search for rust"},
        {"role": "assistant", "content": null, "tool_calls": [{
            "id": "call_1",
            "type": "function",
            "function": {"name": "web_search", "arguments": "{\"query\":\"rust\"}"}
        }]},
        {"role": "tool", "content": "Rust is a systems language...", "tool_call_id": "call_1"}
    ]"#;

    let messages: Vec<agent_core::types::message::Message> =
        serde_json::from_str(messages_json).expect("应能解析 Hermes 消息列表");

    assert_eq!(messages.len(), 6);
    assert_eq!(role_name(&messages[0].role), "system");
    assert_eq!(role_name(&messages[3].role), "user");
    assert!(messages[4].tool_calls.is_some());
    assert_eq!(messages[5].tool_call_id.as_deref(), Some("call_1"));
}

// ── 记忆文件格式兼容性 ──

#[test]
fn test_memory_format_compat() {
    // Hermes 风格的 MEMORY.md 格式
    let memory_content =
        "# User Preferences\n\n- Prefers concise answers\n- Works with Rust\n- Likes dark mode";
    let temp_dir = std::env::temp_dir().join("aether_mem_test");
    let mem_dir = temp_dir.join("memory");
    std::fs::create_dir_all(&mem_dir).ok();
    std::fs::write(mem_dir.join("MEMORY.md"), memory_content).ok();

    use agent_core::memory::core::CoreMemory;
    let core = CoreMemory::new(&temp_dir);
    let content = core.read().unwrap();
    assert!(content.contains("Rust"), "应能读取 Hermes 风格的 MEMORY.md");
    assert!(content.contains("concise"), "应保留所有内容");

    std::fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_user_profile_format_compat() {
    // Hermes 风格的 USER.md 格式
    let profile_content = "# User Profile\n\nName: Developer\nTech Stack: Rust, TypeScript\n";
    let temp_dir = std::env::temp_dir().join("aether_profile_test");
    let mem_dir = temp_dir.join("memory");
    std::fs::create_dir_all(&mem_dir).ok();
    std::fs::write(mem_dir.join("USER.md"), profile_content).ok();

    use agent_core::memory::core::UserProfile;
    let profile = UserProfile::new(&temp_dir);
    let content = profile.read().unwrap();
    assert!(
        content.contains("Developer"),
        "应能读取 Hermes 风格的 USER.md"
    );

    std::fs::remove_dir_all(&temp_dir).ok();
}

// ── FTS5 查询兼容性 ──

#[test]
fn test_aether_skill_roundtrip() {
    // Aether 生成 → Aether 读取的一致性测试
    use agent_core::skills::FileSkillStore;

    let temp_dir = std::env::temp_dir().join("aether_skill_rt");
    std::fs::create_dir_all(&temp_dir).ok();

    let skill_content = "---\nname: test-skill\ndescription: A test skill\nversion: 1.0.0\n---\n\n# Test\n\nThis is a test skill.";
    let skill_path = temp_dir.join("test-skill.md");
    std::fs::write(&skill_path, skill_content).ok();

    let skill = FileSkillStore::parse_skill_file(&skill_path).unwrap();
    assert_eq!(skill.name, "test-skill");
    assert_eq!(skill.description, "A test skill");
    assert_eq!(skill.version, "1.0.0");

    std::fs::remove_dir_all(&temp_dir).ok();
}

// ── FTS5 搜索兼容性 ──

#[test]
fn test_session_schema_compat() {
    // Hermes 的 sessions 表结构应该被 Aether 兼容
    let temp_db = std::env::temp_dir().join("aether_schema_test.db");
    let _ = std::fs::remove_file(&temp_db);

    let store = agent_core::memory::state::SqliteSessionStore::new(&temp_db);
    let _ = std::fs::remove_file(&temp_db);

    // T-2.9: 删除恒真断言。验证真正创建成功
    match store {
        Ok(_) => {}
        Err(e) => panic!("SQLite schema 创建失败: {}", e),
    }
}

#[test]
fn test_message_format_roundtrip() {
    // Aether 序列化 → 反序列化一致性
    let msg = agent_core::types::message::Message::assistant_text("Hello, world!");
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: agent_core::types::message::Message = serde_json::from_str(&json).unwrap();
    if let Some(agent_core::types::message::Content::Text(t)) = &parsed.content {
        assert_eq!(t, "Hello, world!");
    } else {
        panic!("反序列化后 content 丢失");
    }
}

#[test]
fn test_error_format_compat() {
    // Hermes 风格的错误格式应该匹配
    use agent_core::AetherError;
    let e = AetherError::ToolNotFound("test_tool".to_string());
    let msg = e.to_string();
    assert!(msg.contains("TE001"), "错误应该有错误码");
    assert!(msg.contains("test_tool"), "错误应该包含工具名");
}

// ── 工具函数 ──

fn walkdir(dir: &Path, max_depth: u32) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = dir.read_dir() {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && max_depth > 0 {
                files.extend(walkdir(&path, max_depth - 1));
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files
}

/// 辅助：MessageRole → &str
fn role_name(r: &agent_core::types::message::MessageRole) -> &'static str {
    use agent_core::types::message::MessageRole::*;
    match r {
        System => "system",
        User => "user",
        Assistant => "assistant",
        Tool => "tool",
    }
}

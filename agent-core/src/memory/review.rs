//! Background Review — 每轮对话后自动审查并生成记忆/技能

use crate::error::AetherError;
use crate::memory::core::CoreMemory;
use crate::types::message::{Content, Message};
use std::path::Path;

/// Hermes 移植的记忆审查提示词
const MEMORY_REVIEW_PROMPT: &str = r#"
Review the conversation above and consider saving to memory if appropriate.

Focus on:
1. Has the user revealed things about themselves — their persona, desires, preferences, or personal details worth remembering?
2. Has the user expressed expectations about how you should behave, their work style, or ways they want you to operate?

If something stands out, save it using the memory tool.
If nothing is worth saving, just say 'Nothing to save.' and stop.
"#;

/// Hermes 移植的技能审查提示词
const SKILL_REVIEW_PROMPT: &str = r#"
Review the conversation above and update the skill library. Be ACTIVE — most sessions produce at least one skill update, even if small.

Signals to look for (any one of these warrants action):
  • User corrected your style, tone, format, legibility, or verbosity.
  • User corrected your workflow, approach, or sequence of steps.
  • Non-trivial technique, fix, workaround, debugging path, or tool-usage pattern emerged.
  • A skill that got loaded or consulted this session turned out to be wrong or outdated.

Preference order:
  1. UPDATE A CURRENTLY-LOADED SKILL.
  2. UPDATE AN EXISTING SKILL.
  3. CREATE A NEW SKILL.
"#;

/// 审查条件：是否需要触发审查
pub fn should_review(messages: &[Message], tool_call_count: usize) -> bool {
    if messages.len() < 3 {
        return false;
    }
    // 工具调用 > 5 次
    if tool_call_count > 5 {
        return true;
    }
    // 用户纠正过（含有"不要"、"错了"等关键词）
    let correction_words = [
        "不要",
        "错了",
        "不对",
        "stop",
        "don't",
        "wrong",
        "incorrect",
        "fix",
    ];
    for msg in messages.iter().rev().take(3) {
        if let Some(Content::Text(t)) = &msg.content {
            let lower = t.to_lowercase();
            if correction_words.iter().any(|w| lower.contains(w)) {
                return true;
            }
        }
    }
    false
}

/// 执行审查并写入记忆/技能
pub async fn review_and_learn(
    messages: &[Message],
    tool_call_count: usize,
    hermes_home: &Path,
    model: &dyn crate::llm::ChatModel,
) -> Result<(), AetherError> {
    if !should_review(messages, tool_call_count) {
        return Ok(());
    }

    let conversation_text: String = messages
        .iter()
        .filter_map(|m| match &m.content {
            Some(Content::Text(t)) => Some(t.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    // 记忆审查
    let memory_prompt = format!("{}\n\n---\n\n{}", conversation_text, MEMORY_REVIEW_PROMPT);
    let memory_msg = Message::user(&memory_prompt);
    if let Ok(resp) = model.invoke(&[memory_msg], &[]).await {
        if let Some(text) = resp.content {
            if !text.contains("Nothing to save") {
                let core_memory = CoreMemory::new(hermes_home);
                if let Ok(existing) = core_memory.read() {
                    let updated = format!("{}\n- {}\n", existing, text);
                    core_memory.write(&updated).ok();
                }
            }
        }
    }

    // 技能审查
    let skill_prompt = format!("{}\n\n---\n\n{}", conversation_text, SKILL_REVIEW_PROMPT);
    let skill_msg = Message::user(&skill_prompt);
    if let Ok(resp) = model.invoke(&[skill_msg], &[]).await {
        if let Some(text) = resp.content {
            if !text.contains("Nothing to save") && !text.contains("no update") {
                let skills_dir = hermes_home.join("skills");
                std::fs::create_dir_all(&skills_dir).ok();
                let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let skill_path = skills_dir.join(format!("learned_{}.md", ts));
                // T-3.4: 修复skill命名碰撞 — 每个技能用唯一时间戳命名
                let skill_name = format!("review-{}", ts);
                let content = format!(
                    "---\nname: {}\ndescription: 自动从对话学到的技能\nauthor: Aether\n---\n\n{}",
                    skill_name, text
                );
                std::fs::write(&skill_path, &content).ok();
            }
        }
    }

    Ok(())
}

use crate::types::message::Message;

/// 系统提示词组装器
/// T-2.7: 真三层 — stable层不含Local::now()(可cache), contextual含时间+环境(每turn刷新)
pub struct PromptBuilder;

impl PromptBuilder {
    /// 构建完整的系统提示词（三层）
    pub fn build(identity: Option<&str>, context: Option<&str>, dynamic: Option<&str>) -> String {
        let mut parts = Vec::new();

        // 稳定层：身份定义。不含 Local::now()/UUID 等 per-session 变化内容 — 可被 Anthropic cache
        if let Some(id) = identity {
            parts.push(id.to_string());
        } else {
            parts.push(Self::stable_identity());
        }

        // 上下文层：每 turn 变化（时间、目录、记忆等）
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut ctx_block = format!("当前时间: {}", time);
        if let Some(ctx) = context {
            if !ctx.is_empty() {
                ctx_block.push_str("\n");
                ctx_block.push_str(ctx);
            }
        }
        parts.push(format!("\n<cached-context>\n{}\n</cached-context>", ctx_block));

        // 易变层：按需注入
        if let Some(dyn_content) = dynamic {
            if !dyn_content.is_empty() {
                parts.push(format!("\n<volatile>\n{}\n</volatile>", dyn_content));
            }
        }

        parts.join("\n")
    }

    pub fn build_system_message(
        identity: Option<&str>, context: Option<&str>, dynamic: Option<&str>,
    ) -> Message {
        Message::system(Self::build(identity, context, dynamic))
    }

    /// Stable identity — no Local::now(), no UUID, no per-turn variation.
    /// T-2.7: This is the only layer eligible for Anthropic prompt caching.
    fn stable_identity() -> String {
        "你是 Aether，一个智能的 AI 助手。你可以使用各种工具来帮助用户完成任务。".to_string()
    }
}

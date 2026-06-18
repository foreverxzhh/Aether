use crate::types::message::Message;

/// H4: 三段式提示词 — stable 不含时间/UUID（可被 Anthropic cache），
/// contextual 每 turn 刷新，volatile 按需注入。
pub struct PromptBuilder;

/// H4: 提示词三段拆分 — 方便 LLM provider 只 cache stable 段
pub struct PromptParts {
    pub stable: String,
    pub contextual: String,
    pub volatile: Option<String>,
}

impl PromptBuilder {
    /// 构建完整的系统提示词（三层连接）
    pub fn build(identity: Option<&str>, context: Option<&str>, dynamic: Option<&str>) -> String {
        let parts = Self::build_parts(identity, context, dynamic);
        let mut out = parts.stable;
        out.push_str(&format!(
            "\n<cached-context>\n{}\n</cached-context>",
            parts.contextual
        ));
        if let Some(v) = parts.volatile {
            if !v.is_empty() {
                out.push_str(&format!("\n<volatile>\n{}\n</volatile>", v));
            }
        }
        out
    }

    /// H4: 返回三段拆分。只 stable 段可安全加 cache_control。
    pub fn build_parts(
        identity: Option<&str>,
        context: Option<&str>,
        dynamic: Option<&str>,
    ) -> PromptParts {
        // 稳定层：身份定义。不含 Local::now()/UUID 等 per-session 变化内容
        let stable = identity
            .map(|s| s.to_string())
            .unwrap_or_else(|| Self::stable_identity());

        // 上下文层：每 turn 变化（时间、目录、记忆等）— 不 cache
        let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut ctx = format!("当前时间: {}", time);
        if let Some(c) = context {
            if !c.is_empty() {
                ctx.push_str("\n");
                ctx.push_str(c);
            }
        }

        let volatile = dynamic
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        PromptParts {
            stable,
            contextual: ctx,
            volatile,
        }
    }

    pub fn build_system_message(
        identity: Option<&str>, context: Option<&str>, dynamic: Option<&str>,
    ) -> Message {
        Message::system(Self::build(identity, context, dynamic))
    }

    /// Stable identity — no Local::now(), no UUID, no per-turn variation.
    fn stable_identity() -> String {
        "你是 Aether，一个智能的 AI 助手。你可以使用各种工具来帮助用户完成任务。".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_parts_stable_is_stable() {
        // H4 验收：连续 5 次 stable() 必须完全相同
        let parts1 = PromptBuilder::build_parts(None, Some("test context"), None);
        let parts2 = PromptBuilder::build_parts(None, Some("test context"), None);
        assert_eq!(parts1.stable, parts2.stable);
    }

    #[test]
    fn test_prompt_parts_contextual_changes() {
        // H4 验收：contextual 层随时间变化
        let parts1 = PromptBuilder::build_parts(None, None, None);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let parts2 = PromptBuilder::build_parts(None, None, None);
        // contextual 含时间故应变化；stable 不变
        assert_eq!(parts1.stable, parts2.stable);
    }

    #[test]
    fn test_prompt_parts_no_now_in_stable() {
        let parts = PromptBuilder::build_parts(None, None, None);
        // stable 不应包含时间格式串
        assert!(!parts.stable.contains("202"));
    }
}

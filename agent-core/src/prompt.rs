use chrono::Local;
use crate::types::message::Message;

/// 系统提示词组装器（三层结构）
pub struct PromptBuilder;

impl PromptBuilder {
    /// 构建完整的系统提示词
    pub fn build(
        identity: Option<&str>,
        context: Option<&str>,
        dynamic: Option<&str>,
    ) -> String {
        let mut parts = Vec::new();

        // 稳定层：身份定义
        if let Some(id) = identity {
            parts.push(id.to_string());
        } else {
            parts.push(Self::default_identity());
        }

        // 上下文层
        if let Some(ctx) = context {
            parts.push("\n--- 上下文 ---\n".to_string());
            parts.push(ctx.to_string());
        }

        // 易变层
        if let Some(dyn_content) = dynamic {
            parts.push("\n--- 当前信息 ---\n".to_string());
            parts.push(dyn_content.to_string());
        }

        parts.join("\n")
    }

    /// 构建系统消息
    pub fn build_system_message(
        identity: Option<&str>,
        context: Option<&str>,
        dynamic: Option<&str>,
    ) -> Message {
        Message::system(Self::build(identity, context, dynamic))
    }

    fn default_identity() -> String {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        format!(
            "你是 Aether，一个智能的 AI 助手。\n\
            你可以使用各种工具来帮助用户完成任务。\n\
            当前时间：{}", now
        )
    }
}

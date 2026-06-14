use crate::error::AetherError;
use crate::types::message::{Content, Message, MessageRole};

/// 上下文压缩结果
pub struct CompressionResult {
    pub compressed_count: u32,
    pub summary: String,
    pub child_session_id: String,
    pub original_message_count: usize,
}

/// 上下文压缩器
pub struct ContextCompressor;

impl ContextCompressor {
    /// 估算消息 token 数（简易版，1 token ≈ 2 字符）
    pub fn estimate_tokens(messages: &[Message]) -> u32 {
        messages.iter().map(|m| {
            let text_len = match &m.content {
                Some(Content::Text(t)) => t.len() as u32 / 2,
                _ => 0,
            };
            let tool_len = m.tool_calls.as_ref()
                .map(|tc| serde_json::to_string(tc).unwrap_or_default().len() as u32 / 2)
                .unwrap_or(0);
            text_len + tool_len + 10
        }).sum()
    }

    /// 判断是否需要压缩（超过上下文窗口的 75%）
    pub fn needs_compression(messages: &[Message], context_limit: u32) -> bool {
        Self::estimate_tokens(messages) > (context_limit as f64 * 0.75) as u32
    }

    /// 执行压缩：保留头部+尾部，用 LLM 摘要替换中间部分
    pub async fn compress(
        messages: &[Message],
        context_limit: u32,
        model: &dyn crate::llm::ChatModel,
    ) -> Result<CompressionResult, AetherError> {
        if messages.is_empty() {
            return Err(AetherError::ContextOverflow("空消息无法压缩".into()));
        }

        // 头尾保护：保留前2条(system+首条user)和后3条(最近对话)
        let head_count = 2.min(messages.len());
        let tail_count = 3.min(messages.len().saturating_sub(head_count));

        let head = &messages[..head_count];
        let tail = &messages[messages.len() - tail_count..];
        let middle = &messages[head_count..messages.len() - tail_count];

        // 中间部分做摘要
        let summary = if !middle.is_empty() {
            let middle_text: Vec<String> = middle.iter().map(|m| {
                let role = match m.role {
                    MessageRole::User => "用户",
                    MessageRole::Assistant => "助手",
                    MessageRole::System => "系统",
                    MessageRole::Tool => "工具",
                };
                let text = match &m.content {
                    Some(Content::Text(t)) => t.clone(),
                    _ => String::new(),
                };
                format!("{}: {}", role, text)
            }).collect();

            let prompt = format!(
                "请为以下对话生成简洁的摘要（保留关键信息和决定），不超过200字：\n\n{}",
                middle_text.join("\n")
            );

            let summary_msg = Message::user(&prompt);
            match model.invoke(&[summary_msg], &[]).await {
                Ok(resp) => resp.content.unwrap_or_else(|| "[摘要生成失败]".to_string()),
                Err(_) => "[摘要生成失败]".to_string(),
            }
        } else {
            String::new()
        };

        // 组装压缩后的消息
        let mut compressed: Vec<Message> = Vec::new();
        compressed.extend_from_slice(head);
        if !summary.is_empty() {
            compressed.push(Message::system(format!(
                "[以下是中间对话的摘要]\n{}", summary
            )));
        }
        compressed.extend_from_slice(tail);

        Ok(CompressionResult {
            compressed_count: compressed.len() as u32,
            summary,
            child_session_id: uuid::Uuid::new_v4().to_string(),
            original_message_count: messages.len(),
        })
    }
}

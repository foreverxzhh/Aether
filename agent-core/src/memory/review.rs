//! Background Review（TODO: Phase 6 实现完整学习闭环）
use crate::error::AetherError;

/// 审查一次对话并生成记忆/技能
pub async fn review_conversation(
    _messages: &[crate::types::message::Message],
    _hermes_home: &std::path::Path,
) -> Result<(), AetherError> {
    // TODO: 调用辅助 LLM 审查并生成技能/记忆
    Ok(())
}

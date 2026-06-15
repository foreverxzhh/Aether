//! T-1.6: delegate_batch deleted (was format!("[task done]") stub).
//! delegate() kept as subquery_llm — a one-shot Q&A with auxiliary model.
//! No tools, no ReAct cycle. Real sub-agent delegation → T-3.6.

use crate::error::AetherError;
use crate::types::message::Message;

/// One-shot Q&A with a secondary model. NOT a sub-agent — no tools, no loop, no memory.
/// Real sub-agent delegation → FIX_PLAN T-3.6.
pub async fn subquery_llm(
    model: &dyn crate::llm::ChatModel,
    goal: &str,
    context: Option<&str>,
) -> Result<String, AetherError> {
    let mut messages = vec![Message::system("You are a focused task executor.")];
    if let Some(ctx) = context {
        messages.push(Message::system(ctx));
    }
    messages.push(Message::user(goal));
    let response = model.invoke(&messages, &[]).await?;
    Ok(response.content.unwrap_or_else(|| "[No response]".to_string()))
}

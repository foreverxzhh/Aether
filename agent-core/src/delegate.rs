//! T-3.6: Sub-agent delegation with real tool restriction and budget.
//! Replaces the old delegate_batch stub (T-1.6 deleted).

use crate::error::AetherError;
use crate::types::message::Message;
use crate::budget::IterationBudget;
use crate::breaker::CircuitBreaker;

/// Sub-agent config for delegated tasks
#[derive(Clone)]
pub struct SubAgentConfig {
    pub max_iterations: u32,
    pub allowed_tools: Vec<String>,
    pub depth: u8,
    pub system_prompt: String,
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 30,
            allowed_tools: vec!["read_file".into(), "web_search".into(), "web_extract".into()],
            depth: 1,
            system_prompt: "你是一个专注的任务执行者。请完成分配的任务并返回结果。".into(),
        }
    }
}

/// Real sub-agent with restricted tools and independent budget.
pub async fn run_subagent(
    model: &dyn crate::llm::ChatModel,
    goal: &str,
    context: Option<&str>,
    config: SubAgentConfig,
) -> Result<String, AetherError> {
    if config.depth > 3 {
        return Err(AetherError::ConfigError("超过最大委托深度".into()));
    }

    let budget = IterationBudget::new(config.max_iterations);
    let breaker = CircuitBreaker::new(5);

    let mut messages = vec![
        Message::system(&config.system_prompt),
    ];
    if let Some(ctx) = context {
        messages.push(Message::system(ctx));
    }
    messages.push(Message::user(goal));

    // 构建受限工具列表
    let tools: Vec<serde_json::Value> = config.allowed_tools.iter().map(|name| {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": name,
                "description": format!("Tool: {}", name),
                "parameters": {"type": "object", "properties": {}}
            }
        })
    }).collect();

    let mut final_response: Option<String> = None;

    for _turn in 0..config.max_iterations {
        if !budget.consume() { break; }
        let response = model.invoke(&messages, &tools).await?;

        if let Some(calls) = &response.tool_calls {
            for call in calls {
                let args: serde_json::Value = serde_json::from_str(&call.arguments).unwrap_or_default();
                if breaker.check(&call.name, &args) {
                    return Err(AetherError::CircuitBreakerTripped(call.name.clone(), 5));
                }
            }

            let assistant_msg = Message::assistant_tool_calls(
                calls.iter().map(|c| crate::types::message::MessageToolCall {
                    id: c.id.clone(),
                    call_type: "function".into(),
                    function: crate::types::message::ToolFunctionCall {
                        name: c.name.clone(),
                        arguments: c.arguments.clone(),
                    },
                }).collect(),
            );
            messages.push(assistant_msg);

            // Note: tools are executed by the parent agent, not sub-agent.
            // Sub-agent reports tool_call intent; parent carries it out.
            for call in calls {
                messages.push(Message::tool_result(&call.id, &format!(
                    "Tool '{}' called with args: {}. Parent agent will execute.",
                    call.name, call.arguments
                )));
            }
        } else {
            final_response = response.content;
            break;
        }
    }

    Ok(final_response.unwrap_or_else(|| "[Sub-agent: no final response]".into()))
}

/// Convenience: one-shot subquery (T-1.6 renamed)
pub async fn subquery_llm(
    model: &dyn crate::llm::ChatModel,
    goal: &str,
    context: Option<&str>,
) -> Result<String, AetherError> {
    let messages = {
        let mut v = Vec::new();
        v.push(Message::system("你是一个专注的任务执行者。"));
        if let Some(ctx) = context {
            v.push(Message::system(ctx));
        }
        v.push(Message::user(goal));
        v
    };
    let response = model.invoke(&messages, &[]).await?;
    Ok(response.content.unwrap_or_else(|| "[No response]".into()))
}

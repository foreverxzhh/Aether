use tracing::{info, warn};
use crate::agent::AIAgent;
use crate::breaker::CircuitBreaker;
use crate::budget::IterationBudget;
use crate::error::AetherError;
use crate::prompt::PromptBuilder;
use crate::types::message::Message;
use crate::types::model::TurnResult;

/// 运行一轮对话（ReAct 循环）
pub async fn run_conversation(
    agent: &AIAgent,
    user_message: &str,
) -> Result<TurnResult, AetherError> {
    let model = agent.model()
        .ok_or_else(|| AetherError::ConfigError("模型未配置".to_string()))?;

    let max_iterations = agent.config.max_iterations;
    let budget = IterationBudget::new(max_iterations);
    let breaker = CircuitBreaker::new(5);
    let mut turn_count = 0u32;

    // ── 组装消息 ──
    let system_msg = PromptBuilder::build_system_message(
        agent.config.system_prompt.as_deref(),
        None,
        None,
    );

    let user_msg = Message::user(user_message);
    let mut messages = vec![system_msg, user_msg];

    // ── 构建工具列表 ──
    let tools = agent.get_tool_definitions();

    let mut final_response: Option<String> = None;
    let mut all_tool_results = Vec::new();

    // ── 主循环 ──
    while turn_count < max_iterations && budget.remaining() > 0 {
        turn_count += 1;

        info!(
            turn = turn_count,
            messages = messages.len(),
            budget_remaining = budget.remaining(),
            "LLM 调用"
        );

        if !budget.consume() {
            warn!("迭代预算耗尽");
            break;
        }

        // 调用 LLM
        let response = model.invoke(&messages, &tools).await?;

        // 处理工具调用
        if let Some(calls) = &response.tool_calls {
            // 熔断检查
            for call in calls {
                let args: serde_json::Value =
                    serde_json::from_str(&call.arguments).unwrap_or(serde_json::json!({}));
                if breaker.check(&call.name, &args) {
                    return Err(AetherError::CircuitBreakerTripped(
                        call.name.clone(),
                        5,
                    ));
                }
            }

            // 添加 assistant 消息（含 tool_calls）
            let assistant_msg = Message::assistant_tool_calls(
                calls
                    .iter()
                    .map(|c| crate::types::message::MessageToolCall {
                        id: c.id.clone(),
                        call_type: "function".to_string(),
                        function: crate::types::message::ToolFunctionCall {
                            name: c.name.clone(),
                            arguments: c.arguments.clone(),
                        },
                    })
                    .collect(),
            );
            messages.push(assistant_msg);

            // 执行工具
            for call in calls {
                let args: serde_json::Value =
                    serde_json::from_str(&call.arguments).unwrap_or(serde_json::json!({}));

                info!(tool = call.name, "执行工具");
                let result = agent.execute_tool(&call.name, args).await;

                match result {
                    Ok(output) => {
                        info!(tool = call.name, "工具执行成功");
                        let tool_msg = Message::tool_result(&call.id, &output);
                        messages.push(tool_msg);
                        all_tool_results.push(call.clone());
                    }
                    Err(e) => {
                        warn!(tool = call.name, error = %e, "工具执行失败");
                        let err_msg = Message::tool_result(&call.id, &format!("错误: {}", e));
                        messages.push(err_msg);
                    }
                }
            }

            // 压缩上下文（如果消息太多）
            if messages.len() > 50 {
                // TODO: T067-T070 上下文压缩
                warn!("消息数超过 50，需要上下文压缩（尚未实现）");
            }
        } else {
            // 没有工具调用 → 这就是最终回复
            final_response = response.content;
            if final_response.is_some() {
                messages.push(Message::assistant_text(
                    final_response.as_ref().unwrap(),
                ));
            }
            break;
        }

        // Budget 耗尽时的优雅处理
        if budget.remaining() == 0 && final_response.is_none() {
            warn!("Budget 耗尽，请求模型总结");
            let summarize_msg = Message::user(
                "你的迭代预算已用尽。请总结你目前完成的工作，然后结束。",
            );
            messages.push(summarize_msg);

            let summary = model.invoke(&messages, &[]).await?;
            final_response = summary.content;
            if let Some(ref text) = final_response {
                messages.push(Message::assistant_text(text));
            }
            break;
        }
    }

    if final_response.is_none() {
        final_response = Some("已达到最大迭代次数，对话结束。".to_string());
    }

    Ok(TurnResult {
        final_response,
        messages,
        tool_results: all_tool_results,
        usage: None,
        turn_count,
    })
}

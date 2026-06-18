use crate::agent::AIAgent;
use crate::breaker::CircuitBreaker;
use crate::budget::IterationBudget;
use crate::error::AetherError;
use crate::prompt::PromptBuilder;
use crate::types::message::Message;
use crate::types::model::TurnResult;
use std::time::Duration;
use tracing::{info, warn};

/// 判断错误是否可重试
fn is_retryable(err: &AetherError) -> bool {
    matches!(
        err,
        AetherError::LlmError(_) | AetherError::LlmEmptyResponse | AetherError::LlmParseError(_)
    )
}

/// 运行一轮对话（ReAct 循环）
pub async fn run_conversation(
    agent: &AIAgent,
    user_message: &str,
) -> Result<TurnResult, AetherError> {
    let model = agent
        .model()
        .ok_or_else(|| AetherError::ConfigError("模型未配置".to_string()))?;

    let max_iterations = agent.config.max_iterations;
    let budget = IterationBudget::new(max_iterations);
    let breaker = CircuitBreaker::new(5);
    let mut turn_count = 0u32;

    // ── 组装消息 ──
    // T-2.5: 消费 compression_enabled 配置（为 false 时跳过压缩）
    let compression_enabled = agent.config.compression_enabled;

    let cwd = std::env::current_dir().ok();
    let cwd_str = cwd.as_ref().map(|p| p.to_string_lossy().to_string());
    // T-2.5: 消费 memory_enabled 配置（为 false 时不加载记忆上下文）
    let context_text = if agent.config.memory_enabled {
        crate::context::ContextEngine::collect_context(cwd_str.as_deref())
    } else { String::new() };
    let context_ref = if context_text.is_empty() {
        None
    } else {
        Some(context_text.as_str())
    };

    let system_msg = PromptBuilder::build_system_message(
        agent.config.system_prompt.as_deref(),
        context_ref,
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

        // 调用 LLM（带重试）
        let response = {
            let mut last_err = None;
            let mut response = None;
            for attempt in 0..3 {
                match model.invoke(&messages, &tools).await {
                    Ok(r) => {
                        response = Some(r);
                        break;
                    }
                    Err(e) if is_retryable(&e) => {
                        warn!(attempt = attempt + 1, error = %e, "LLM 调用失败，重试中");
                        last_err = Some(e);
                        // 简单随机抖动: 避免雷群效应
                        let jitter_ms = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.subsec_millis() as u64 % 150)
                            .unwrap_or(50);
                        tokio::time::sleep(Duration::from_millis(
                            500 * 2u64.pow(attempt) + jitter_ms,
                        ))
                        .await;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            response.ok_or_else(|| {
                last_err.unwrap_or(AetherError::LlmError("LLM 调用重试耗尽".to_string()))
            })?
        };

        // 处理工具调用
        if let Some(calls) = response.tool_calls.clone() {
            // 熔断检查
            for call in &calls {
                let args = serde_json::from_str(&call.arguments).unwrap_or_default();
                if breaker.check(&call.name, &args) {
                    return Err(AetherError::CircuitBreakerTripped(call.name.clone(), 5));
                }
            }

            // 添加 assistant 消息
            let call_msgs: Vec<_> = calls
                .iter()
                .map(|c| crate::types::message::MessageToolCall {
                    id: c.id.clone(),
                    call_type: "function".to_string(),
                    function: crate::types::message::ToolFunctionCall {
                        name: c.name.clone(),
                        arguments: c.arguments.clone(),
                    },
                })
                .collect();
            messages.push(Message::assistant_tool_calls(call_msgs));

            // 并行执行工具
            let mut tool_futures = Vec::new();
            let calls_owned = calls.clone();
            for call in calls_owned {
                let name = call.name;
                let cid = call.id;
                let args = serde_json::from_str(&call.arguments).unwrap_or_default();
                let registry = agent.tools.clone();
                tool_futures.push(tokio::spawn(async move {
                    let result = registry.read().await.execute(&name, args).await;
                    (cid, name, result)
                }));
            }
            for r in futures::future::join_all(tool_futures).await {
                match r {
                    Ok((id, name, Ok(out))) => {
                        info!(tool = %name, "工具执行成功");
                        messages.push(Message::tool_result(&id, &out));
                    }
                    Ok((id, name, Err(e))) => {
                        warn!(tool = %name, error = %e, "工具执行失败");
                        messages.push(Message::tool_result(&id, &format!("错误: {}", e)));
                    }
                    Err(e) => warn!(error = %e, "工具线程崩溃"),
                }
            }

            all_tool_results.extend(calls);

            // T-2.5: 消费 compression_enabled（为 false 时跳过）
            if compression_enabled && messages.len() > 10 {
                let current_tokens =
                    crate::compression::ContextCompressor::estimate_tokens(&messages);
                // H6: 用 config.compression_threshold_ratio 替代硬编码
                let threshold =
                    (128000.0 * agent.config.compression_threshold_ratio) as u32;
                if current_tokens > threshold {
                    info!(current_tokens, "触发上下文压缩");
                    match crate::compression::ContextCompressor::compress(&messages, 128000, model)
                        .await
                    {
                        Ok(result) => {
                            info!(
                                original = result.original_message_count,
                                compressed = result.compressed_count,
                                "压缩完成 — 替换对话历史"
                            );
                            // T-2.2: 真正替换消息
                            messages = result.compressed_messages;
                            // 退还一次迭代预算作为补偿
                            budget.refund();
                        }
                        Err(e) => warn!(error = %e, "压缩失败"),
                    }
                }
            }
        } else {
            // 没有工具调用 → 这就是最终回复
            final_response = response.content;
            if final_response.is_some() {
                messages.push(Message::assistant_text(final_response.as_ref().unwrap()));
            }
            break;
        }

        // Budget 耗尽时的优雅处理
        if budget.remaining() == 0 && final_response.is_none() {
            warn!("Budget 耗尽，请求模型总结");
            let summarize_msg =
                Message::user("你的迭代预算已用尽。请总结你目前完成的工作，然后结束。");
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

//! T-3.6: Sub-agent delegation with real tool restriction and budget.
//! Replaces the old delegate_batch stub (T-1.6 deleted).

use crate::error::AetherError;
use crate::llm::ChatModel;
use crate::tools::{Tool, ToolRegistry};
use crate::types::message::Message;
use crate::budget::IterationBudget;
use crate::breaker::CircuitBreaker;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Sub-agent config for delegated tasks
#[derive(Clone)]
pub struct SubAgentConfig {
    pub max_iterations: u32,
    pub allowed_tools: Vec<String>,
    pub depth: u32,
    pub max_depth: u32,
    pub system_prompt: String,
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 30,
            allowed_tools: vec!["read_file".into(), "web_search".into(), "web_extract".into()],
            depth: 1,
            max_depth: 2,
            system_prompt: "你是一个专注的任务执行者。请完成分配的任务并返回结果。".into(),
        }
    }
}

/// Real sub-agent with restricted tools and independent budget.
///
/// `registry` 可选：若为 `Some`，子 agent 真正调用工具；若为 `None`，仅做 LLM 单轮推理。
pub async fn run_subagent(
    model: &dyn ChatModel,
    goal: &str,
    context: Option<&str>,
    config: SubAgentConfig,
    registry: Option<&ToolRegistry>,
) -> Result<String, AetherError> {
    if config.depth > config.max_depth {
        return Err(AetherError::MaxSpawnDepthExceeded(config.max_depth));
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

    // 构建受限工具定义：从 registry 中查找真实定义；若没有 registry，则用占位 schema
    let all_defs: Vec<Value> = registry.map(|r| r.get_definitions()).unwrap_or_default();
    let tools: Vec<Value> = config
        .allowed_tools
        .iter()
        .map(|name| {
            if let Some(def) = all_defs.iter().find(|d| {
                d.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    == Some(name.as_str())
            }) {
                def.clone()
            } else {
                json!({
                    "type": "function",
                    "function": {
                        "name": name,
                        "description": format!("Tool: {}", name),
                        "parameters": {"type": "object", "properties": {}}
                    }
                })
            }
        })
        .collect();

    let mut final_response: Option<String> = None;

    for _turn in 0..config.max_iterations {
        if !budget.consume() { break; }
        let response = model.invoke(&messages, &tools).await?;

        if let Some(calls) = &response.tool_calls {
            for call in calls {
                let args: Value = serde_json::from_str(&call.arguments).unwrap_or_default();
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

            // T-3.6: 真执行工具（受 allowed_tools 限制）
            for call in calls {
                if !config.allowed_tools.contains(&call.name) {
                    messages.push(Message::tool_result(
                        &call.id,
                        &format!("拒绝执行: 工具 '{}' 不在子 agent 允许列表内", call.name),
                    ));
                    continue;
                }
                let args: Value = serde_json::from_str(&call.arguments).unwrap_or_default();
                let result_str = match registry {
                    Some(r) => match r.execute(&call.name, args).await {
                        Ok(s) => s,
                        Err(e) => format!("[工具错误] {}", e),
                    },
                    None => format!(
                        "[未注入 registry] Tool '{}' called with args: {}",
                        call.name, call.arguments
                    ),
                };
                messages.push(Message::tool_result(&call.id, &result_str));
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
    model: &dyn ChatModel,
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

/// T-3.6: Delegate Tool — LLM 通过 tool-call 触发子 agent
///
/// 持有父 agent 的 model + registry + 深度。
/// 子 agent 创建时深度 +1；超过 `max_depth` 即拒绝。
pub struct Delegate {
    parent_model: Arc<dyn ChatModel>,
    registry: Arc<RwLock<ToolRegistry>>,
    max_iterations: u32,
    max_depth: u32,
    current_depth: u32,
}

impl Delegate {
    pub fn new(
        model: Arc<dyn ChatModel>,
        registry: Arc<RwLock<ToolRegistry>>,
        max_iterations: u32,
        max_depth: u32,
    ) -> Self {
        Self {
            parent_model: model,
            registry,
            max_iterations,
            max_depth,
            current_depth: 0,
        }
    }
}

#[async_trait]
impl Tool for Delegate {
    fn name(&self) -> &str {
        "delegate"
    }
    fn toolset(&self) -> &str {
        "delegate"
    }
    fn description(&self) -> &str {
        "Delegate a sub-task to a child agent with a restricted tool subset"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "goal": {
                    "type": "string",
                    "description": "What the sub-agent should accomplish"
                },
                "allowed_tools": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Tool names the sub-agent may use (subset of parent tools)"
                },
                "context": {
                    "type": "string",
                    "description": "Optional context to pass to the sub-agent"
                }
            },
            "required": ["goal"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let next_depth = self.current_depth + 1;
        if next_depth > self.max_depth {
            return Err(AetherError::MaxSpawnDepthExceeded(self.max_depth));
        }
        let goal = args
            .get("goal")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AetherError::ToolInvalidArgs("缺少 goal 参数".into()))?
            .to_string();
        let context = args
            .get("context")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let default_tools: Vec<String> = vec![
            "read_file".into(),
            "search_files".into(),
            "web_search".into(),
            "web_extract".into(),
        ];
        let allowed = args
            .get("allowed_tools")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or(default_tools);

        let cfg = SubAgentConfig {
            max_iterations: self.max_iterations,
            allowed_tools: allowed,
            depth: next_depth,
            max_depth: self.max_depth,
            system_prompt: "你是一个专注的任务执行者。请完成分配的任务并返回结果。".into(),
        };

        // 在 read guard 内运行整个子 agent；ToolRegistry::execute 只取读锁，
        // tokio RwLock 允许多重读，所以即使父 agent 还持有读锁也不会死锁。
        let guard = self.registry.read().await;
        let out = run_subagent(
            self.parent_model.as_ref(),
            &goal,
            context.as_deref(),
            cfg,
            Some(&*guard),
        )
        .await?;
        Ok(json!({ "result": out }).to_string())
    }
}

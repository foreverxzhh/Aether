use crate::types::message::Message;
use serde::{Deserialize, Serialize};

/// LLM 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallInfo>>,
    pub finish_reason: FinishReason,
    pub usage: Option<TokenUsage>,
}

/// 工具调用信息（从模型响应中提取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub arguments: String, // JSON 字符串
}

/// 结束原因
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    Error,
    Interrupted,
    BudgetExhausted,
    Other(String),
}

impl FinishReason {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "stop" => FinishReason::Stop,
            "length" | "max_tokens" => FinishReason::Length,
            "tool_calls" => FinishReason::ToolCalls,
            _ => FinishReason::Other(s.to_string()),
        }
    }
}

/// Token 用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<u32>,
}

/// 流式响应 Chunk
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub delta: String,
    pub tool_calls: Option<Vec<ToolCallInfo>>,
    pub finish_reason: Option<FinishReason>,
    pub usage: Option<TokenUsage>,
}

/// R-1.1: 流式事件枚举 — chat_stream 的标准化输出
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 文本增量
    Text(String),
    /// LLM 请求调用工具
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    /// 工具执行结果
    ToolResult {
        tool_call_id: String,
        result: String,
    },
    /// 最终回复（ReAct 循环结束）
    Done(String),
    /// 错误
    Error(String),
}

/// 一次对话回合的完整结果
#[derive(Debug, Clone)]
pub struct TurnResult {
    pub final_response: Option<String>,
    pub messages: Vec<Message>,
    pub tool_results: Vec<ToolCallInfo>,
    pub usage: Option<TokenUsage>,
    pub turn_count: u32,
}

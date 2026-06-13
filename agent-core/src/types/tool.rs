use serde::{Deserialize, Serialize};

/// 工具定义（发给模型的 JSON Schema）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub def_type: String, // "function"
    pub function: ToolFunction,
}

/// 工具的 function 段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// 工具注册信息（内部注册表使用）
#[derive(Debug, Clone)]
pub struct ToolEntry {
    pub name: String,
    pub toolset: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
    pub handler: ToolHandler,
    pub check_fn: Option<ToolCheckFn>,
    pub requires_env: Vec<String>,
    pub is_async: bool,
    pub emoji: String,
}

/// 工具处理器签名
pub type ToolHandler = fn(serde_json::Value) -> Result<String, String>;

/// 工具可用性检查函数
pub type ToolCheckFn = fn() -> bool;

impl ToolDef {
    pub fn new(name: &str, description: &str, parameters: serde_json::Value) -> Self {
        Self {
            def_type: "function".to_string(),
            function: ToolFunction {
                name: name.to_string(),
                description: Some(description.to_string()),
                parameters: Some(parameters),
            },
        }
    }
}

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub name: String,
    pub arguments: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::error::AetherError;
use super::Tool;

/// 记忆工具（读写记忆）
pub struct Memory;

#[async_trait]
impl Tool for Memory {
    fn name(&self) -> &str { "memory" }
    fn description(&self) -> &str { "读写长期记忆（跨会话保持）" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "write", "search"],
                    "description": "操作类型"
                },
                "key": {"type": "string", "description": "记忆键名（write 时必填）"},
                "value": {"type": "string", "description": "记忆内容（write 时必填）"},
                "query": {"type": "string", "description": "搜索关键词（search 时必填）"}
            },
            "required": ["action"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let action = args.get("action").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 action 参数".into())
        )?;
        // TODO: Phase 5 实现完整记忆存储
        match action {
            "read" => Ok(json!({"memories": [], "note": "记忆系统尚未完全实现"}).to_string()),
            "write" => Ok(json!({"success": true}).to_string()),
            "search" => Ok(json!({"results": []}).to_string()),
            _ => Err(AetherError::ToolInvalidArgs(format!("不支持的动作: {}", action))),
        }
    }
}

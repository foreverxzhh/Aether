use async_trait::async_trait;
use serde_json::{json, Value};
use crate::error::AetherError;
use crate::memory::core::{CoreMemory, UserProfile, default_hermes_home};
use super::Tool;

/// 记忆工具（读写 L1-L2 记忆）
pub struct Memory;

#[async_trait]
impl Tool for Memory {
    fn name(&self) -> &str { "memory" }
    fn description(&self) -> &str { "读写长期记忆（跨会话保持，存储在 MEMORY.md 和 USER.md）" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string", "enum": ["read", "write", "search"],
                    "description": "操作类型: read=读取全部记忆, write=写入新记忆, search=搜索记忆"
                },
                "key": {"type": "string", "description": "记忆类型: memory(MEMORY.md) 或 profile(USER.md)"},
                "value": {"type": "string", "description": "要写入的内容（write 时必填）"},
                "query": {"type": "string", "description": "搜索关键词（search 时必填）"}
            },
            "required": ["action"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let action = args.get("action").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 action 参数".into())
        )?;
        let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("memory");
        let hermes_home = default_hermes_home();

        let content = match action {
            "read" => {
                if key == "memory" || key == "all" {
                    let core = CoreMemory::new(&hermes_home);
                    let mem = core.read().unwrap_or_default();
                    let profile = UserProfile::new(&hermes_home);
                    let user = profile.read().unwrap_or_default();
                    json!({"memory": mem, "profile": user})
                } else if key == "profile" {
                    let profile = UserProfile::new(&hermes_home);
                    json!({"profile": profile.read().unwrap_or_default()})
                } else {
                    json!({"error": format!("未知的 key: {}", key)})
                }
            }
            "write" => {
                let value = args.get("value").and_then(|v| v.as_str()).ok_or(
                    AetherError::ToolInvalidArgs("write 操作需要 value 参数".into())
                )?;
                if key == "memory" || key == "all" {
                    let core = CoreMemory::new(&hermes_home);
                    let existing = core.read().unwrap_or_default();
                    core.write(&format!("{}\n- {}\n", existing, value))?;
                }
                if key == "profile" || key == "all" {
                    let profile = UserProfile::new(&hermes_home);
                    let existing = profile.read().unwrap_or_default();
                    profile.write(&format!("{}\n- {}\n", existing, value))?;
                }
                json!({"success": true})
            }
            "search" => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let core = CoreMemory::new(&hermes_home);
                let mem = core.read().unwrap_or_default();
                let results: Vec<&str> = mem.lines()
                    .filter(|l| l.to_lowercase().contains(&query.to_lowercase()))
                    .collect();
                json!({"results": results, "count": results.len()})
            }
            _ => return Err(AetherError::ToolInvalidArgs(format!("不支持的动作: {}", action))),
        };

        Ok(content.to_string())
    }
}

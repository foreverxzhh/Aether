use async_trait::async_trait;
use serde_json::{json, Value};
use crate::error::AetherError;
use super::Tool;

/// 技能列表
pub struct SkillsList;

#[async_trait]
impl Tool for SkillsList {
    fn name(&self) -> &str { "skills_list" }
    fn description(&self) -> &str { "列出所有可用的技能" }
    fn parameters(&self) -> Value { json!({}) }
    async fn call(&self, _args: Value) -> Result<String, AetherError> {
        // TODO: Phase 5 实现从 skills 目录读取
        Ok(json!({
            "skills": [],
            "note": "技能系统尚未完全实现",
            "count": 0
        }).to_string())
    }
}

/// 查看技能详情
pub struct SkillView;

#[async_trait]
impl Tool for SkillView {
    fn name(&self) -> &str { "skill_view" }
    fn description(&self) -> &str { "查看技能详细内容" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "技能名称"}
            },
            "required": ["name"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let _name = args.get("name").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 name 参数".into())
        )?;
        Ok(json!({"note": "技能系统尚未完全实现"}).to_string())
    }
}

/// 技能管理
pub struct SkillManage;

#[async_trait]
impl Tool for SkillManage {
    fn name(&self) -> &str { "skill_manage" }
    fn description(&self) -> &str { "创建、更新或删除技能" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {"type": "string", "enum": ["create", "update", "delete"]},
                "name": {"type": "string", "description": "技能名称"},
                "content": {"type": "string", "description": "技能内容（Markdown 格式）"}
            },
            "required": ["action", "name"]
        })
    }
    async fn call(&self, _args: Value) -> Result<String, AetherError> {
        Ok(json!({"success": true, "note": "技能系统尚未完全实现"}).to_string())
    }
}

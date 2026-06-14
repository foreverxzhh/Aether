use async_trait::async_trait;
use serde_json::{json, Value};
use crate::error::AetherError;
use crate::skills::FileSkillStore;
use crate::memory::core::default_hermes_home;
use super::Tool;

fn skills_dir() -> std::path::PathBuf {
    default_hermes_home().join("skills")
}

/// 技能列表
pub struct SkillsList;

#[async_trait]
impl Tool for SkillsList {
    fn name(&self) -> &str { "skills_list" }
    fn description(&self) -> &str { "列出所有可用的技能" }
    fn parameters(&self) -> Value { json!({}) }
    async fn call(&self, _args: Value) -> Result<String, AetherError> {
        let dir = skills_dir();
        if !dir.exists() {
            return Ok(json!({"skills": [], "count": 0}).to_string());
        }

        let mut skills = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| AetherError::IoError(e.to_string()))? {
            let entry = entry.map_err(|e| AetherError::IoError(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            if let Ok(skill) = FileSkillStore::parse_skill_file(&path) {
                skills.push(json!({
                    "name": skill.name,
                    "description": skill.description,
                    "version": skill.version,
                    "category": skill.category,
                }));
            }
        }

        Ok(json!({"skills": skills, "count": skills.len()}).to_string())
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
                "name": {"type": "string", "description": "技能名称（不含 .md）"}
            },
            "required": ["name"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let name = args.get("name").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 name 参数".into())
        )?;

        let path = skills_dir().join(format!("{}.md", name));
        if !path.exists() {
            let path2 = skills_dir().join(name);
            if !path2.exists() {
                return Ok(json!({"error": format!("技能 '{}' 未找到", name)}).to_string());
            }
            let skill = FileSkillStore::parse_skill_file(&path2)?;
            return Ok(json!(skill).to_string());
        }
        let skill = FileSkillStore::parse_skill_file(&path)?;
        Ok(json!(skill).to_string())
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
                "content": {"type": "string", "description": "技能内容（含 frontmatter 的 Markdown）"}
            },
            "required": ["action", "name"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let action = args.get("action").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 action 参数".into())
        )?;
        let name = args.get("name").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 name 参数".into())
        )?;

        match action {
            "create" | "update" => {
                let content = args.get("content").and_then(|v| v.as_str()).ok_or(
                    AetherError::ToolInvalidArgs("create/update 需要 content 参数".into())
                )?;
                let dir = skills_dir();
                std::fs::create_dir_all(&dir).map_err(|e| AetherError::IoError(e.to_string()))?;
                std::fs::write(dir.join(format!("{}.md", name)), content)
                    .map_err(|e| AetherError::IoError(e.to_string()))?;
                Ok(json!({"success": true, "action": action, "name": name}).to_string())
            }
            "delete" => {
                let path = skills_dir().join(format!("{}.md", name));
                if path.exists() {
                    std::fs::remove_file(&path)
                        .map_err(|e| AetherError::IoError(e.to_string()))?;
                }
                Ok(json!({"success": true, "action": "delete", "name": name}).to_string())
            }
            _ => Err(AetherError::ToolInvalidArgs(format!("不支持的动作: {}", action))),
        }
    }
}

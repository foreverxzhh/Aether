use super::Tool;
use crate::error::AetherError;
use crate::memory::core::default_hermes_home;
use crate::skills::FileSkillStore;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;

/// T-1.4: profile-aware skills dir 解析
fn resolve_skills_dir(profile_home: Option<&PathBuf>) -> PathBuf {
    profile_home
        .cloned()
        .unwrap_or_else(default_hermes_home)
        .join("skills")
}

/// 技能列表
pub struct SkillsList {
    profile_home: Option<PathBuf>,
}

impl SkillsList {
    pub fn new(profile_home: Option<PathBuf>) -> Self {
        Self { profile_home }
    }
}

#[async_trait]
impl Tool for SkillsList {
    fn name(&self) -> &str {
        "skills_list"
    }
    fn toolset(&self) -> &str {
        "skills"
    }
    fn description(&self) -> &str {
        "列出所有可用的技能"
    }
    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {
            "category": {"type": "string", "description": "按分类筛选"}
        }})
    }
    async fn call(&self, _args: Value) -> Result<String, AetherError> {
        let dir = resolve_skills_dir(self.profile_home.as_ref());
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
pub struct SkillView {
    profile_home: Option<PathBuf>,
}

impl SkillView {
    pub fn new(profile_home: Option<PathBuf>) -> Self {
        Self { profile_home }
    }
}

#[async_trait]
impl Tool for SkillView {
    fn name(&self) -> &str {
        "skill_view"
    }
    fn toolset(&self) -> &str {
        "skills"
    }
    fn description(&self) -> &str {
        "查看技能详细内容"
    }
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
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 name 参数".into()))?;

        let dir = resolve_skills_dir(self.profile_home.as_ref());
        let path = dir.join(format!("{}.md", name));
        if !path.exists() {
            let path2 = dir.join(name);
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
pub struct SkillManage {
    profile_home: Option<PathBuf>,
}

impl SkillManage {
    pub fn new(profile_home: Option<PathBuf>) -> Self {
        Self { profile_home }
    }
}

#[async_trait]
impl Tool for SkillManage {
    fn name(&self) -> &str {
        "skill_manage"
    }
    fn toolset(&self) -> &str {
        "skills"
    }
    fn description(&self) -> &str {
        "创建、更新或删除技能"
    }
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
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 action 参数".into()))?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 name 参数".into()))?;

        let dir = resolve_skills_dir(self.profile_home.as_ref());
        match action {
            "create" | "update" => {
                let content = args.get("content").and_then(|v| v.as_str()).ok_or(
                    AetherError::ToolInvalidArgs("create/update 需要 content 参数".into()),
                )?;
                std::fs::create_dir_all(&dir).map_err(|e| AetherError::IoError(e.to_string()))?;
                std::fs::write(dir.join(format!("{}.md", name)), content)
                    .map_err(|e| AetherError::IoError(e.to_string()))?;
                Ok(json!({"success": true, "action": action, "name": name}).to_string())
            }
            "delete" => {
                let path = dir.join(format!("{}.md", name));
                if path.exists() {
                    std::fs::remove_file(&path).map_err(|e| AetherError::IoError(e.to_string()))?;
                }
                Ok(json!({"success": true, "action": "delete", "name": name}).to_string())
            }
            _ => Err(AetherError::ToolInvalidArgs(format!(
                "不支持的动作: {}",
                action
            ))),
        }
    }
}

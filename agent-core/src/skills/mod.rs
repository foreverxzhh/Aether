use crate::error::AetherError;
use crate::types::message::Message;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// 技能文件（agentskills.io 格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
}

/// 技能存储
pub struct FileSkillStore {
    skills_dir: PathBuf,
}

impl FileSkillStore {
    pub fn new(hermes_home: &Path) -> Self {
        Self {
            skills_dir: hermes_home.join("skills"),
        }
    }

    /// 解析 agentskills.io 格式的 Markdown 文件
    pub fn parse_skill_file(path: &Path) -> Result<Skill, AetherError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AetherError::IoError(format!("读取技能文件失败: {}", e)))?;

        // 解析 frontmatter (--- 之间的 YAML)
        let (frontmatter, body): (Option<&str>, &str) = if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let yaml_str = &content[3..3 + end];
                let md_body = &content[3 + end + 3..].trim();
                (Some(yaml_str), md_body)
            } else {
                (None, content.as_str())
            }
        } else {
            (None, content.as_str())
        };

        let mut skill = Skill {
            name: path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            description: String::new(),
            version: "1.0.0".to_string(),
            content: body.to_string(),
            category: None,
            tags: vec![],
        };

        // 解析 YAML frontmatter
        if let Some(yaml_str) = frontmatter {
            if let Ok(val) = serde_yaml::from_str::<Value>(yaml_str) {
                if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                    skill.name = name.to_string();
                }
                if let Some(desc) = val.get("description").and_then(|v| v.as_str()) {
                    skill.description = desc.to_string();
                }
                if let Some(ver) = val.get("version").and_then(|v| v.as_str()) {
                    skill.version = ver.to_string();
                }
                if let Some(cat) = val
                    .get("category")
                    .or_else(|| {
                        val.get("metadata")
                            .and_then(|m| m.get("hermes"))
                            .and_then(|h| h.get("category"))
                    })
                    .and_then(|v| v.as_str())
                {
                    skill.category = Some(cat.to_string());
                }
                if let Some(tags) = val
                    .get("tags")
                    .or_else(|| {
                        val.get("metadata")
                            .and_then(|m| m.get("hermes"))
                            .and_then(|h| h.get("tags"))
                    })
                    .and_then(|v| v.as_array())
                {
                    skill.tags = tags
                        .iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
        }

        Ok(skill)
    }
}

#[async_trait]
impl crate::memory::Memory for FileSkillStore {
    async fn add(&mut self, _messages: &[Message]) -> Result<(), AetherError> {
        // Skills 存储不直接添加消息
        Ok(())
    }

    async fn get_context(&self, query: &str, limit: usize) -> Result<Vec<Message>, AetherError> {
        // 搜索技能并返回相关上下文
        let mut results = Vec::new();
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                if count >= limit {
                    break;
                }
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Ok(skill) = Self::parse_skill_file(&path) {
                        if skill.name.contains(query)
                            || skill.description.contains(query)
                            || skill.content.contains(query)
                        {
                            let msg = Message::system(format!(
                                "技能: {}\n描述: {}\n\n{}",
                                skill.name, skill.description, skill.content
                            ));
                            results.push(msg);
                            count += 1;
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    async fn clear(&mut self) -> Result<(), AetherError> {
        Ok(())
    }
}

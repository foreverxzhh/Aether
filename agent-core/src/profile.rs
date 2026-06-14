use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::config::AgentConfig;
use crate::error::AetherError;

/// Profile 系统：多实例隔离
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: Option<String>,
    pub config: AgentConfig,
}

impl Profile {
    pub fn new(name: &str, config: AgentConfig) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            config,
        }
    }

    /// 获取该 profile 的 HERMES_HOME 路径
    pub fn hermes_home(&self, profiles_root: &Path) -> PathBuf {
        if self.name == "default" {
            profiles_root.parent().unwrap_or(profiles_root).to_path_buf()
        } else {
            profiles_root.join(&self.name)
        }
    }
}

/// Profile 管理器
pub struct ProfileManager {
    profiles_root: PathBuf,
    active: String,
}

impl ProfileManager {
    pub fn new(home_dir: &Path) -> Self {
        Self {
            profiles_root: home_dir.join("profiles"),
            active: "default".to_string(),
        }
    }

    pub fn active(&self) -> &str {
        &self.active
    }

    pub fn set_active(&mut self, name: &str) {
        self.active = name.to_string();
    }

    pub fn active_hermes_home(&self, hermes_home: &Path) -> PathBuf {
        if self.active == "default" {
            hermes_home.to_path_buf()
        } else {
            self.profiles_root.join(&self.active)
        }
    }

    pub fn list_profiles(&self) -> Result<Vec<String>, AetherError> {
        let mut profiles = vec!["default".to_string()];
        if self.profiles_root.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.profiles_root) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            profiles.push(name.to_string());
                        }
                    }
                }
            }
        }
        Ok(profiles)
    }
}

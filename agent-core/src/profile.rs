use crate::config::AgentConfig;
use crate::error::AetherError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct Profile {
    pub name: String,
    pub description: Option<String>,
}

impl Profile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
        }
    }

    pub fn hermes_home(&self) -> PathBuf {
        let base = dirs_or_default();
        if self.name == "default" {
            base
        } else {
            base.join("profiles").join(&self.name)
        }
    }
}

pub struct ProfileManager {
    active: String,
}

impl ProfileManager {
    pub fn new(active: Option<String>) -> Self {
        Self {
            active: active.unwrap_or_else(|| "default".into()),
        }
    }
    pub fn active(&self) -> &str {
        &self.active
    }
    pub fn set_active(&mut self, name: &str) {
        self.active = name.to_string();
    }

    pub fn home(&self) -> PathBuf {
        let base = dirs_or_default();
        if self.active == "default" {
            base
        } else {
            base.join("profiles").join(&self.active)
        }
    }

    pub fn list_profiles(&self) -> Result<Vec<String>, AetherError> {
        let mut profiles = vec!["default".to_string()];
        let profiles_dir = dirs_or_default().join("profiles");
        if profiles_dir.exists() {
            for e in
                std::fs::read_dir(&profiles_dir).map_err(|e| AetherError::IoError(e.to_string()))?
            {
                let e = e.map_err(|e| AetherError::IoError(e.to_string()))?;
                if e.path().is_dir() {
                    profiles.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
        Ok(profiles)
    }
}

fn dirs_or_default() -> PathBuf {
    std::env::var("HERMES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(
                std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".into()),
            )
            .join(".hermes")
        })
}

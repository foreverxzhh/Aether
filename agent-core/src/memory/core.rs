use std::path::{Path, PathBuf};
use crate::error::AetherError;

/// L1 核心记忆（MEMORY.md）
pub struct CoreMemory {
    path: PathBuf,
}

impl CoreMemory {
    pub fn new(hermes_home: &Path) -> Self {
        Self {
            path: hermes_home.join("memory").join("MEMORY.md"),
        }
    }

    /// 读取核心记忆
    pub fn read(&self) -> Result<String, AetherError> {
        if !self.path.exists() {
            return Ok(String::new());
        }
        std::fs::read_to_string(&self.path)
            .map_err(|e| AetherError::IoError(format!("读取 MEMORY.md 失败: {}", e)))
    }

    /// 写入核心记忆
    pub fn write(&self, content: &str) -> Result<(), AetherError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AetherError::IoError(format!("创建目录失败: {}", e)))?;
        }
        std::fs::write(&self.path, content)
            .map_err(|e| AetherError::IoError(format!("写入 MEMORY.md 失败: {}", e)))
    }

    /// 追加内容
    pub fn append(&self, text: &str) -> Result<(), AetherError> {
        let mut content = self.read()?;
        content.push_str(text);
        content.push('\n');
        self.write(&content)
    }
}

/// L2 用户画像（USER.md）
pub struct UserProfile {
    path: PathBuf,
}

impl UserProfile {
    pub fn new(hermes_home: &Path) -> Self {
        Self {
            path: hermes_home.join("memory").join("USER.md"),
        }
    }

    pub fn read(&self) -> Result<String, AetherError> {
        if !self.path.exists() {
            return Ok(String::new());
        }
        std::fs::read_to_string(&self.path)
            .map_err(|e| AetherError::IoError(format!("读取 USER.md 失败: {}", e)))
    }

    pub fn write(&self, content: &str) -> Result<(), AetherError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&self.path, content)
            .map_err(|e| AetherError::IoError(format!("写入 USER.md 失败: {}", e)))
    }
}

/// 获取默认 HERMES_HOME
pub fn default_hermes_home() -> PathBuf {
    dirs_or_default()
}

fn dirs_or_default() -> PathBuf {
    std::env::var("HERMES_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string()))
            .join(".hermes")
        })
}

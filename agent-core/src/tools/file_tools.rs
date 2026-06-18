use super::Tool;
use crate::error::AetherError;
use async_trait::async_trait;
use serde_json::{json, Value};
use similar::TextDiff;

/// T-3.7: 路径安全检查 — canonicalize 解析符号链接 + 白名单根目录
fn secure_path(path: &str) -> Result<std::path::PathBuf, AetherError> {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return Err(AetherError::ToolExecutionError("不允许使用绝对路径".into()));
    }
    if p.components().any(|c| c == std::path::Component::ParentDir) {
        return Err(AetherError::ToolExecutionError("不允许目录穿越路径".into()));
    }
    // canonicalize 解析符号链接
    let cwd = std::env::current_dir().map_err(|e| AetherError::IoError(e.to_string()))?;
    let cwd_canon = std::fs::canonicalize(&cwd).unwrap_or_else(|_| cwd.clone());
    let absolute = cwd_canon.join(p);
    // R-3.6: 不 fallback 到绝对路径。对新文件走 parent canonicalize。
    let canonical = match std::fs::canonicalize(&absolute) {
        Ok(p) => p,
        Err(_) => {
            let parent = absolute
                .parent()
                .ok_or_else(|| AetherError::ToolExecutionError("路径无父目录".into()))?;
            let parent_canon = std::fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
            parent_canon.join(
                absolute
                    .file_name()
                    .ok_or_else(|| AetherError::ToolExecutionError("路径无文件名".into()))?,
            )
        }
    };
    if !canonical.starts_with(&cwd_canon) {
        return Err(AetherError::ToolExecutionError("路径越权访问".into()));
    }
    Ok(canonical)
}

/// 读取文件
pub struct ReadFile;
#[async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &str {
        "read_file"
    }
    fn toolset(&self) -> &str {
        "file"
    }
    fn description(&self) -> &str {
        "读取文件内容"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string","description":"文件路径"}},"required":["path"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let path = secure_path(
            args.get("path")
                .and_then(|v| v.as_str())
                .ok_or(AetherError::ToolInvalidArgs("缺少 path 参数".into()))?,
        )?;
        std::fs::read_to_string(&path)
            .map_err(|e| AetherError::ToolExecutionError(format!("读取失败: {}", e)))
    }
}

/// 写入文件
pub struct WriteFile;
#[async_trait]
impl Tool for WriteFile {
    fn name(&self) -> &str {
        "write_file"
    }
    fn toolset(&self) -> &str {
        "file"
    }
    fn description(&self) -> &str {
        "写入文件内容（覆盖）"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let path = secure_path(
            args.get("path")
                .and_then(|v| v.as_str())
                .ok_or(AetherError::ToolInvalidArgs("缺少 path 参数".into()))?,
        )?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 content 参数".into()))?;
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).ok();
        }
        std::fs::write(&path, content)
            .map_err(|e| AetherError::ToolExecutionError(format!("写入失败: {}", e)))?;
        Ok(json!({"success":true,"path":path.to_string_lossy(),"bytes":content.len()}).to_string())
    }
}

/// 文件补丁
pub struct Patch;
#[async_trait]
impl Tool for Patch {
    fn name(&self) -> &str {
        "patch"
    }
    fn toolset(&self) -> &str {
        "file"
    }
    fn description(&self) -> &str {
        "对文件应用行级补丁（diff 格式）"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"path":{"type":"string"},"old_string":{"type":"string"},"new_string":{"type":"string"}},"required":["path","old_string","new_string"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let path = secure_path(
            args.get("path")
                .and_then(|v| v.as_str())
                .ok_or(AetherError::ToolInvalidArgs("缺少 path 参数".into()))?,
        )?;
        let old = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let new = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AetherError::ToolExecutionError(format!("读取失败: {}", e)))?;
        if !content.contains(old) {
            return Err(AetherError::ToolExecutionError("未找到 old_string".into()));
        }
        let result = content.replace(old, new);
        std::fs::write(&path, &result)
            .map_err(|e| AetherError::ToolExecutionError(format!("写入失败: {}", e)))?;
        let diff = TextDiff::from_lines(&content, &result)
            .unified_diff()
            .to_string();
        Ok(json!({"success":true,"diff":diff}).to_string())
    }
}

/// 搜索文件
pub struct SearchFiles;
#[async_trait]
impl Tool for SearchFiles {
    fn name(&self) -> &str {
        "search_files"
    }
    fn toolset(&self) -> &str {
        "file"
    }
    fn description(&self) -> &str {
        "在目录中搜索文件（支持 glob 和内容搜索）"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"},"content":{"type":"string"}},"required":["pattern","path"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("**/*");
        let base = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let content_filter = args.get("content").and_then(|v| v.as_str());
        let full_pattern = std::path::Path::new(base).join(pattern);
        let pattern_str = full_pattern.to_string_lossy().to_string();
        let mut results = Vec::new();
        for entry in glob::glob(&pattern_str)
            .map_err(|e| AetherError::ToolExecutionError(e.to_string()))?
            .flatten()
            .take(50)
        {
            if let Some(ct) = content_filter {
                if let Ok(text) = std::fs::read_to_string(&entry) {
                    if text.contains(ct) {
                        results.push(entry.display().to_string());
                    }
                }
            } else {
                results.push(entry.display().to_string());
            }
        }
        Ok(json!({"files":results,"count":results.len()}).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_security_reject_absolute() {
        #[cfg(unix)]
        assert!(secure_path("/etc/passwd").is_err());
        #[cfg(windows)]
        assert!(secure_path("C:\\Windows\\system32").is_err());
    }
    #[tokio::test]
    async fn test_security_reject_traversal() {
        assert!(secure_path("../secret.txt").is_err());
        assert!(secure_path("a/../../b").is_err());
        #[cfg(windows)]
        assert!(secure_path("subdir\\..\\..\\secret.txt").is_err());
    }
    #[tokio::test]
    async fn test_security_allow_relative() {
        assert!(secure_path("test.txt").is_ok());
        assert!(secure_path("subdir/file.txt").is_ok());
    }
}

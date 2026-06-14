use async_trait::async_trait;
use serde_json::{json, Value};
use similar::TextDiff;
use crate::error::AetherError;
use super::Tool;

/// 读取文件
pub struct ReadFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &str { "read_file" }
    fn description(&self) -> &str { "读取文件内容" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "文件路径"}
            },
            "required": ["path"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 path 参数".into())
        )?;
        let content = std::fs::read_to_string(path)
            .map_err(|e| AetherError::ToolExecutionError(format!("读取失败: {}", e)))?;
        Ok(content)
    }
}

/// 写入文件
pub struct WriteFile;

#[async_trait]
impl Tool for WriteFile {
    fn name(&self) -> &str { "write_file" }
    fn description(&self) -> &str { "写入文件内容（覆盖）" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "文件路径"},
                "content": {"type": "string", "description": "文件内容"}
            },
            "required": ["path", "content"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 path 参数".into())
        )?;
        let content = args.get("content").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 content 参数".into())
        )?;
        std::fs::write(path, content)
            .map_err(|e| AetherError::ToolExecutionError(format!("写入失败: {}", e)))?;
        Ok(json!({"success": true, "path": path, "bytes": content.len()}).to_string())
    }
}

/// 文件补丁（基于 diff）
pub struct Patch;

#[async_trait]
impl Tool for Patch {
    fn name(&self) -> &str { "patch" }
    fn description(&self) -> &str { "对文件应用行级补丁（diff 格式）" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "文件路径"},
                "old_string": {"type": "string", "description": "要被替换的字符串"},
                "new_string": {"type": "string", "description": "替换后的字符串"}
            },
            "required": ["path", "old_string", "new_string"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 path 参数".into())
        )?;
        let old = args.get("old_string").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 old_string 参数".into())
        )?;
        let new = args.get("new_string").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 new_string 参数".into())
        )?;

        let content = std::fs::read_to_string(path)
            .map_err(|e| AetherError::ToolExecutionError(format!("读取失败: {}", e)))?;

        if !content.contains(old) {
            return Err(AetherError::ToolExecutionError("未找到 old_string".into()));
        }

        let result = content.replace(old, new);
        std::fs::write(path, &result)
            .map_err(|e| AetherError::ToolExecutionError(format!("写入失败: {}", e)))?;

        let diff = TextDiff::from_lines(&content, &result)
            .unified_diff()
            .to_string();

        Ok(json!({"success": true, "diff": diff}).to_string())
    }
}

/// 搜索文件
pub struct SearchFiles;

#[async_trait]
impl Tool for SearchFiles {
    fn name(&self) -> &str { "search_files" }
    fn description(&self) -> &str { "在目录中搜索文件（支持 glob 和内容搜索）" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": "glob 模式（如 **/*.rs）"},
                "path": {"type": "string", "description": "搜索目录"},
                "content": {"type": "string", "description": "内容搜索关键词（可选）"}
            },
            "required": ["pattern", "path"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let pattern = args.get("pattern").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 pattern 参数".into())
        )?;
        let base = args.get("path").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 path 参数".into())
        )?;
        let content_filter = args.get("content").and_then(|v| v.as_str());

        let base_path = std::path::Path::new(base);
        let full_pattern = base_path.join(pattern);
        let pattern_str = full_pattern.to_string_lossy().to_string();

        let mut results = Vec::new();
        let glob_pattern = glob::glob(&pattern_str)
            .map_err(|e| AetherError::ToolExecutionError(format!("glob 错误: {}", e)))?;

        for entry in glob_pattern.flatten().take(50) {
            if let Some(content) = content_filter {
                if let Ok(text) = std::fs::read_to_string(&entry) {
                    if text.contains(content) {
                        results.push(entry.display().to_string());
                    }
                }
            } else {
                results.push(entry.display().to_string());
            }
        }

        Ok(json!({"files": results, "count": results.len()}).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadFile;
        let result = tool.call(json!({"path": "/tmp/nonexistent_xyz_123"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_and_read() {
        let dir = std::env::temp_dir().join("aether_test_write");
        let path = dir.join("test.txt");
        std::fs::create_dir_all(&dir).ok();

        let write = WriteFile;
        let result = write.call(json!({
            "path": path.to_string_lossy(),
            "content": "hello world"
        })).await;
        assert!(result.is_ok());

        let read = ReadFile;
        let result = read.call(json!({"path": path.to_string_lossy()})).await;
        assert_eq!(result.unwrap(), "hello world");

        std::fs::remove_dir_all(dir).ok();
    }
}

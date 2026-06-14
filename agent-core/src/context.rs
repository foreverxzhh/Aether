use std::path::Path;

/// 上下文引擎：收集当前工作目录的相关信息注入 prompt
pub struct ContextEngine;

impl ContextEngine {
    /// 收集上下文信息
    pub fn collect_context(workspace_dir: Option<&str>) -> String {
        let mut parts = Vec::new();

        // 当前时间
        let now = chrono::Local::now();
        parts.push(format!("当前时间: {}", now.format("%Y-%m-%d %H:%M:%S")));

        // 工作目录文件列表
        if let Some(dir) = workspace_dir {
            let path = Path::new(dir);
            if path.exists() {
                let files = Self::list_files(path, 3);
                if !files.is_empty() {
                    parts.push(format!("工作目录文件:\n{}", files));
                }
            }
        }

        parts.join("\n")
    }

    /// 列出目录中最近修改的文件（递归，最多 depth 层）
    fn list_files(dir: &Path, depth: u32) -> String {
        if depth == 0 {
            return String::new();
        }
        let mut entries = Vec::new();
        if let Ok(read) = dir.read_dir() {
            for entry in read.flatten() {
                let path = entry.path();
                if path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
                {
                    continue; // 跳过隐藏文件
                }
                if path.is_dir() {
                    let sub = Self::list_files(&path, depth - 1);
                    if !sub.is_empty() {
                        entries.push(format!(
                            "  {}/",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                        for line in sub.lines() {
                            entries.push(format!("  {}", line));
                        }
                    }
                } else if path.is_file() {
                    entries.push(format!(
                        "  {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }
        }
        entries.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_context_contains_time() {
        let ctx = ContextEngine::collect_context(None);
        assert!(ctx.contains("当前时间"));
    }
}

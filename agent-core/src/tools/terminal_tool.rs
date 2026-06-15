use super::Tool;
use crate::error::AetherError;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// 终端执行（本地）
pub struct Terminal;

/// 危险命令正则（比子串匹配更精确）
static DANGEROUS_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"rm\s+(-[a-zA-Z]*[rRf]+\s+)*(/|/\*)", // rm -rf / 或 rm -rf /*
        r"mkfs\.?\w*",                         // mkfs variants
        r"dd\s+if=",                           // dd raw write
        r":\(\)\s*\{[^}]*:\|:&\s*\};:",        // fork bomb
        r">\s*/dev/sd[a-z]",                   // 覆盖块设备
        r"chmod\s+(-R\s+)?0?777\s+/",          // 危险的 chmod
        r"fork\s*bomb|shutdown|reboot|halt",   // 系统命令
    ]
    .iter()
    .map(|p| Regex::new(&format!("(?i){}", p)).unwrap())
    .collect()
});

#[async_trait]
impl Tool for Terminal {
    fn name(&self) -> &str {
        "terminal"
    }
    fn description(&self) -> &str {
        "在终端中执行命令（本地 shell）"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "要执行的命令"},
                "timeout": {"type": "number", "description": "超时秒数（默认 30）"}
            },
            "required": ["command"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 command 参数".into()))?;

        // 正则安全检查
        for pattern in DANGEROUS_PATTERNS.iter() {
            if pattern.is_match(command) {
                return Err(AetherError::ToolExecutionError(format!(
                    "危险命令被阻止: 匹配模式 '{}'",
                    pattern.as_str()
                )));
            }
        }

        let timeout_secs = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);

        // T-2.1: Cross-platform shell invocation
        let (shell, shell_arg) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let output = timeout(Duration::from_secs(timeout_secs), async {
            Command::new(shell)
                .args([shell_arg, command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
        })
        .await
        .map_err(|_| AetherError::ToolExecutionError("命令执行超时".into()))?
        .map_err(|e| AetherError::ToolExecutionError(format!("命令执行失败: {}", e)))?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1),
            "success": output.status.success(),
        })
        .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo() {
        let r = Terminal
            .call(json!({"command": "echo hello"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["exit_code"], 0);
    }

    #[tokio::test]
    async fn test_block_dangerous() {
        assert!(Terminal.call(json!({"command": "rm -rf /"})).await.is_err());
        assert!(Terminal
            .call(json!({"command": "rm -rf /*"}))
            .await
            .is_err());
        assert!(Terminal
            .call(json!({"command": "chmod -R 777 /tmp"}))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_allow_mkdir() {
        let r = Terminal
            .call(json!({"command": "mkdir test_dir 2>nul || echo exists"}))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(v["exit_code"], 0);
    }
}

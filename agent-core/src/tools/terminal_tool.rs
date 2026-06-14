use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::timeout;
use std::time::Duration;
use crate::error::AetherError;
use super::Tool;

/// 终端执行（本地）
pub struct Terminal;

/// 危险命令列表（阻止执行）
const DANGEROUS_COMMANDS: &[&str] = &[
    "rm -rf /", "rm -rf /*", "mkfs", "dd if=", ":(){ :|:& };:",
    "> /dev/sda", "| sh", "wget ", "curl ",
];

#[async_trait]
impl Tool for Terminal {
    fn name(&self) -> &str { "terminal" }
    fn description(&self) -> &str { "在终端中执行命令（本地 shell）" }
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
        let command = args.get("command").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 command 参数".into())
        )?;

        // 安全检查
        let cmd_lower = command.to_lowercase();
        for dangerous in DANGEROUS_COMMANDS {
            if cmd_lower.contains(dangerous) {
                return Err(AetherError::ToolExecutionError(
                    format!("危险命令被阻止: {}", dangerous)
                ));
            }
        }

        let timeout_secs = args.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let output = timeout(Duration::from_secs(timeout_secs), async {
            Command::new("cmd")
                .args(["/C", command])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
        }).await
            .map_err(|_| AetherError::ToolExecutionError("命令执行超时".into()))?
            .map_err(|e| AetherError::ToolExecutionError(format!("命令执行失败: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": output.status.code().unwrap_or(-1),
            "success": output.status.success(),
        }).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo() {
        let tool = Terminal;
        let result = tool.call(json!({"command": "echo hello"})).await;
        assert!(result.is_ok());
        let resp: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(resp["exit_code"], 0);
    }

    #[tokio::test]
    async fn test_block_dangerous() {
        let tool = Terminal;
        let result = tool.call(json!({"command": "rm -rf /"})).await;
        assert!(result.is_err());
    }
}

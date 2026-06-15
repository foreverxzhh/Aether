//! 终端后端扩展：Docker 容器、SSH 远程、隔离沙箱
use super::Tool;
use crate::error::AetherError;
use async_trait::async_trait;
use serde_json::{json, Value};

/// Docker 容器内执行命令
pub struct DockerTerminal;
#[async_trait]
impl Tool for DockerTerminal {
    fn name(&self) -> &str {
        "docker_terminal"
    }
    fn description(&self) -> &str {
        "在 Docker 容器中执行命令（需要 Docker daemon）"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{
            "container":{"type":"string","description":"容器名称或ID"},
            "command":{"type":"string","description":"命令"},
            "workdir":{"type":"string","description":"工作目录"}
        },"required":["container","command"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let container = args
            .get("container")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 container 参数".into()))?;
        let cmd = args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("echo hello");
        let wd = args.get("workdir").and_then(|v| v.as_str()).unwrap_or("/");

        let output = tokio::process::Command::new("docker")
            .args(["exec", "-w", wd, container, "sh", "-c", cmd])
            .output()
            .await
            .map_err(|e| AetherError::ToolExecutionError(format!("Docker 执行失败: {}", e)))?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1),
        })
        .to_string())
    }
}

/// SSH 远程执行命令
pub struct SshTerminal;
#[async_trait]
impl Tool for SshTerminal {
    fn name(&self) -> &str {
        "ssh_terminal"
    }
    fn description(&self) -> &str {
        "通过 SSH 在远程主机上执行命令"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{
            "host":{"type":"string","description":"远程主机地址"},
            "command":{"type":"string","description":"命令"},
            "user":{"type":"string","description":"SSH用户名"}
        },"required":["host","command"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let host = args
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 host 参数".into()))?;
        let cmd = args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("echo hello");
        let user = args.get("user").and_then(|v| v.as_str()).unwrap_or("root");

        let output = tokio::process::Command::new("ssh")
            .args([&format!("{}@{}", user, host), cmd])
            .output()
            .await
            .map_err(|e| AetherError::ToolExecutionError(format!("SSH 执行失败: {}", e)))?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1),
        })
        .to_string())
    }
}

/// 代码执行沙箱
/// T-4.1: 默认在宿主执行(python/node/sh)。生产环境建议 Docker(--network=none)
/// 移动端不可用。README应注明 "ExecuteCode: desktop only, unsafe in production"
pub struct ExecuteCode;
#[async_trait]
impl Tool for ExecuteCode {
    fn name(&self) -> &str {
        "execute_code"
    }
    fn description(&self) -> &str {
        "在隔离进程中执行代码片段（Python/JavaScript），限制资源"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{
            "language":{"type":"string","enum":["python","javascript","shell"]},
            "code":{"type":"string","description":"代码内容"},
            "timeout":{"type":"number","description":"超时秒数(默认10)"}
        },"required":["language","code"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let lang = args
            .get("language")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 language 参数".into()))?;
        let code = args
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 code 参数".into()))?;
        let timeout_secs = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(10);

        let (interpreter, arg) = match lang {
            "python" => ("python", "-c"),
            "javascript" => ("node", "-e"),
            "shell" => ("sh", "-c"),
            _ => {
                return Err(AetherError::ToolInvalidArgs(format!(
                    "不支持的语言: {}",
                    lang
                )))
            }
        };

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new(interpreter)
                .arg(arg)
                .arg(code)
                .output(),
        )
        .await
        .map_err(|_| AetherError::ToolExecutionError("代码执行超时".into()))?
        .map_err(|e| AetherError::ToolExecutionError(format!("执行失败: {}", e)))?;

        Ok(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code().unwrap_or(-1),
        })
        .to_string())
    }
}

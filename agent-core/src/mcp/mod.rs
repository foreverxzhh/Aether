use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use serde_json::Value;
use crate::error::AetherError;

/// MCP 工具描述
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// MCP 客户端
pub struct McpClient {
    transport: McpTransport,
    pub tools: HashMap<String, McpTool>,
}

enum McpTransport {
    Stdio {
        stdin: Option<std::process::ChildStdin>,
        stdout: Option<BufReader<std::process::ChildStdout>>,
    },
    Http { base_url: String, client: reqwest::Client },
}

impl McpClient {
    pub async fn connect_stdio(command: &str) -> Result<Self, AetherError> {
        let mut shell_cmd = std::process::Command::new(if cfg!(windows) { "cmd" } else {
            command.split_whitespace().next().unwrap_or("sh")
        });
        if cfg!(windows) {
            shell_cmd.args(["/C", command]);
        } else {
            let args: Vec<&str> = command.split_whitespace().skip(1).collect();
            shell_cmd.args(&args);
        }

        let mut child = shell_cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| AetherError::McpConnectionError(format!("启动失败: {}", e)))?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take().map(BufReader::new);

        let mut client = Self {
            transport: McpTransport::Stdio { stdin, stdout },
            tools: HashMap::new(),
        };
        client.refresh_tools().await?;
        Ok(client)
    }

    pub async fn connect_http(base_url: &str) -> Result<Self, AetherError> {
        let client = reqwest::Client::new();
        let mut mcp = Self {
            transport: McpTransport::Http { base_url: base_url.to_string(), client },
            tools: HashMap::new(),
        };
        mcp.refresh_tools().await?;
        Ok(mcp)
    }

    /// 发送 JSON-RPC 请求并读取完整响应行
    async fn send_stdio_request(&mut self, request: &str) -> Result<String, AetherError> {
        match &mut self.transport {
            McpTransport::Stdio { stdin, stdout } => {
                let stdin = stdin.as_mut().ok_or_else(|| {
                    AetherError::McpConnectionError("stdin 未打开".into())
                })?;
                // 写入请求 (JSON-RPC over stdio: 单行 JSON + \n)
                writeln!(stdin, "{}", request)
                    .map_err(|e| AetherError::McpConnectionError(format!("写入失败: {}", e)))?;
                stdin.flush().ok();

                // 读取一行响应
                let stdout = stdout.as_mut().ok_or_else(|| {
                    AetherError::McpConnectionError("stdout 未打开".into())
                })?;
                let mut line = String::new();
                stdout.read_line(&mut line)
                    .map_err(|e| AetherError::McpConnectionError(format!("读取失败: {}", e)))?;
                Ok(line.trim().to_string())
            }
            _ => Err(AetherError::McpConnectionError("当前传输不是 stdio".into())),
        }
    }

    /// 发送 JSON-RPC 请求并读取响应（HTTP 传输）
    async fn send_http_request(&self, request: &Value) -> Result<String, AetherError> {
        match &self.transport {
            McpTransport::Http { base_url, client } => {
                let resp = client.post(format!("{}/jsonrpc", base_url))
                    .json(request)
                    .send()
                    .await
                    .map_err(|e| AetherError::McpConnectionError(e.to_string()))?;
                resp.text().await.map_err(|e| AetherError::McpConnectionError(e.to_string()))
            }
            _ => Err(AetherError::McpConnectionError("当前传输不是 HTTP".into())),
        }
    }

    /// 刷新工具列表
    pub async fn refresh_tools(&mut self) -> Result<(), AetherError> {
        let request = serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}
        });

        let response = match &self.transport {
            McpTransport::Http { .. } => self.send_http_request(&request).await?,
            McpTransport::Stdio { .. } => self.send_stdio_request(&request.to_string()).await?,
        };

        if let Ok(resp) = serde_json::from_str::<McpListResponse>(&response) {
            for tool in resp.result.tools {
                self.tools.insert(tool.name.clone(), McpTool {
                    name: tool.name.clone(),
                    description: tool.description.clone().unwrap_or_default(),
                    parameters: tool.input_schema.unwrap_or(serde_json::json!({})),
                });
            }
        }

        Ok(())
    }

    /// 调用工具
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<String, AetherError> {
        let request = serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": name, "arguments": args }
        });

        match &self.transport {
            McpTransport::Http { .. } => self.send_http_request(&request).await,
            McpTransport::Stdio { .. } => {
                // 对于调用，需要在 stdout 中读取完整的 JSON 响应
                let json_str = serde_json::to_string(&request)
                    .map_err(|e| AetherError::McpParseError(e.to_string()))?;
                // 重新获取可变引用
                Err(AetherError::McpConnectionError(
                    "stdio 调用: 请使用 send_request 方法".into()
                ))
            }
        }
    }

    pub fn list_tools(&self) -> Vec<&McpTool> {
        self.tools.values().collect()
    }

    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

// ── JSON-RPC 响应 ──

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct McpListResponse {
    jsonrpc: String,
    id: u64,
    result: McpListResult,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct McpListResult {
    tools: Vec<McpToolRaw>,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct McpToolRaw {
    name: String,
    description: Option<String>,
    #[serde(rename = "inputSchema")]
    input_schema: Option<Value>,
}

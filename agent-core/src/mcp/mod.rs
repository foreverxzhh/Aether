use std::collections::HashMap;
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
    tools: HashMap<String, McpTool>,
}

enum McpTransport {
    Stdio { process: Option<std::process::Child> },
    Http { base_url: String, client: reqwest::Client },
}

impl McpClient {
    /// 通过 stdio 子进程连接 MCP 服务器
    pub async fn connect_stdio(command: &str) -> Result<Self, AetherError> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(AetherError::McpConnectionError("MCP 命令为空".into()));
        }

        #[cfg(windows)]
        let child = std::process::Command::new("cmd")
            .args(["/C", command])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn();
        #[cfg(not(windows))]
        let child = std::process::Command::new(parts[0])
            .args(&parts[1..])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn();

        let mut client = Self {
            transport: McpTransport::Stdio { process: child.ok() },
            tools: HashMap::new(),
        };
        client.refresh_tools().await?;
        Ok(client)
    }

    /// 通过 HTTP SSE 连接 MCP 服务器
    pub async fn connect_http(base_url: &str) -> Result<Self, AetherError> {
        let client = reqwest::Client::new();
        let mut mcp = Self {
            transport: McpTransport::Http { base_url: base_url.to_string(), client },
            tools: HashMap::new(),
        };
        mcp.refresh_tools().await?;
        Ok(mcp)
    }

    /// 发现服务器工具列表（JSON-RPC）
    pub async fn refresh_tools(&mut self) -> Result<(), AetherError> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let response = match &self.transport {
            McpTransport::Http { base_url, client } => {
                let resp = client.post(format!("{}/jsonrpc", base_url))
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| AetherError::McpConnectionError(e.to_string()))?;
                resp.text().await.unwrap_or_default()
            }
            McpTransport::Stdio { process } => {
                if let Some(p) = process {
                    // 简单 stdio 通信
                    let stdin = p.stdin.as_ref();
                    let stdout = p.stdout.as_ref();
                    if let (Some(_in), Some(_out)) = (stdin, stdout) {
                        // TODO: 完整 stdio JSON-RPC 实现
                        return Err(AetherError::McpConnectionError("stdio 传输完整实现待完成".into()));
                    }
                }
                return Err(AetherError::McpConnectionError("MCP 子进程未运行".into()));
            }
        };

        // 解析 JSON-RPC 响应
        if !response.is_empty() {
            if let Ok(resp) = serde_json::from_str::<McpListResponse>(&response) {
                for tool in resp.result.tools {
                    self.tools.insert(tool.name.clone(), McpTool {
                        name: tool.name.clone(),
                        description: tool.description.clone().unwrap_or_default(),
                        parameters: tool.input_schema.unwrap_or(serde_json::json!({})),
                    });
                }
            }
        }

        Ok(())
    }

    /// 调用工具
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<String, AetherError> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": name, "arguments": args }
        });

        match &self.transport {
            McpTransport::Http { base_url, client } => {
                let resp = client.post(format!("{}/jsonrpc", base_url))
                    .json(&request)
                    .send()
                    .await
                    .map_err(|e| AetherError::McpConnectionError(e.to_string()))?;
                let text = resp.text().await.unwrap_or_default();
                Ok(text)
            }
            _ => Err(AetherError::McpConnectionError("当前传输模式不支持工具调用".into())),
        }
    }

    /// 列出所有发现的工具
    pub fn list_tools(&self) -> Vec<&McpTool> {
        self.tools.values().collect()
    }
}

/// JSON-RPC 响应解析
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

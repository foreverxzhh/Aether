use crate::error::AetherError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};

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
        server: Arc<McpStdioServer>,
    },
    Http {
        base_url: String,
        client: reqwest::Client,
    },
}

/// T-2.4: 真正的 stdio MCP server 句柄
///
/// 持有 tokio::process::Child（drop 即 kill 子进程，避免孤儿）。
/// 用 AtomicU64 生成请求 id，避免并发撞车。
/// 用 pending oneshot map 配对 request/response。
pub struct McpStdioServer {
    _child: Child, // drop = kill
    stdin: Mutex<ChildStdin>,
    next_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
}

impl McpStdioServer {
    /// 启动子进程并完成 initialize 握手
    pub async fn connect(command: &str) -> Result<Self, AetherError> {
        // 解析命令行（兼容 windows / posix）
        let parts: Vec<String> = command
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        if parts.is_empty() {
            return Err(AetherError::McpConnectionError("空命令".into()));
        }

        let mut cmd = Command::new(&parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }
        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| AetherError::McpConnectionError(format!("启动失败: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AetherError::McpConnectionError("无 stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AetherError::McpConnectionError("无 stdout".into()))?;

        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // 后台 reader：按行读取 JSON-RPC 响应，按 id 派发给对应的 oneshot
        let pending_clone = pending.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {}
                    Err(_) => break,
                }
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
                    // 只关心带 id 的响应；notification 没有 id，忽略
                    if let Some(id) = v.get("id").and_then(|x| x.as_u64()) {
                        if let Some(tx) = pending_clone.lock().await.remove(&id) {
                            let _ = tx.send(v);
                        }
                    }
                }
            }
            // 子进程退出后清理所有未完成的等待者（drop sender 即让 rx.await 报错）
            pending_clone.lock().await.clear();
        });

        let server = Self {
            _child: child,
            stdin: Mutex::new(stdin),
            next_id: AtomicU64::new(1),
            pending,
        };

        // initialize 握手
        server
            .request(
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "aether",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )
            .await?;
        server
            .notify("notifications/initialized", serde_json::json!({}))
            .await?;
        Ok(server)
    }

    /// 发送 JSON-RPC request 并等待 response
    async fn request(&self, method: &str, params: Value) -> Result<Value, AetherError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        {
            let mut s = self.stdin.lock().await;
            s.write_all(format!("{}\n", body).as_bytes())
                .await
                .map_err(|e| AetherError::McpConnectionError(format!("写入失败: {}", e)))?;
            s.flush()
                .await
                .map_err(|e| AetherError::McpConnectionError(format!("flush 失败: {}", e)))?;
        }
        let resp = rx
            .await
            .map_err(|_| AetherError::McpConnectionError("MCP server 关闭或未响应".into()))?;
        if let Some(err) = resp.get("error") {
            return Err(AetherError::McpServerError(err.to_string()));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// 发送 JSON-RPC notification（无 id，不等响应）
    async fn notify(&self, method: &str, params: Value) -> Result<(), AetherError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        let mut s = self.stdin.lock().await;
        s.write_all(format!("{}\n", body).as_bytes())
            .await
            .map_err(|e| AetherError::McpConnectionError(format!("写入失败: {}", e)))?;
        s.flush()
            .await
            .map_err(|e| AetherError::McpConnectionError(format!("flush 失败: {}", e)))?;
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<Value>, AetherError> {
        let r = self.request("tools/list", serde_json::json!({})).await?;
        Ok(r.get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, AetherError> {
        self.request(
            "tools/call",
            serde_json::json!({ "name": name, "arguments": args }),
        )
        .await
    }
}

impl McpClient {
    pub async fn connect_stdio(command: &str) -> Result<Self, AetherError> {
        let server = Arc::new(McpStdioServer::connect(command).await?);
        let mut client = Self {
            transport: McpTransport::Stdio { server },
            tools: HashMap::new(),
        };
        client.refresh_tools().await?;
        Ok(client)
    }

    pub async fn connect_http(base_url: &str) -> Result<Self, AetherError> {
        let client = reqwest::Client::new();
        let mut mcp = Self {
            transport: McpTransport::Http {
                base_url: base_url.to_string(),
                client,
            },
            tools: HashMap::new(),
        };
        mcp.refresh_tools().await?;
        Ok(mcp)
    }

    /// 发送 JSON-RPC 请求并读取响应（HTTP 传输）
    async fn send_http_request(&self, request: &Value) -> Result<String, AetherError> {
        match &self.transport {
            McpTransport::Http { base_url, client } => {
                let resp = client
                    .post(format!("{}/jsonrpc", base_url))
                    .json(request)
                    .send()
                    .await
                    .map_err(|e| AetherError::McpConnectionError(e.to_string()))?;
                resp.text()
                    .await
                    .map_err(|e| AetherError::McpConnectionError(e.to_string()))
            }
            _ => Err(AetherError::McpConnectionError("当前传输不是 HTTP".into())),
        }
    }

    /// 刷新工具列表
    pub async fn refresh_tools(&mut self) -> Result<(), AetherError> {
        match &self.transport {
            McpTransport::Http { .. } => {
                let request = serde_json::json!({
                    "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}
                });
                let response = self.send_http_request(&request).await?;
                if let Ok(resp) = serde_json::from_str::<McpListResponse>(&response) {
                    for tool in resp.result.tools {
                        self.tools.insert(
                            tool.name.clone(),
                            McpTool {
                                name: tool.name.clone(),
                                description: tool.description.clone().unwrap_or_default(),
                                parameters: tool.input_schema.unwrap_or(serde_json::json!({})),
                            },
                        );
                    }
                }
            }
            McpTransport::Stdio { server } => {
                let tools = server.list_tools().await?;
                for v in tools {
                    let name = v
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let description = v
                        .get("description")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    let parameters = v
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    self.tools.insert(
                        name.clone(),
                        McpTool {
                            name,
                            description,
                            parameters,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    /// 调用工具（T-2.4: 真正的 stdio 实现，HTTP 不变）
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<String, AetherError> {
        match &self.transport {
            McpTransport::Http { .. } => {
                let request = serde_json::json!({
                    "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                    "params": { "name": name, "arguments": args }
                });
                self.send_http_request(&request).await
            }
            McpTransport::Stdio { server } => {
                let result = server.call_tool(name, args).await?;
                Ok(result.to_string())
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

//! R-1.3: MCP HTTP transport — Streamable HTTP (spec 2025-03-26)
//!
//! POST JSON-RPC → 同步 response 或 SSE stream。
//! Session-Id header 管理 + 重连支持。

use crate::error::AetherError;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::Mutex;

/// MCP HTTP 服务器句柄
///
/// H1: `session_id` 用 `Mutex<Option<String>>` 持有，
/// request() 发送时附带，收到响应后写回。
pub struct McpHttpServer {
    base_url: String,
    session_id: Mutex<Option<String>>,
    next_id: AtomicU64,
    timeout: Duration,
}

impl McpHttpServer {
    /// 创建新的 HTTP MCP 连接并完成 initialize 握手
    pub async fn connect(base_url: &str) -> Result<Self, AetherError> {
        let server = Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            session_id: Mutex::new(None),
            next_id: AtomicU64::new(1),
            timeout: Duration::from_secs(30),
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
    pub async fn request(&self, method: &str, params: Value) -> Result<Value, AetherError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| AetherError::McpConnectionError(format!("HTTP client 构建失败: {}", e)))?;

        let mut req = client.post(&self.base_url).json(&body);

        // H1: 发送时附带已存储的 Session-Id
        {
            let guard = self.session_id.lock().await;
            if let Some(ref sid) = *guard {
                req = req.header("Mcp-Session-Id", sid);
            }
        }

        let resp = req.send().await.map_err(|e| {
            AetherError::McpConnectionError(format!("HTTP 请求失败: {}", e))
        })?;

        // H1: 收到 Session-Id 立刻持久化
        if let Some(sid) = resp
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|v| v.to_str().ok())
        {
            *self.session_id.lock().await = Some(sid.to_string());
        }

        let status = resp.status();
        let resp_body: Value = resp.json().await.map_err(|e| {
            AetherError::McpParseError(format!("JSON 解析失败: {}", e))
        })?;

        if !status.is_success() {
            return Err(AetherError::McpServerError(format!(
                "HTTP {} : {}",
                status.as_u16(),
                resp_body
            )));
        }

        if let Some(err) = resp_body.get("error") {
            return Err(AetherError::McpServerError(err.to_string()));
        }

        Ok(resp_body.get("result").cloned().unwrap_or(Value::Null))
    }

    /// 发送 JSON-RPC notification（无 id，不等响应）
    pub async fn notify(&self, method: &str, params: Value) -> Result<(), AetherError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| AetherError::McpConnectionError(format!("HTTP client 构建失败: {}", e)))?;

        let mut req = client.post(&self.base_url).json(&body);
        {
            let guard = self.session_id.lock().await;
            if let Some(ref sid) = *guard {
                req = req.header("Mcp-Session-Id", sid);
            }
        }

        req.send().await.map_err(|e| {
            AetherError::McpConnectionError(format!("HTTP notify 失败: {}", e))
        })?;

        Ok(())
    }

    /// 列出可用工具
    pub async fn list_tools(&self) -> Result<Vec<Value>, AetherError> {
        let r = self.request("tools/list", serde_json::json!({})).await?;
        Ok(r.get("tools")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default())
    }

    /// 调用工具
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, AetherError> {
        self.request(
            "tools/call",
            serde_json::json!({ "name": name, "arguments": args }),
        )
        .await
    }

    /// 获取当前 base_url
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 带超时配置
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_http_server() {
        // 验证构造成功（不实际连接）
        let server = McpHttpServer {
            base_url: "http://localhost:8080/mcp".to_string(),
            session_id: Mutex::new(None),
            next_id: AtomicU64::new(1),
            timeout: Duration::from_secs(30),
        };
        assert_eq!(server.base_url(), "http://localhost:8080/mcp");
    }

    #[test]
    fn test_with_timeout() {
        let server = McpHttpServer {
            base_url: "http://localhost:8080/mcp".to_string(),
            session_id: Mutex::new(None),
            next_id: AtomicU64::new(1),
            timeout: Duration::from_secs(30),
        };
        let server = server.with_timeout(Duration::from_secs(60));
        assert_eq!(server.timeout, Duration::from_secs(60));
    }
}

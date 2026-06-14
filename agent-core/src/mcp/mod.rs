use serde_json::Value;
use crate::error::AetherError;

/// MCP 客户端
pub struct McpClient;

impl McpClient {
    /// 通过 stdio 连接 MCP 服务器
    pub async fn connect_stdio(_command: &str) -> Result<Self, AetherError> {
        // TODO: Phase 7 实现
        Err(AetherError::McpConnectionError("MCP stdio 尚未实现".to_string()))
    }

    /// 发现服务器工具列表
    pub async fn list_tools(&self) -> Result<Vec<McpTool>, AetherError> {
        // TODO: 发送 JSON-RPC 请求
        Ok(vec![])
    }

    /// 调用工具
    pub async fn call_tool(&self, _name: &str, _args: Value) -> Result<String, AetherError> {
        Err(AetherError::McpConnectionError("MCP 调用尚未实现".to_string()))
    }
}

/// MCP 服务器暴露的工具
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

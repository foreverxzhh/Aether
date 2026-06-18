//! R-3.1: MCP Server — 将 Aether 内置工具暴露为 MCP 协议
//!
//! 通过 stdio JSON-RPC 与 MCP host（Claude Desktop / Cursor 等）通信。
//! 在 CLI 中用 `aether mcp-server` 子命令启动。

use crate::error::AetherError;
use crate::tools::ToolRegistry;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// MCP Server — 接受 host 调用，暴露 Aether 工具
pub struct McpServer {
    tools: Arc<std::sync::RwLock<ToolRegistry>>,
    next_id: AtomicU64,
    server_info: Value,
    initialized: std::sync::atomic::AtomicBool,
}

impl McpServer {
    /// 创建新的 MCP server
    pub fn new(tools: Arc<std::sync::RwLock<ToolRegistry>>) -> Self {
        Self {
            tools,
            next_id: AtomicU64::new(0),
            server_info: serde_json::json!({
                "name": "aether",
                "version": env!("CARGO_PKG_VERSION"),
            }),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 运行 MCP server — 阻塞，从 stdin 读 JSON-RPC，写回 stdout
    pub fn run(&self) -> Result<(), AetherError> {
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        let stdout = std::io::stdout();

        for line in reader.lines() {
            let line = line.map_err(|e| AetherError::IoError(e.to_string()))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let request: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    self.write_error(&mut stdout.lock(), None, &format!("Parse error: {}", e));
                    continue;
                }
            };

            let id = request.get("id").cloned();
            let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let params = request
                .get("params")
                .cloned()
                .unwrap_or(Value::Null);

            // 如果 id 是 null → notification，不需要响应
            let is_notification = id.is_none() || id.as_ref() == Some(&Value::Null);

            match method {
                "initialize" => {
                    let result = self.handle_initialize(&params);
                    if !is_notification {
                        self.write_response(&mut stdout.lock(), id.as_ref(), &result);
                    }
                    self.initialized.store(true, Ordering::SeqCst);
                }
                "notifications/initialized" => {
                    // 确认收到 — 不响应
                }
                "tools/list" => {
                    if !self.initialized.load(Ordering::SeqCst) {
                        self.write_error(&mut stdout.lock(), id.as_ref(), "Server not initialized. Send initialize first.");
                    } else {
                        let result = self.handle_tools_list();
                        if !is_notification {
                            self.write_response(&mut stdout.lock(), id.as_ref(), &result);
                        }
                    }
                }
                "tools/call" => {
                    if !self.initialized.load(Ordering::SeqCst) {
                        self.write_error(&mut stdout.lock(), id.as_ref(), "Server not initialized. Send initialize first.");
                    } else {
                        let result = self.handle_tools_call(&params);
                        if !is_notification {
                            self.write_response(&mut stdout.lock(), id.as_ref(), &result);
                        }
                    }
                }
                "ping" => {
                    if !is_notification {
                        self.write_response(&mut stdout.lock(), id.as_ref(), &serde_json::json!({}));
                    }
                }
                _ => {
                    if !is_notification {
                        self.write_error(
                            &mut stdout.lock(),
                            id.as_ref(),
                            &format!("Unknown method: {}", method),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_initialize(&self, _params: &Value) -> Value {
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": self.server_info,
        })
    }

    fn handle_tools_list(&self) -> Value {
        let registry = self.tools.read().unwrap_or_else(|e| e.into_inner());
        let defs = registry.get_definitions();
        let tools: Vec<Value> = defs
            .iter()
            .map(|d| {
                let func = d.get("function").cloned().unwrap_or(Value::Null);
                let name = func
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                let description = func
                    .get("description")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                let input_schema = func
                    .get("parameters")
                    .cloned()
                    .unwrap_or(serde_json::json!({"type": "object", "properties": {}}));
                serde_json::json!({
                    "name": name,
                    "description": description,
                    "inputSchema": input_schema,
                })
            })
            .collect();

        serde_json::json!({ "tools": tools })
    }

    fn handle_tools_call(&self, params: &Value) -> Value {
        let name = params
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        if name.is_empty() {
            return serde_json::json!({
                "content": [{"type": "text", "text": "Error: missing tool name"}],
                "isError": true,
            });
        }

        // Clone Arc before read to avoid Send bound on RwLockReadGuard
        let tools = self.tools.clone();
        let result = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                std::thread::spawn(move || {
                    let registry = tools.read().unwrap_or_else(|e| e.into_inner());
                    handle.block_on(registry.execute(&name, arguments))
                })
                .join()
                .unwrap_or_else(|_| Err(AetherError::ToolExecutionError("工具执行线程 panic".into())))
            }
            Err(_) => match tokio::runtime::Runtime::new() {
                Ok(rt) => {
                    let registry = tools.read().unwrap_or_else(|e| e.into_inner());
                    rt.block_on(registry.execute(&name, arguments))
                }
                Err(e) => Err(AetherError::ToolExecutionError(format!("tokio init: {}", e))),
            },
        };

        match result {
            Ok(output) => serde_json::json!({
                "content": [{"type": "text", "text": output}],
            }),
            Err(e) => serde_json::json!({
                "content": [{"type": "text", "text": format!("Error: {}", e)}],
                "isError": true,
            }),
        }
    }

    fn write_response(&self, w: &mut dyn Write, id: Option<&Value>, result: &Value) {
        let id = id.cloned().unwrap_or(Value::Null);
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        });
        let _ = writeln!(w, "{}", response);
        let _ = w.flush();
    }

    fn write_error(&self, w: &mut dyn Write, id: Option<&Value>, message: &str) {
        let id = id.cloned().unwrap_or(Value::Null);
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32603,
                "message": message,
            },
        });
        let _ = writeln!(w, "{}", response);
        let _ = w.flush();
    }
}

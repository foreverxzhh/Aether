use crate::error::AetherError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::Instant;

/// 工具抽象
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn toolset(&self) -> &str {
        "core"
    }
    fn description(&self) -> &str {
        ""
    }
    fn parameters(&self) -> Value {
        serde_json::json!({})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError>;
    fn is_available(&self) -> bool {
        true
    }
}

/// 工具注册表（用 Arc 避免跨 await 的锁问题）
pub struct ToolRegistry {
    tools: StdRwLock<HashMap<String, Arc<dyn Tool>>>,
    check_cache: StdRwLock<HashMap<String, (bool, Instant)>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: StdRwLock::new(HashMap::new()),
            check_cache: StdRwLock::new(HashMap::new()),
        }
    }

    pub fn register<T: Tool + 'static>(&self, tool: T) {
        let name = tool.name().to_string();
        self.tools.write().unwrap().insert(name, Arc::new(tool));
    }

    /// 执行工具
    pub async fn execute(&self, name: &str, args: Value) -> Result<String, AetherError> {
        let tool_arc = {
            let guard = self
                .tools
                .read()
                .map_err(|e| AetherError::ToolExecutionError(format!("注册表锁错误: {}", e)))?;
            guard.get(name).cloned()
        };

        match tool_arc {
            Some(tool) => tool.call(args).await,
            None => Err(AetherError::ToolNotFound(name.to_string())),
        }
    }

    /// 获取所有可用工具定义
    pub fn get_definitions(&self) -> Vec<Value> {
        let guard = match self.tools.read() {
            Ok(g) => g,
            Err(_) => return vec![],
        };

        guard
            .values()
            .filter(|t| t.is_available())
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name(),
                        "description": t.description(),
                        "parameters": t.parameters(),
                    }
                })
            })
            .collect()
    }

    pub fn count(&self) -> usize {
        self.tools.read().map(|g| g.len()).unwrap_or(0)
    }
}

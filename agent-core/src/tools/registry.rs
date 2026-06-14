use async_trait::async_trait;

/// 工具抽象
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn toolset(&self) -> &str { "core" }
    fn description(&self) -> &str { "" }
    fn parameters(&self) -> serde_json::Value { serde_json::json!({}) }
    async fn call(&self, args: serde_json::Value) -> Result<String, crate::error::AetherError>;
    fn is_available(&self) -> bool { true }
}

/// 工具注册表
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// 注册工具
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// 按名称查找工具
    pub fn find(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.iter().find(|t| t.name() == name).map(|t| t.as_ref())
    }

    /// 获取所有已注册并可用的工具
    pub fn available_tools(&self) -> Vec<&dyn Tool> {
        self.tools.iter()
            .map(|t| t.as_ref())
            .filter(|t| t.is_available())
            .collect()
    }

    /// 工具数量
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::error::AetherError;

    struct EchoTool;
    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        async fn call(&self, args: serde_json::Value) -> Result<String, AetherError> {
            Ok(serde_json::to_string(&args).unwrap_or_default())
        }
    }

    #[tokio::test]
    async fn test_register_and_find() {
        let mut r = ToolRegistry::new();
        r.register(Box::new(EchoTool));
        assert_eq!(r.count(), 1);
        assert!(r.find("echo").is_some());
        assert!(r.find("nonexistent").is_none());
    }
}

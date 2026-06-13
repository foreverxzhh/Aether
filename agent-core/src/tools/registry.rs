use super::Tool;

/// 工具注册表（TODO: Phase 4 实现完整版本）
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

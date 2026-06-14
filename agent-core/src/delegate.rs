use crate::AIAgent;
use crate::error::AetherError;

/// 子 Agent 委托
pub struct Delegation;

impl Delegation {
    /// 创建一个子 Agent 并执行任务
    pub async fn delegate(
        _parent: &AIAgent,
        _goal: &str,
        _context: Option<&str>,
    ) -> Result<String, AetherError> {
        // TODO: Phase 7 实现完整委托
        Err(AetherError::ConfigError("子 Agent 委托尚未实现".to_string()))
    }
}

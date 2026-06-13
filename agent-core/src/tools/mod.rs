pub mod registry;

use async_trait::async_trait;
use crate::error::AetherError;
use serde_json::Value;

/// 工具抽象
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称
    fn name(&self) -> &str;

    /// 所属工具集
    fn toolset(&self) -> &str;

    /// 描述（发给模型的）
    fn description(&self) -> &str;

    /// JSON Schema 参数定义
    fn parameters(&self) -> Value;

    /// 执行工具
    async fn call(&self, args: Value) -> Result<String, AetherError>;

    /// 可用性检查（如 API Key 是否配置）
    fn is_available(&self) -> bool {
        true
    }
}

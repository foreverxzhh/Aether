use thiserror::Error;

/// 统一的 Aether 错误枚举
#[derive(Error, Debug, Clone)]
pub enum AetherError {
    // ── LLM 相关 ──
    #[error("[AE001] LLM 调用失败: {0}")]
    LlmError(String),

    #[error("[AE002] LLM 返回空响应")]
    LlmEmptyResponse,

    #[error("[AE003] LLM 响应解析失败: {0}")]
    LlmParseError(String),

    #[error("[AE004] 模型不支持工具调用")]
    NoToolSupport,

    #[error("[AE005] 上下文超限: {0}")]
    ContextOverflow(String),

    // ── 工具相关 ──
    #[error("[TE001] 工具未找到: {0}")]
    ToolNotFound(String),

    #[error("[TE002] 工具执行失败: {0}")]
    ToolExecutionError(String),

    #[error("[TE003] 工具参数校验失败: {0}")]
    ToolInvalidArgs(String),

    #[error("[TE004] 工具不可用: {0}")]
    ToolUnavailable(String),

    #[error("[TE005] 熔断器触发: 工具 {0} 连续 {1} 次调用签名相同")]
    CircuitBreakerTripped(String, u32),

    // ── Agent 引擎 ──
    #[error("[AE101] Agent 配置错误: {0}")]
    ConfigError(String),

    #[error("[AE102] 迭代预算耗尽")]
    BudgetExhausted,

    #[error("[AE103] Agent 被中断")]
    Interrupted,

    #[error("[AE104] 最大迭代次数耗尽 ({0})")]
    MaxIterationsReached(u32),

    #[error("[AE105] 会话未找到: {0}")]
    SessionNotFound(String),

    #[error("[AE106] 子 Agent 委托深度超限 ({0})")]
    MaxSpawnDepthExceeded(u32),

    // ── MCP 相关 ──
    #[error("[ME001] MCP 连接失败: {0}")]
    McpConnectionError(String),

    #[error("[ME002] MCP 服务器错误: {0}")]
    McpServerError(String),

    #[error("[ME003] MCP JSON-RPC 解析失败: {0}")]
    McpParseError(String),

    // ── 存储相关 ──
    #[error("[SE001] 数据库错误: {0}")]
    DatabaseError(String),

    #[error("[SE002] 内存不足: {0}")]
    OutOfMemory(String),

    // ── 系统 ──
    #[error("[OE001] I/O 错误: {0}")]
    IoError(String),

    #[error("[OE002] 序列化错误: {0}")]
    SerdeError(String),

    #[error("[OE003] 不支持的 API 模式: {0}")]
    UnsupportedApiMode(String),

    #[error("[OE004] Profile 不存在: {0}")]
    ProfileNotFound(String),
}

impl From<std::io::Error> for AetherError {
    fn from(e: std::io::Error) -> Self {
        AetherError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for AetherError {
    fn from(e: serde_json::Error) -> Self {
        AetherError::SerdeError(e.to_string())
    }
}

#[cfg(feature = "rusqlite")]
impl From<rusqlite::Error> for AetherError {
    fn from(e: rusqlite::Error) -> Self {
        AetherError::DatabaseError(e.to_string())
    }
}

#[cfg(feature = "reqwest")]
impl From<reqwest::Error> for AetherError {
    fn from(e: reqwest::Error) -> Self {
        AetherError::LlmError(format!("HTTP 请求失败: {}", e))
    }
}

/// 错误码前缀分类
impl AetherError {
    pub fn category(&self) -> &str {
        match self {
            Self::LlmError(_)
            | Self::LlmEmptyResponse
            | Self::LlmParseError(_)
            | Self::NoToolSupport
            | Self::ContextOverflow(_) => "LLM",

            Self::ToolNotFound(_)
            | Self::ToolExecutionError(_)
            | Self::ToolInvalidArgs(_)
            | Self::ToolUnavailable(_)
            | Self::CircuitBreakerTripped(..) => "TOOL",

            Self::ConfigError(_)
            | Self::BudgetExhausted
            | Self::Interrupted
            | Self::MaxIterationsReached(_)
            | Self::SessionNotFound(_)
            | Self::MaxSpawnDepthExceeded(_) => "AGENT",

            Self::McpConnectionError(_) | Self::McpServerError(_) | Self::McpParseError(_) => "MCP",

            Self::DatabaseError(_) | Self::OutOfMemory(_) => "STORAGE",

            Self::IoError(_)
            | Self::SerdeError(_)
            | Self::UnsupportedApiMode(_)
            | Self::ProfileNotFound(_) => "SYSTEM",
        }
    }
}

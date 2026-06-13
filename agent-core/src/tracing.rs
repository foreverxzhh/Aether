use tracing_subscriber::{fmt, EnvFilter, Registry};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;

/// 初始化 tracing 日志系统
pub fn init_tracing(level: &str) {
    let filter = EnvFilter::builder()
        .with_default_directive(
            level.parse().unwrap_or(LevelFilter::INFO.into())
        )
        .from_env_lossy();

    let _subscriber = Registry::default()
        .with(filter)
        .with(fmt::Layer::default()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true));

    if tracing_subscriber::registry()
        .try_init()
        .is_err()
    {
        // 已经初始化过，忽略
    }
}

/// 创建 span 的辅助宏
#[macro_export]
macro_rules! agent_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::INFO, $name)
    };
    ($name:expr, $($key:ident = $value:expr),*) => {
        tracing::span!(tracing::Level::INFO, $name, $($key = %$value),*)
    };
}

/// LLM 调用 span
#[macro_export]
macro_rules! llm_span {
    ($provider:expr, $model:expr) => {
        tracing::span!(
            tracing::Level::INFO,
            "llm_call",
            provider = %$provider,
            model = %$model
        )
    };
}

/// 工具调用 span
#[macro_export]
macro_rules! tool_span {
    ($name:expr) => {
        tracing::span!(
            tracing::Level::INFO,
            "tool_call",
            tool = %$name
        )
    };
}

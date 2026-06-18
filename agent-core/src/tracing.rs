//! R-1.4 + H3: tracing 初始化 — 链式调用保证 filter 真生效
//!
//! v1 (M1) 曾构造 `let _subscriber = Registry::...` 后 drop，
//! 导致 `log_level` 配置完全无效。
//! v2 用 `Registry::default().with(filter).with(layer).try_init()` 链式调用。

use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter, Registry};

/// 初始化 tracing 日志系统
///
/// 先尝试从 `RUST_LOG` 环境变量读取，fallback 到 config.log_level。
/// 输出到 stdout，含 target / thread / file / line 信息。
pub fn init_tracing(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    Registry::default()
        .with(filter)
        .with(
            fmt::Layer::default()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
        .try_init()
        .ok();
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

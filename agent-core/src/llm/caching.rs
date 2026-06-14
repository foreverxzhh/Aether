/// Prompt Caching 支持
use crate::types::message::Message;

/// 缓存断点跟踪
pub struct CacheTracker {
    last_breakpoint: u32,
    pub is_dirty: bool,
}

impl CacheTracker {
    pub fn new() -> Self {
        Self {
            last_breakpoint: 0,
            is_dirty: false,
        }
    }

    /// 检测是否需要插入缓存断点（每 1000 token 一个）
    pub fn needs_breakpoint(&self, current_tokens: u32) -> bool {
        current_tokens - self.last_breakpoint >= 1000
    }

    pub fn mark_breakpoint(&mut self, tokens: u32) {
        self.last_breakpoint = tokens;
    }

    /// 标记缓存失效（切换工具集/重载记忆时调用）
    pub fn invalidate(&mut self) {
        self.is_dirty = true;
        self.last_breakpoint = 0;
    }

    /// 标记会话不再可缓存
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// 为 Anthropic API 添加 cache_control 标记
    pub fn apply_cache_control(headers: &mut Vec<(String, String)>) {
        headers.push(("anthropic-cache-control".into(), "true".into()));
    }

    /// 检查是否应该阻止某次操作（缓存安全检查）
    pub fn would_break_cache(operation: &str) -> bool {
        matches!(
            operation,
            "reload_memory" | "switch_tools" | "rebuild_system_prompt"
        )
    }
}

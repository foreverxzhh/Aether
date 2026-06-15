use std::collections::HashMap;
use std::sync::Mutex;

/// 熔断器：检测相同工具签名连续调用
#[derive(Debug)]
pub struct CircuitBreaker {
    threshold: u32,
    history: Mutex<HashMap<String, u32>>,
}

impl CircuitBreaker {
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold,
            history: Mutex::new(HashMap::new()),
        }
    }

    /// 检查工具调用是否触发熔断
    /// 返回 true = 已熔断（应该阻止调用）
    pub fn check(&self, tool_name: &str, args: &serde_json::Value) -> bool {
        let signature = format!(
            "{}:{}",
            tool_name,
            serde_json::to_string(args).unwrap_or_default()
        );
        let hash = signature_hash(&signature);

        let mut hist = self.history.lock().unwrap();
        let count = hist.entry(hash.clone()).or_insert(0);
        *count += 1;

        if *count >= self.threshold {
            return true; // 熔断
        }
        false
    }

    /// 重置（新的一轮对话）
    pub fn reset(&self) {
        let mut hist = self.history.lock().unwrap();
        hist.clear();
    }
}

/// 签名哈希（SipHash via DefaultHasher，非MD5。跨进程不保证稳定，仅用于同进程防重复）
fn signature_hash(input: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_breaker() {
        let b = CircuitBreaker::new(3);
        let args = json!({"query": "hello"});
        assert!(!b.check("test_tool", &args)); // 1st
        assert!(!b.check("test_tool", &args)); // 2nd
        assert!(b.check("test_tool", &args)); // 3rd → 触发熔断
        assert!(b.check("test_tool", &args)); // 4th → 仍熔断
    }

    #[test]
    fn test_different_args_no_trigger() {
        let b = CircuitBreaker::new(3);
        assert!(!b.check("tool", &json!({"x": 1})));
        assert!(!b.check("tool", &json!({"x": 2})));
        assert!(!b.check("tool", &json!({"x": 3})));
        assert!(!b.check("tool", &json!({"x": 1}))); // 不同参数，不会连续≥3次相同
    }

    #[test]
    fn test_reset() {
        let b = CircuitBreaker::new(2);
        let args = json!({"a": 1});
        b.check("t", &args);
        b.reset();
        assert!(!b.check("t", &args)); // reset 后重新计数
    }
}

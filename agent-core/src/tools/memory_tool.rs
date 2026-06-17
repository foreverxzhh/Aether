use super::Tool;
use crate::error::AetherError;
use crate::memory::core::{default_hermes_home, CoreMemory, UserProfile};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::LazyLock;

/// 记忆工具（读写 L1-L2 记忆）
///
/// T-1.4: 持有 profile-aware 的 `hermes_home`。
/// `None` → fallback 到 `default_hermes_home()`（用于兼容老调用方）。
pub struct Memory {
    hermes_home: Option<PathBuf>,
}

impl Memory {
    pub fn new(hermes_home: Option<PathBuf>) -> Self {
        Self { hermes_home }
    }

    fn home(&self) -> PathBuf {
        self.hermes_home
            .clone()
            .unwrap_or_else(default_hermes_home)
    }
}

/// T-3.9 v2: 脱敏 secret + 去重
///
/// 与 v1 的区别：扩展正则覆盖 Anthropic / OpenAI project / GitHub fine-grained
/// PAT / AWS access key / Google API / GitLab / Slack / JWT / 以及 PEM 私钥块。
/// 顺序敏感：把更具体的前缀放在通用 `sk-` 之前，避免 `sk-ant-...` 被通用规则
/// 提前截短。
static SECRET_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"sk-ant-[A-Za-z0-9_\-]{20,}",                       // Anthropic
        r"sk-proj-[A-Za-z0-9_\-]{20,}",                      // OpenAI project keys
        r"sk-[A-Za-z0-9]{20,}",                              // OpenAI / 通用 sk- token
        r"ghp_[A-Za-z0-9]{30,}",                             // GitHub personal
        r"gho_[A-Za-z0-9]{30,}",                             // GitHub OAuth
        r"github_pat_[A-Za-z0-9_]{50,}",                     // GitHub fine-grained PAT
        r"AKIA[0-9A-Z]{16}",                                 // AWS access key ID
        r"AIza[0-9A-Za-z_\-]{35}",                           // Google API key
        r"ya29\.[0-9A-Za-z_\-]{20,}",                        // Google OAuth token
        r"glpat-[0-9a-zA-Z_\-]{20,}",                        // GitLab PAT
        r"xox[baprs]-[0-9A-Za-z_\-]{10,}",                   // Slack token
        r"eyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}", // JWT
    ]
    .iter()
    .map(|p| Regex::new(p).expect("static secret regex"))
    .collect()
});

static PRIVATE_KEY_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN [A-Z ]+PRIVATE KEY-----.*?-----END [A-Z ]+PRIVATE KEY-----")
        .expect("static PEM regex")
});

pub fn redact_secrets(s: &str) -> String {
    let mut out = s.to_string();
    for pat in SECRET_PATTERNS.iter() {
        out = pat.replace_all(&out, "<redacted-secret>").to_string();
    }
    out = PRIVATE_KEY_BLOCK
        .replace_all(&out, "<redacted-private-key>")
        .to_string();
    out
}

/// T-3.9: 去重检查（最近几行已存在则跳过写入）
fn is_duplicate(existing: &str, new_line: &str) -> bool {
    let trimmed = new_line.trim();
    if trimmed.is_empty() { return true; }
    existing.lines().any(|l| l.trim() == trimmed)
}

const MEMORY_MAX_BYTES: usize = 64 * 1024; // T-3.9: 限制记忆文件大小

#[async_trait]
impl Tool for Memory {
    fn name(&self) -> &str {
        "memory"
    }
    fn toolset(&self) -> &str {
        "memory"
    }
    fn description(&self) -> &str {
        "读写长期记忆（跨会话保持，存储在 MEMORY.md 和 USER.md）"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string", "enum": ["read", "write", "search"],
                    "description": "操作类型: read=读取全部记忆, write=写入新记忆, search=搜索记忆"
                },
                "key": {"type": "string", "description": "记忆类型: memory(MEMORY.md) 或 profile(USER.md)"},
                "value": {"type": "string", "description": "要写入的内容（write 时必填）"},
                "query": {"type": "string", "description": "搜索关键词（search 时必填）"}
            },
            "required": ["action"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 action 参数".into()))?;
        let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("memory");
        let hermes_home = self.home();

        let content = match action {
            "read" => {
                if key == "memory" || key == "all" {
                    let core = CoreMemory::new(&hermes_home);
                    let mem = core.read().unwrap_or_default();
                    let profile = UserProfile::new(&hermes_home);
                    let user = profile.read().unwrap_or_default();
                    json!({"memory": mem, "profile": user})
                } else if key == "profile" {
                    let profile = UserProfile::new(&hermes_home);
                    json!({"profile": profile.read().unwrap_or_default()})
                } else {
                    json!({"error": format!("未知的 key: {}", key)})
                }
            }
            "write" => {
                let value = args.get("value").and_then(|v| v.as_str()).ok_or(
                    AetherError::ToolInvalidArgs("write 操作需要 value 参数".into()),
                )?;
                if key == "memory" || key == "all" {
                    let core = CoreMemory::new(&hermes_home);
                    let mut existing = core.read().unwrap_or_default();
                    let safe_value = redact_secrets(value);
                    if !is_duplicate(&existing, &safe_value) {
                        let new_line = format!("- {}", safe_value);
                        existing.push_str(&new_line);
                        existing.push('\n');
                        // T-3.9: 截断过大文件
                        if existing.len() > MEMORY_MAX_BYTES {
                            existing = existing[existing.len() - MEMORY_MAX_BYTES..].to_string();
                        }
                        core.write(&existing)?;
                    }
                }
                if key == "profile" || key == "all" {
                    let profile = UserProfile::new(&hermes_home);
                    let mut existing = profile.read().unwrap_or_default();
                    let safe_value = redact_secrets(value);
                    if !is_duplicate(&existing, &safe_value) {
                        existing.push_str(&format!("- {}\n", safe_value));
                        if existing.len() > MEMORY_MAX_BYTES {
                            existing = existing[existing.len() - MEMORY_MAX_BYTES..].to_string();
                        }
                        profile.write(&existing)?;
                    }
                }
                json!({"success": true})
            }
            "search" => {
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let core = CoreMemory::new(&hermes_home);
                let mem = core.read().unwrap_or_default();
                let results: Vec<&str> = mem
                    .lines()
                    .filter(|l| l.to_lowercase().contains(&query.to_lowercase()))
                    .collect();
                json!({"results": results, "count": results.len()})
            }
            _ => {
                return Err(AetherError::ToolInvalidArgs(format!(
                    "不支持的动作: {}",
                    action
                )))
            }
        };

        Ok(content.to_string())
    }
}

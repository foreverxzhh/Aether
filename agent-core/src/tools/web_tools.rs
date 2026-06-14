use async_trait::async_trait;
use serde_json::{json, Value};
use crate::error::AetherError;
use super::Tool;

/// 检查 URL 是否安全（防 SSRF）
fn is_url_safe(url_str: &str) -> Result<(), AetherError> {
    let url_lower = url_str.to_lowercase();
    // 阻止内网地址和云元数据端点
    let blocked = [
        "127.0.0.1", "localhost", "0.0.0.0", "::1",
        "169.254.169.254", // AWS/云元数据
        "10.", "172.16.", "172.17.", "172.18.", "172.19.",
        "172.20.", "172.21.", "172.22.", "172.23.", "172.24.",
        "172.25.", "172.26.", "172.27.", "172.28.", "172.29.",
        "172.30.", "172.31.", "192.168.",
        "metadata.google.internal", "metadata",
        "file://", "gopher://", "ftp://",
    ];
    for b in &blocked {
        if url_lower.contains(b) {
            return Err(AetherError::ToolExecutionError(
                format!("URL 被阻止（内网地址/不安全协议）: 匹配 '{}'", b)
            ));
        }
    }
    // 只允许 http/https
    if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
        return Err(AetherError::ToolExecutionError("只允许 http/https 协议".into()));
    }
    Ok(())
}

/// Web 搜索
pub struct WebSearch;
#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &str { "web_search" }
    fn description(&self) -> &str { "搜索互联网信息" }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let query = args.get("query").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 query 参数".into())
        )?;
        let encoded: String = query.chars().map(|c| match c {
            'A'..='Z'|'a'..='z'|'0'..='9'|'-'|'_'|'.'|'~' => c.to_string(),
            ' ' => "+".to_string(), _ => format!("%{:02X}", c as u8),
        }).collect();
        let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1", encoded);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15)).build()
            .map_err(|e| AetherError::ToolExecutionError(e.to_string()))?;
        let resp = client.get(&url).header("User-Agent","Aether/0.1").send().await
            .map_err(|e| AetherError::ToolExecutionError(format!("搜索失败: {}",e)))?;
        resp.text().await.map_err(|e| AetherError::ToolExecutionError(format!("读取失败: {}",e)))
    }
}

/// Web 页面抓取
pub struct WebExtract;
#[async_trait]
impl Tool for WebExtract {
    fn name(&self) -> &str { "web_extract" }
    fn description(&self) -> &str { "抓取网页内容并提取可读文本" }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let url = args.get("url").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 url 参数".into())
        )?;
        is_url_safe(url)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30)).build()
            .map_err(|e| AetherError::ToolExecutionError(e.to_string()))?;
        let resp = client.get(url).header("User-Agent",
            "Mozilla/5.0 (Aether/0.1)").send().await
            .map_err(|e| AetherError::ToolExecutionError(format!("请求失败: {}",e)))?;
        let html = resp.text().await
            .map_err(|e| AetherError::ToolExecutionError(format!("读取失败: {}",e)))?;

        let doc = scraper::Html::parse_document(&html);
        let mut text = String::new();
        if let Ok(sel) = scraper::Selector::parse("title") {
            if let Some(el) = doc.select(&sel).next() {
                text.push_str(&format!("# {}\n\n", el.text().collect::<String>()));
            }
        }
        if let Ok(sel) = scraper::Selector::parse("p,h1,h2,h3,li,pre,td") {
            for e in doc.select(&sel) {
                let inner = e.text().collect::<String>().trim().to_string();
                if !inner.is_empty() { text.push_str(&inner); text.push('\n'); }
            }
        }
        if text.len() > 10000 { text.truncate(10000); text.push_str("\n..."); }
        if text.is_empty() { text = "[页面无可见文本内容]".to_string(); }
        Ok(text)
    }
}

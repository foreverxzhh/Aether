use async_trait::async_trait;
use serde_json::{json, Value};
use crate::error::AetherError;
use super::Tool;

/// Web 搜索（通过 DuckDuckGo 或 API）
pub struct WebSearch;

#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &str { "web_search" }
    fn description(&self) -> &str { "搜索互联网信息" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "搜索关键词"}
            },
            "required": ["query"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let query = args.get("query").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 query 参数".into())
        )?;

        // 尝试用 DuckDuckGo 搜索
        let url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1", urlencoding(query));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| AetherError::ToolExecutionError(e.to_string()))?;

        let resp = client.get(&url)
            .header("User-Agent", "Aether/0.1")
            .send()
            .await
            .map_err(|e| AetherError::ToolExecutionError(format!("搜索请求失败: {}", e)))?;

        let text = resp.text().await
            .map_err(|e| AetherError::ToolExecutionError(format!("读取响应失败: {}", e)))?;

        Ok(text)
    }
}

/// Web 页面抓取
pub struct WebExtract;

#[async_trait]
impl Tool for WebExtract {
    fn name(&self) -> &str { "web_extract" }
    fn description(&self) -> &str { "抓取网页内容并提取可读文本" }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "网页 URL"}
            },
            "required": ["url"]
        })
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let url = args.get("url").and_then(|v| v.as_str()).ok_or(
            AetherError::ToolInvalidArgs("缺少 url 参数".into())
        )?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AetherError::ToolExecutionError(e.to_string()))?;

        let resp = client.get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| AetherError::ToolExecutionError(format!("请求失败: {}", e)))?;

        let html = resp.text().await
            .map_err(|e| AetherError::ToolExecutionError(format!("读取响应失败: {}", e)))?;

        // 用 scraper 提取文本
        let doc = scraper::Html::parse_document(&html);
        let mut text = String::new();

        // 提取标题
        if let Ok(sel) = scraper::Selector::parse("title") {
            if let Some(el) = doc.select(&sel).next() {
                text.push_str(&format!("# {}\n\n", el.text().collect::<String>()));
            }
        }

        // 提取正文段落
        if let Ok(sel) = scraper::Selector::parse("p, h1, h2, h3, li, pre, td") {
            for element in doc.select(&sel) {
                let inner = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if !inner.is_empty() {
                    text.push_str(&inner);
                    text.push('\n');
                }
            }
        }

        if text.is_empty() {
            // 降级：body 全文
            if let Ok(body_sel) = scraper::Selector::parse("body") {
                text = doc.select(&body_sel)
                    .next()
                    .map(|e| e.text().collect::<Vec<_>>().join(" "))
                    .unwrap_or_default();
            }
        }

        if text.len() > 10000 { text.truncate(10000); text.push_str("\n..."); }
        if text.is_empty() { text = "[页面无可见文本内容]".to_string(); }
        Ok(text)
    }
}

fn urlencoding(s: &str) -> String {
    s.chars().map(|c| match c {
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
        ' ' => "+".to_string(),
        _ => format!("%{:02X}", c as u8),
    }).collect()
}

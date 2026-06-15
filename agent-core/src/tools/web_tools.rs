use super::Tool;
use crate::error::AetherError;
use async_trait::async_trait;
use serde_json::{json, Value};

/// T-3.8: 检查 URL 是否安全（防 SSRF）— URL解析 + IP检查 + 阻止编码绕过
fn is_url_safe(url_str: &str) -> Result<(), AetherError> {
    // 只允许 http/https
    let url_lower = url_str.to_lowercase();
    if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
        return Err(AetherError::ToolExecutionError("只允许 http/https 协议".into()));
    }
    // 提取 host（基本解析：截取://和下一个/之间）
    let host_start = url_str.find("://").map(|i| i + 3).unwrap_or(0);
    let host_end = url_str[host_start..].find('/').map(|i| host_start + i).unwrap_or(url_str.len());
    let host_with_port = &url_str[host_start..host_end];
    let host = host_with_port.split(':').next().unwrap_or(host_with_port);
    let host_lower = host.to_lowercase();

    // 阻止 localhost 和特殊主机名
    if host_lower == "localhost" || host_lower == "metadata.google.internal" {
        return Err(AetherError::ToolExecutionError(format!("URL 被阻止: {}", host)));
    }

    // 解析 IPv4（支持十进制/十六进制/八进制编码绕过）
    let ip_parts: Vec<&str> = host_lower.split('.').collect();
    if ip_parts.len() == 4 {
        if let (Some(a), Some(b), Some(c), Some(d)) = (
            parse_ip_part(ip_parts[0]), parse_ip_part(ip_parts[1]),
            parse_ip_part(ip_parts[2]), parse_ip_part(ip_parts[3]),
        ) {
            let is_private = match a {
                10 => true,
                127 => true,
                0 => true,
                169 if b == 254 => true,
                172 if (16..=31).contains(&b) => true,
                192 if b == 168 => true,
                _ => false,
            };
            if is_private {
                return Err(AetherError::ToolExecutionError(format!(
                    "内网地址被阻止: {}.{}.{}.{}", a, b, c, d
                )));
            }
        }
    }

    // 阻止云元数据 IP
    if host_lower.contains("169.254") || host_lower == "0.0.0.0" || host_lower == "::1" {
        return Err(AetherError::ToolExecutionError(format!("IP 被阻止: {}", host)));
    }

    Ok(())
}

/// 解析单个 IP 段（支持十进制/十六进制/八进制）
fn parse_ip_part(s: &str) -> Option<u8> {
    if s.is_empty() { return None; }
    if s.len() > 1 && s.starts_with('0') && !s.starts_with("0x") && !s.starts_with("0X") {
        // 八进制: 0开头
        u8::from_str_radix(s, 8).ok()
    } else if s.starts_with("0x") || s.starts_with("0X") {
        // 十六进制
        u8::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse::<u8>().ok()
    }
}

/// Web 搜索
pub struct WebSearch;
#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &str {
        "web_search"
    }
    fn description(&self) -> &str {
        "搜索互联网信息"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 query 参数".into()))?;
        let encoded: String = query
            .chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                ' ' => "+".to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect();
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1",
            encoded
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| AetherError::ToolExecutionError(e.to_string()))?;
        let resp = client
            .get(&url)
            .header("User-Agent", "Aether/0.1")
            .send()
            .await
            .map_err(|e| AetherError::ToolExecutionError(format!("搜索失败: {}", e)))?;
        resp.text()
            .await
            .map_err(|e| AetherError::ToolExecutionError(format!("读取失败: {}", e)))
    }
}

/// Web 页面抓取
pub struct WebExtract;
#[async_trait]
impl Tool for WebExtract {
    fn name(&self) -> &str {
        "web_extract"
    }
    fn description(&self) -> &str {
        "抓取网页内容并提取可读文本"
    }
    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]})
    }
    async fn call(&self, args: Value) -> Result<String, AetherError> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or(AetherError::ToolInvalidArgs("缺少 url 参数".into()))?;
        is_url_safe(url)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AetherError::ToolExecutionError(e.to_string()))?;
        let resp = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Aether/0.1)")
            .send()
            .await
            .map_err(|e| AetherError::ToolExecutionError(format!("请求失败: {}", e)))?;
        let html = resp
            .text()
            .await
            .map_err(|e| AetherError::ToolExecutionError(format!("读取失败: {}", e)))?;

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
                if !inner.is_empty() {
                    text.push_str(&inner);
                    text.push('\n');
                }
            }
        }
        if text.len() > 10000 {
            text.truncate(10000);
            text.push_str("\n...");
        }
        if text.is_empty() {
            text = "[页面无可见文本内容]".to_string();
        }
        Ok(text)
    }
}

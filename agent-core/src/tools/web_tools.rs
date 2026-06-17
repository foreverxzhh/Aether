use super::Tool;
use crate::error::AetherError;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use url::Url;

/// T-3.8 v2: 严格 SSRF 防御
///
/// 与 v1 的区别：
/// - 用 `url::Url::parse` 替代字符串切割，杜绝 `http://evil.com@127.0.0.1`、
///   `[::1]`、IDN、空 host 等绕过。
/// - 通过 `ToSocketAddrs` 真正做 DNS 解析，逐 IP 检查 private/loopback/
///   link-local/ULA。这样 `localtest.me`、`127.0.0.1.nip.io`、自建 A 记录
///   指向 169.254.169.254 等情形都会被拦下。
/// - 覆盖 IPv4 和 IPv6（含 v4-mapped、ULA、文档段）。
///
/// 旧的字符串级 IPv4 检查（含八进制/十六进制段）作为 best-effort 早期拒绝
/// 仍然保留，DNS 失败时不会让恶意输入溜走。
pub fn is_url_safe(url_str: &str) -> Result<(), AetherError> {
    // 1) 严格 URL 解析
    let url = Url::parse(url_str)
        .map_err(|e| AetherError::ToolExecutionError(format!("无效 URL: {}", e)))?;

    // 2) 协议白名单
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AetherError::ToolExecutionError(format!(
            "只允许 http/https 协议: {}",
            scheme
        )));
    }

    let host = url
        .host_str()
        .ok_or_else(|| AetherError::ToolExecutionError("URL 缺少 host".into()))?;
    let host_lower = host.to_lowercase();

    // 3) 显式黑名单 + 字符串级 IPv4 兜底（防 DNS 解析阶段无法判定的输入）
    if host_lower == "localhost"
        || host_lower == "metadata.google.internal"
        || host_lower.ends_with(".internal")
        || host_lower.ends_with(".local")
    {
        return Err(AetherError::ToolExecutionError(format!(
            "URL host 被阻止: {}",
            host
        )));
    }

    // 兼容老防线：直接写 IP 的形式做八进制/十六进制段解码后再判一次
    let ip_parts: Vec<&str> = host_lower.split('.').collect();
    if ip_parts.len() == 4 {
        if let (Some(a), Some(b), Some(c), Some(d)) = (
            parse_ip_part(ip_parts[0]),
            parse_ip_part(ip_parts[1]),
            parse_ip_part(ip_parts[2]),
            parse_ip_part(ip_parts[3]),
        ) {
            let v4 = Ipv4Addr::new(a, b, c, d);
            if is_private_or_local(&IpAddr::V4(v4)) {
                return Err(AetherError::ToolExecutionError(format!(
                    "内网/保留地址被阻止: {}",
                    v4
                )));
            }
        }
    }

    // 4) DNS 解析 — 拦截域名间接指向内网/元数据 IP 的情形
    let port = url.port_or_known_default().unwrap_or(80);
    // 注意 IPv6 字面量 host 已经包含中括号？url::Url::host_str 返回的是去括号后的
    // 形式，可以直接 format 成 `[v6]:port`。
    let socket_target = if host.contains(':') && !host.starts_with('[') {
        format!("[{}]:{}", host, port)
    } else {
        format!("{}:{}", host, port)
    };

    let resolved = socket_target.to_socket_addrs().map_err(|e| {
        AetherError::ToolExecutionError(format!("DNS 解析失败 {}: {}", host, e))
    })?;

    let mut saw_any = false;
    for addr in resolved {
        saw_any = true;
        let ip = addr.ip();
        if is_private_or_local(&ip) {
            return Err(AetherError::ToolExecutionError(format!(
                "URL 解析到内网/保留地址 {} -> {}",
                host, ip
            )));
        }
    }
    if !saw_any {
        return Err(AetherError::ToolExecutionError(format!(
            "URL host 无可达地址: {}",
            host
        )));
    }

    Ok(())
}

/// 判断 IP 是否属于不应外发的私有/保留段。
pub fn is_private_or_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let oct = v4.octets();
            v4.is_loopback()                          // 127.0.0.0/8
                || v4.is_private()                    // 10/8, 172.16/12, 192.168/16
                || v4.is_link_local()                 // 169.254/16（含云元数据）
                || v4.is_broadcast()                  // 255.255.255.255
                || v4.is_documentation()              // 192.0.2/24, 198.51.100/24, 203.0.113/24
                || v4.is_unspecified()                // 0.0.0.0
                // CGNAT 100.64.0.0/10
                || (oct[0] == 100 && (oct[1] & 0xC0) == 64)
                // 保留段 240.0.0.0/4（不包括广播）
                || (oct[0] & 0xF0) == 0xF0 && !v4.is_broadcast()
        }
        IpAddr::V6(v6) => {
            let seg = v6.segments();
            if v6.is_loopback() || v6.is_unspecified() {
                return true;
            }
            // ULA fc00::/7
            if (seg[0] & 0xfe00) == 0xfc00 {
                return true;
            }
            // link-local fe80::/10
            if (seg[0] & 0xffc0) == 0xfe80 {
                return true;
            }
            // documentation 2001:db8::/32
            if seg[0] == 0x2001 && seg[1] == 0x0db8 {
                return true;
            }
            // IPv4-mapped ::ffff:0:0/96 —— 用底层 segments 判断以兼容老编译器
            // 前 80 位为 0，segments[5] == 0xffff，后 32 位组成对应 v4。
            if seg[0] == 0
                && seg[1] == 0
                && seg[2] == 0
                && seg[3] == 0
                && seg[4] == 0
                && seg[5] == 0xffff
            {
                let mapped = Ipv4Addr::new(
                    (seg[6] >> 8) as u8,
                    (seg[6] & 0xff) as u8,
                    (seg[7] >> 8) as u8,
                    (seg[7] & 0xff) as u8,
                );
                if is_private_or_local(&IpAddr::V4(mapped)) {
                    return true;
                }
            }
            false
        }
    }
}

/// 解析单个 IP 段（支持十进制/十六进制/八进制）
fn parse_ip_part(s: &str) -> Option<u8> {
    if s.is_empty() {
        return None;
    }
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
    fn toolset(&self) -> &str {
        "web"
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
        // v2: WebSearch 也走 SSRF 检查，避免被环境变量/代理改写后打到内网。
        is_url_safe(&url)?;
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
    fn toolset(&self) -> &str {
        "web"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn rejects_loopback_v4_literal() {
        assert!(is_url_safe("http://127.0.0.1/").is_err());
        assert!(is_url_safe("http://127.1/").is_err()); // url crate 会展开
    }

    #[test]
    fn rejects_aws_metadata_ip() {
        let ip = IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254));
        assert!(is_private_or_local(&ip));
        assert!(is_url_safe("http://169.254.169.254/latest/meta-data/").is_err());
    }

    #[test]
    fn rejects_ipv6_loopback() {
        let ip = IpAddr::V6(Ipv6Addr::LOCALHOST);
        assert!(is_private_or_local(&ip));
        assert!(is_url_safe("http://[::1]/admin").is_err());
    }

    #[test]
    fn rejects_ipv4_mapped_v6_private() {
        // ::ffff:127.0.0.1
        let v6 = Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x7f00, 0x0001);
        assert!(is_private_or_local(&IpAddr::V6(v6)));
    }

    #[test]
    fn rejects_unsupported_scheme() {
        assert!(is_url_safe("file:///etc/passwd").is_err());
        assert!(is_url_safe("gopher://example.com/").is_err());
    }

    #[test]
    fn rejects_localhost_hostname() {
        assert!(is_url_safe("http://localhost/").is_err());
        assert!(is_url_safe("http://metadata.google.internal/").is_err());
    }
}

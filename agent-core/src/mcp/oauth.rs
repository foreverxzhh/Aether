//! R-3.2: MCP OAuth 2.1 + PKCE
//!
//! MCP Streamable HTTP 的 OAuth 认证层。
//! - OAuth 2.1 Authorization Code flow + PKCE (S256)
//! - Token 存储抽象（默认文件，可替换 keyring）
//! - 自动 refresh

use crate::error::AetherError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// ── Token 存储 trait ──

/// OAuth token 存储后端
///
/// 默认实现：[FileTokenStore]（~/.aether/oauth/）。
/// 可替换为 keyring（macOS Keychain / Windows Credential Manager）。
pub trait TokenStore: Send + Sync {
    fn save(&self, server_id: &str, token: &OAuthToken) -> Result<(), AetherError>;
    fn load(&self, server_id: &str) -> Result<Option<OAuthToken>, AetherError>;
    fn delete(&self, server_id: &str) -> Result<(), AetherError>;
}

// ── OAuth 数据结构 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>, // unix timestamp
    pub token_type: String,
}

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PkcePair {
    pub code_verifier: String,
    pub code_challenge: String,
}

// ── PKCE 工具 ──

/// 生成 PKCE code_verifier + code_challenge (S256)
pub fn generate_pkce() -> PkcePair {
    let verifier = random_base64url(43);
    let challenge = sha256_base64url(verifier.as_bytes());
    PkcePair {
        code_verifier: verifier,
        code_challenge: challenge,
    }
}

fn random_base64url(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
    base64_url(&bytes)
}

fn sha256_base64url(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    base64_url(&hasher.finalize())
}

fn base64_url(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// ── OAuth 流程 ──

/// 构建授权 URL（用户需在浏览器中打开）
pub fn build_auth_url(config: &OAuthConfig, pkce: &PkcePair) -> String {
    let redirect = config
        .redirect_uri
        .as_deref()
        .unwrap_or("http://localhost:0/callback");
    let scopes = config.scopes.join(" ");
    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256",
        config.auth_url, config.client_id, redirect, scopes, pkce.code_challenge
    )
}

/// 用 authorization code 换 token
pub async fn exchange_code(
    config: &OAuthConfig,
    code: &str,
    pkce: &PkcePair,
) -> Result<OAuthToken, AetherError> {
    let client = reqwest::Client::new();
    let redirect = config
        .redirect_uri
        .as_deref()
        .unwrap_or("http://localhost:0/callback");

    let mut body = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("redirect_uri", redirect.to_string()),
        ("code_verifier", pkce.code_verifier.clone()),
        ("client_id", config.client_id.clone()),
    ];

    if let Some(ref secret) = config.client_secret {
        body.push(("client_secret", secret.clone()));
    }

    let resp = client
        .post(&config.token_url)
        .form(&body)
        .send()
        .await
        .map_err(|e| AetherError::McpConnectionError(format!("Token request failed: {}", e)))?;

    let token: OAuthTokenResponse = resp
        .json()
        .await
        .map_err(|e| AetherError::McpParseError(format!("Token parse failed: {}", e)))?;

    let expires_at = token
        .expires_in
        .map(|secs| std::time::SystemTime::now())
        .and_then(|now| now.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|ts| ts.as_secs())
        .zip(token.expires_in)
        .map(|(now, secs)| now + secs as u64);

    Ok(OAuthToken {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at,
        token_type: token.token_type.unwrap_or_else(|| "Bearer".into()),
    })
}

/// Refresh token
pub async fn refresh_token(
    config: &OAuthConfig,
    refresh_token: &str,
) -> Result<OAuthToken, AetherError> {
    let client = reqwest::Client::new();
    let mut body = vec![
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("client_id", config.client_id.clone()),
    ];
    if let Some(ref secret) = config.client_secret {
        body.push(("client_secret", secret.clone()));
    }

    let resp = client
        .post(&config.token_url)
        .form(&body)
        .send()
        .await
        .map_err(|e| AetherError::McpConnectionError(format!("Refresh failed: {}", e)))?;

    let token: OAuthTokenResponse = resp
        .json()
        .await
        .map_err(|e| AetherError::McpParseError(format!("Refresh parse failed: {}", e)))?;

    Ok(OAuthToken {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at: None,
        token_type: token.token_type.unwrap_or_else(|| "Bearer".into()),
    })
}

/// 检查 token 是否过期
pub fn is_expired(token: &OAuthToken) -> bool {
    token.expires_at.map_or(false, |exp| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now >= exp.saturating_sub(60) // 60s buffer
    })
}

// ── 文件 Token 存储 ──

pub struct FileTokenStore {
    dir: PathBuf,
}

impl FileTokenStore {
    pub fn new() -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".aether").join("oauth");
        std::fs::create_dir_all(&dir).ok();
        // chmod 700 on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).ok();
        }
        Self { dir }
    }

    fn path(&self, server_id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", server_id))
    }
}

impl TokenStore for FileTokenStore {
    fn save(&self, server_id: &str, token: &OAuthToken) -> Result<(), AetherError> {
        let json =
            serde_json::to_string_pretty(token).map_err(|e| AetherError::IoError(e.to_string()))?;
        std::fs::write(self.path(server_id), json)
            .map_err(|e| AetherError::IoError(e.to_string()))?;
        // chmod 600 on unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(self.path(server_id), std::fs::Permissions::from_mode(0o600))
                .ok();
        }
        Ok(())
    }

    fn load(&self, server_id: &str) -> Result<Option<OAuthToken>, AetherError> {
        let p = self.path(server_id);
        if !p.exists() {
            return Ok(None);
        }
        let json =
            std::fs::read_to_string(&p).map_err(|e| AetherError::IoError(e.to_string()))?;
        serde_json::from_str(&json).map(Some).map_err(|e| {
            AetherError::McpParseError(format!("Token deserialize: {}", e))
        })
    }

    fn delete(&self, server_id: &str) -> Result<(), AetherError> {
        let p = self.path(server_id);
        if p.exists() {
            std::fs::remove_file(&p).map_err(|e| AetherError::IoError(e.to_string()))?;
        }
        Ok(())
    }
}

// ── 内部响应类型 ──

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = generate_pkce();
        assert!(!pkce.code_verifier.is_empty());
        assert!(!pkce.code_challenge.is_empty());
        assert_ne!(pkce.code_verifier, pkce.code_challenge);
        // verifier 43-128 chars
        assert!(pkce.code_verifier.len() >= 43);
    }

    #[test]
    fn test_build_auth_url() {
        let config = OAuthConfig {
            client_id: "test-client".into(),
            client_secret: None,
            auth_url: "https://auth.example.com/authorize".into(),
            token_url: "https://auth.example.com/token".into(),
            redirect_uri: Some("http://localhost:8080/cb".into()),
            scopes: vec!["read".into(), "write".into()],
        };
        let pkce = generate_pkce();
        let url = build_auth_url(&config, &pkce);
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test-client"));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn test_is_expired() {
        let token = OAuthToken {
            access_token: "tok".into(),
            refresh_token: None,
            expires_at: Some(1), // 1970 = definitely expired
            token_type: "Bearer".into(),
        };
        assert!(is_expired(&token));

        let future_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 3600;
        let valid = OAuthToken {
            access_token: "tok".into(),
            refresh_token: None,
            expires_at: Some(future_ts),
            token_type: "Bearer".into(),
        };
        assert!(!is_expired(&valid));
    }

    #[test]
    fn test_file_token_store_save_load_delete() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let store = FileTokenStore {
            dir: dir.path().to_path_buf(),
        };

        let token = OAuthToken {
            access_token: "test-access".into(),
            refresh_token: Some("test-refresh".into()),
            expires_at: None,
            token_type: "Bearer".into(),
        };

        store.save("test-server", &token).unwrap();
        let loaded = store.load("test-server").unwrap().unwrap();
        assert_eq!(loaded.access_token, "test-access");
        assert_eq!(loaded.refresh_token.unwrap(), "test-refresh");

        store.delete("test-server").unwrap();
        assert!(store.load("test-server").unwrap().is_none());
    }
}

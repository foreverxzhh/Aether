use super::SessionRecord;
use crate::error::AetherError;
use crate::types::message::Message;
use async_trait::async_trait;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub struct SqliteSessionStore {
    conn: Mutex<Connection>,
}

impl SqliteSessionStore {
    pub fn new(path: &Path) -> Result<Self, AetherError> {
        let conn = Connection::open(path)
            .map_err(|e| AetherError::DatabaseError(format!("打开数据库失败: {}", e)))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS sessions (
                 id TEXT PRIMARY KEY, parent_session_id TEXT,
                 source TEXT NOT NULL DEFAULT 'cli', model TEXT, provider TEXT, config TEXT,
                 created_at TEXT NOT NULL DEFAULT (datetime('now')),
                 updated_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE TABLE IF NOT EXISTS messages (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                 role TEXT NOT NULL, content TEXT, tool_calls TEXT, tool_call_id TEXT,
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(content, tool_calls, content=messages, content_rowid=id);
             -- T-2.3: FTS5 同步触发器
             CREATE TRIGGER IF NOT EXISTS msgs_ai AFTER INSERT ON messages BEGIN
                 INSERT INTO messages_fts(rowid, content, tool_calls) VALUES (new.id, new.content, new.tool_calls);
             END;
             CREATE TRIGGER IF NOT EXISTS msgs_ad AFTER DELETE ON messages BEGIN
                 INSERT INTO messages_fts(messages_fts, rowid, content, tool_calls) VALUES ('delete', old.id, old.content, old.tool_calls);
             END;
             CREATE TRIGGER IF NOT EXISTS msgs_au AFTER UPDATE ON messages BEGIN
                 INSERT INTO messages_fts(messages_fts, rowid, content, tool_calls) VALUES ('delete', old.id, old.content, old.tool_calls);
                 INSERT INTO messages_fts(rowid, content, tool_calls) VALUES (new.id, new.content, new.tool_calls);
             END;"
        ).map_err(|e| AetherError::DatabaseError(format!("建表失败: {}", e)))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn from_conn(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// 在 Mutex 内执行闭包
    fn with_conn<F, T>(&self, f: F) -> Result<T, AetherError>
    where
        F: FnOnce(&Connection) -> Result<T, AetherError>,
    {
        let guard = self
            .conn
            .lock()
            .map_err(|e| AetherError::DatabaseError(format!("数据库锁错误: {}", e)))?;
        f(&guard)
    }
}

fn to_role(s: &str) -> crate::types::message::MessageRole {
    match s {
        "system" => MessageRole::System,
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        _ => MessageRole::User,
    }
}
use crate::types::message::MessageRole;

#[async_trait]
impl super::SessionStore for SqliteSessionStore {
    async fn save_session(&self, session: &SessionRecord) -> Result<(), AetherError> {
        self.with_conn(|conn| {
            let config = serde_json::json!({"model": session.model, "provider": session.provider});
            conn.execute(
                "INSERT OR REPLACE INTO sessions (id, parent_session_id, source, model, provider, config)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![session.id, session.parent_session_id, session.source,
                        session.model, session.provider, config.to_string()],
            ).map_err(|e| AetherError::DatabaseError(e.to_string()))?;

            for msg in &session.messages {
                let content = match &msg.content {
                    Some(crate::types::message::Content::Text(t)) => t.clone(),
                    Some(crate::types::message::Content::Parts(_)) => "[多媒体]".to_string(),
                    None => String::new(),
                };
                let calls = msg.tool_calls.as_ref().map(|tc| serde_json::to_string(tc).unwrap_or_default());
                let role = match msg.role { MessageRole::System => "system", MessageRole::User => "user",
                    MessageRole::Assistant => "assistant", MessageRole::Tool => "tool" };
                conn.execute("INSERT INTO messages (session_id, role, content, tool_calls, tool_call_id) VALUES (?1,?2,?3,?4,?5)",
                    params![session.id, role, content, calls, msg.tool_call_id]).ok();
            }
            Ok(())
        })
    }

    async fn load_session(&self, session_id: &str) -> Result<SessionRecord, AetherError> {
        self.with_conn(|conn| {
            let sid = session_id.to_string();
            let mut stmt = conn.prepare(
                "SELECT id,parent_session_id,source,model,provider,created_at,updated_at FROM sessions WHERE id=?1"
            ).map_err(|e| AetherError::DatabaseError(e.to_string()))?;

            let mut session: Option<SessionRecord> = None;
            let rows = stmt.query_map(params![sid], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?, parent_session_id: row.get(1)?, source: row.get(2)?,
                    model: row.get(3)?, provider: row.get(4)?,
                    messages: vec![], created_at: row.get(5)?, updated_at: row.get(6)?,
                })
            }).map_err(|e| AetherError::DatabaseError(e.to_string()))?;

            for row in rows {
                session = Some(row.map_err(|e| AetherError::DatabaseError(e.to_string()))?);
                break;
            }
            let mut session = session.ok_or_else(|| AetherError::SessionNotFound(session_id.to_string()))?;

            let mut mstmt = conn.prepare(
                "SELECT role,content,tool_calls,tool_call_id FROM messages WHERE session_id=?1 ORDER BY id"
            ).map_err(|e| AetherError::DatabaseError(e.to_string()))?;

            let msgs = mstmt.query_map(params![session_id], |row| {
                let role: String = row.get(0)?;
                let content: Option<String> = row.get(1)?;
                let tc_json: Option<String> = row.get(2)?;
                let tcid: Option<String> = row.get(3)?;
                // T-2.3: 真解析 tool_calls（之前丢弃到 _tc）
                let tool_calls = tc_json.and_then(|s| serde_json::from_str(&s).ok());
                Ok(Message { role: to_role(&role), content: content.map(Content::Text), tool_calls, tool_call_id: tcid, name: None })
            }).map_err(|e| AetherError::DatabaseError(e.to_string()))?;
            use crate::types::message::Content;

            for m in msgs { if let Ok(m) = m { session.messages.push(m); } }
            Ok(session)
        })
    }

    async fn search_sessions(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SessionRecord>, AetherError> {
        self.with_conn(|conn| {
            // T-2.3 / FIX-5: 用 FTS5 MATCH 替换原来的 LIKE。
            // 双引号包裹整个 query 作为 phrase，并 escape 内部双引号，避免
            // 用户输入中的 ":" / "AND" / "*" 等被 FTS5 当成语法。
            let phrase = format!("\"{}\"", query.replace('"', "\"\""));

            let mut stmt = conn.prepare(
                "SELECT DISTINCT s.id, s.parent_session_id, s.source, s.model, s.provider, s.created_at, s.updated_at, \
                        bm25(messages_fts) AS score \
                 FROM messages_fts \
                 JOIN messages m ON m.id = messages_fts.rowid \
                 JOIN sessions s ON s.id = m.session_id \
                 WHERE messages_fts MATCH ?1 \
                 ORDER BY score LIMIT ?2"
            ).map_err(|e| AetherError::DatabaseError(e.to_string()))?;

            let rows = stmt.query_map(params![phrase, limit as i64], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?, parent_session_id: row.get(1)?, source: row.get(2)?,
                    model: row.get(3)?, provider: row.get(4)?,
                    messages: vec![], created_at: row.get(5)?, updated_at: row.get(6)?,
                })
            }).map_err(|e| AetherError::DatabaseError(e.to_string()))?;

            let mut results = Vec::new();
            for row in rows { if let Ok(s) = row { results.push(s); } }
            Ok(results)
        })
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), AetherError> {
        let sid = session_id.to_string();
        self.with_conn(|conn| {
            conn.execute("DELETE FROM messages WHERE session_id=?1", params![sid])
                .map_err(|e| AetherError::DatabaseError(e.to_string()))?;
            conn.execute("DELETE FROM sessions WHERE id=?1", params![sid])
                .map_err(|e| AetherError::DatabaseError(e.to_string()))?;
            Ok(())
        })
    }

    async fn list_sessions(&self) -> Result<Vec<String>, AetherError> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare("SELECT id FROM sessions ORDER BY updated_at DESC")
                .map_err(|e| AetherError::DatabaseError(e.to_string()))?;
            let ids = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| AetherError::DatabaseError(e.to_string()))?;
            let mut results = Vec::new();
            for id in ids {
                if let Ok(id) = id {
                    results.push(id);
                }
            }
            Ok(results)
        })
    }
}

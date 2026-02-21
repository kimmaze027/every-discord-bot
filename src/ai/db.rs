use rusqlite::{params, Connection};
use std::sync::Mutex;

pub struct ChatMessage {
    pub author_name: String,
    pub content: String,
    pub is_bot: bool,
}

pub struct ChatDb {
    conn: Mutex<Connection>,
}

impl ChatDb {
    pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id TEXT NOT NULL,
                author_id TEXT NOT NULL,
                author_name TEXT NOT NULL,
                content TEXT NOT NULL,
                is_bot INTEGER DEFAULT 0,
                has_image INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_messages_channel_time
                ON messages(channel_id, created_at);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_message(
        &self,
        channel_id: &str,
        author_id: &str,
        author_name: &str,
        content: &str,
        is_bot: bool,
        has_image: bool,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO messages (channel_id, author_id, author_name, content, is_bot, has_image)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                channel_id,
                author_id,
                author_name,
                content,
                is_bot as i32,
                has_image as i32
            ],
        )?;
        Ok(())
    }

    pub fn recent_messages(&self, channel_id: &str, limit: usize) -> Vec<ChatMessage> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT author_name, content, is_bot FROM messages
             WHERE channel_id = ?1
             ORDER BY id DESC LIMIT ?2",
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("DB 쿼리 준비 실패: {e}");
                return Vec::new();
            }
        };

        let rows = match stmt.query_map(params![channel_id, limit], |row| {
            Ok(ChatMessage {
                author_name: row.get(0)?,
                content: row.get(1)?,
                is_bot: row.get::<_, i32>(2)? != 0,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("DB 쿼리 실행 실패: {e}");
                return Vec::new();
            }
        };

        let mut messages: Vec<ChatMessage> = rows.filter_map(|r| r.ok()).collect();
        // DESC 순서를 시간순으로 뒤집기
        messages.reverse();
        messages
    }

    pub fn cleanup_old(&self, channel_id: &str, keep: usize) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM messages WHERE channel_id = ?1 AND id NOT IN (
                SELECT id FROM messages WHERE channel_id = ?1 ORDER BY id DESC LIMIT ?2
            )",
            params![channel_id, keep],
        );
    }
}

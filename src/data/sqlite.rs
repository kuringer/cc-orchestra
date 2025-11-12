use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Message {
    pub uuid: String,
    pub session_id: String,
    pub timestamp: i64,
    pub message_type: String,
    pub cwd: String,
}

pub struct ClaudeDb {
    conn: Connection,
}

impl ClaudeDb {
    pub fn new(path: PathBuf) -> Result<Self> {
        // TODO: implement
        Ok(Self {
            conn: Connection::open(path)?
        })
    }

    pub fn get_latest_message(&self, session_id: &str) -> Result<Message> {
        let mut stmt = self.conn.prepare(
            "SELECT uuid, session_id, timestamp, message_type, cwd
             FROM base_messages
             WHERE session_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1"
        )?;

        let msg = stmt.query_row(params![session_id], |row| {
            Ok(Message {
                uuid: row.get(0)?,
                session_id: row.get(1)?,
                timestamp: row.get(2)?,
                message_type: row.get(3)?,
                cwd: row.get(4)?,
            })
        })?;

        Ok(msg)
    }

    pub fn get_active_sessions(&self, since_hours: i64) -> Result<Vec<String>> {
        let cutoff = chrono::Utc::now().timestamp() * 1000 - (since_hours * 3600 * 1000);
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT session_id
             FROM base_messages
             WHERE timestamp > ?1"
        )?;

        let sessions = stmt.query_map(params![cutoff], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(sessions)
    }

    pub fn with_retry<F, T>(path: PathBuf, f: F) -> Result<T>
    where
        F: Fn(&Connection) -> Result<T>,
    {
        let max_attempts = 3;
        let mut attempts = 0;

        loop {
            match Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
                Ok(conn) => return f(&conn),
                Err(_e) if attempts < max_attempts => {
                    attempts += 1;
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

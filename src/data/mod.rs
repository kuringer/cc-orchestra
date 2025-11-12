pub mod sqlite;
pub mod process;
pub mod jsonl;

#[cfg(test)]
mod tests {
    use super::sqlite::*;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    fn create_test_db() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();

        conn.execute(
            "CREATE TABLE base_messages (
                uuid TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                message_type TEXT NOT NULL,
                cwd TEXT NOT NULL
            )",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO base_messages VALUES (?, ?, ?, ?, ?)",
            ["msg1", "session-1", "1762956296000", "user", "/tmp/project1"],
        ).unwrap();

        conn.execute(
            "INSERT INTO base_messages VALUES (?, ?, ?, ?, ?)",
            ["msg2", "session-1", "1762956300000", "assistant", "/tmp/project1"],
        ).unwrap();

        file
    }

    #[test]
    fn test_query_latest_message() {
        let db_file = create_test_db();
        let reader = ClaudeDb::new(db_file.path().to_path_buf()).unwrap();

        let msg = reader.get_latest_message("session-1").unwrap();
        assert_eq!(msg.message_type, "assistant");
        assert_eq!(msg.timestamp, 1762956300000);
    }
}

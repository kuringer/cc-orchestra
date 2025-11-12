use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionActivity {
    pub last_event_type: String,         // "user" or "assistant"
    pub last_content_type: Option<String>, // "tool_use", "text", "thinking"
    pub timestamp: i64,                   // Unix timestamp in milliseconds
}

#[derive(Debug, Deserialize)]
struct JsonlEvent {
    #[serde(rename = "type")]
    event_type: String,
    timestamp: String,
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
}

/// Sanitize CWD path to match Claude's directory naming convention
fn sanitize_cwd(cwd: &Path) -> String {
    let path_str = cwd.to_string_lossy();
    // Replace slashes with dashes and remove leading dash
    path_str.replace('/', "-").trim_start_matches('-').to_string()
}

/// Parse ISO 8601 timestamp to Unix milliseconds
fn parse_timestamp(timestamp: &str) -> Result<i64> {
    let dt = chrono::DateTime::parse_from_rfc3339(timestamp)?;
    Ok(dt.timestamp_millis())
}

/// Get the last content type from a message
fn extract_last_content_type(content: &serde_json::Value) -> Option<String> {
    // Content can be an array of content blocks
    if let Some(array) = content.as_array() {
        if let Some(last_block) = array.last() {
            if let Some(content_type) = last_block.get("type") {
                return content_type.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Read the last N lines from a file efficiently
fn read_last_n_lines(file_path: &Path, n: usize) -> Result<Vec<String>> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    // First, try to estimate where to start reading from
    // This is a simple optimization - we seek to a position roughly n*200 bytes from the end
    // (assuming average line length of ~200 bytes)
    let file_size = reader.seek(SeekFrom::End(0))?;
    let estimated_offset = file_size.saturating_sub((n * 200) as u64);
    reader.seek(SeekFrom::Start(estimated_offset))?;

    // Read all remaining lines
    let mut lines: Vec<String> = reader.lines()
        .filter_map(|line| line.ok())
        .collect();

    // If we didn't start at the beginning, we might have a partial line at the start
    // Remove it to be safe
    if estimated_offset > 0 && !lines.is_empty() {
        lines.remove(0);
    }

    // Take last n lines
    let start_idx = lines.len().saturating_sub(n);
    Ok(lines[start_idx..].to_vec())
}

/// Parse session JSONL file and return activity information
/// Uses a custom home directory if provided, otherwise uses $HOME
fn get_session_activity_with_home(session_id: &str, cwd: &Path, home_dir: Option<&Path>) -> Result<Option<SessionActivity>> {
    let home = if let Some(home) = home_dir {
        home.to_path_buf()
    } else {
        PathBuf::from(std::env::var("HOME")?)
    };

    let sanitized_cwd = sanitize_cwd(cwd);
    let jsonl_path = home
        .join(".claude/projects")
        .join(&sanitized_cwd)
        .join(format!("{}.jsonl", session_id));

    // Return None if file doesn't exist
    if !jsonl_path.exists() {
        return Ok(None);
    }

    // Read last 50 lines for performance
    let lines = read_last_n_lines(&jsonl_path, 50)?;

    // Parse lines in reverse to find the last valid event
    for line in lines.iter().rev() {
        if let Ok(event) = serde_json::from_str::<JsonlEvent>(line) {
            let last_content_type = extract_last_content_type(&event.message.content);
            let timestamp = parse_timestamp(&event.timestamp)?;

            return Ok(Some(SessionActivity {
                last_event_type: event.event_type,
                last_content_type,
                timestamp,
            }));
        }
        // Skip invalid lines and continue
    }

    // No valid events found
    Ok(None)
}

/// Parse session JSONL file and return activity information
pub fn get_session_activity(session_id: &str, cwd: &Path) -> Result<Option<SessionActivity>> {
    get_session_activity_with_home(session_id, cwd, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_jsonl(home_dir: &Path, cwd: &Path, session_id: &str, lines: &[&str]) -> PathBuf {
        // Create the directory structure that matches the actual Claude Code structure
        let sanitized_cwd = sanitize_cwd(cwd);
        let session_dir = home_dir.join(".claude/projects").join(&sanitized_cwd);
        std::fs::create_dir_all(&session_dir).unwrap();

        let jsonl_path = session_dir.join(format!("{}.jsonl", session_id));
        let mut file = File::create(&jsonl_path).unwrap();
        for line in lines {
            writeln!(file, "{}", line).unwrap();
        }
        jsonl_path
    }

    #[test]
    fn test_sanitize_cwd() {
        let cwd = PathBuf::from("/Users/test/project");
        assert_eq!(sanitize_cwd(&cwd), "Users-test-project");

        let cwd = PathBuf::from("/tmp/test");
        assert_eq!(sanitize_cwd(&cwd), "tmp-test");
    }

    #[test]
    fn test_parse_timestamp() {
        let ts = "2025-11-12T19:08:42.643Z";
        let result = parse_timestamp(ts).unwrap();
        assert!(result > 0);
        // Verify it's in the expected range (Nov 2025)
        assert!(result > 1731436000000); // Rough check for Nov 2025
    }

    #[test]
    fn test_extract_last_content_type() {
        let json = serde_json::json!([
            {"type": "thinking", "thinking": "test"},
            {"type": "tool_use", "name": "Bash"}
        ]);

        let content_type = extract_last_content_type(&json);
        assert_eq!(content_type, Some("tool_use".to_string()));

        let json = serde_json::json!([
            {"type": "text", "text": "hello"}
        ]);
        let content_type = extract_last_content_type(&json);
        assert_eq!(content_type, Some("text".to_string()));
    }

    #[test]
    fn test_get_session_activity_user_event() {
        let home_dir = TempDir::new().unwrap();
        let cwd = PathBuf::from("/tmp/test-project");
        let session_id = "test-session-1";

        let lines = vec![
            r#"{"type":"user","timestamp":"2025-11-12T19:08:42.643Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_123","content":"result"}]}}"#,
        ];

        setup_test_jsonl(home_dir.path(), &cwd, session_id, &lines);

        let activity = get_session_activity_with_home(session_id, &cwd, Some(home_dir.path())).unwrap();
        assert!(activity.is_some());

        let activity = activity.unwrap();
        assert_eq!(activity.last_event_type, "user");
        assert_eq!(activity.last_content_type, Some("tool_result".to_string()));
        assert!(activity.timestamp > 0);
    }

    #[test]
    fn test_get_session_activity_assistant_with_tool_use() {
        let home_dir = TempDir::new().unwrap();
        let cwd = PathBuf::from("/tmp/test-project");
        let session_id = "test-session-2";

        let lines = vec![
            r#"{"type":"assistant","timestamp":"2025-11-12T19:08:15.672Z","message":{"content":[{"type":"thinking","thinking":"test"},{"type":"tool_use","name":"Bash","id":"toolu_456"}]}}"#,
        ];

        setup_test_jsonl(home_dir.path(), &cwd, session_id, &lines);

        let activity = get_session_activity_with_home(session_id, &cwd, Some(home_dir.path())).unwrap();
        assert!(activity.is_some());

        let activity = activity.unwrap();
        assert_eq!(activity.last_event_type, "assistant");
        assert_eq!(activity.last_content_type, Some("tool_use".to_string()));
    }

    #[test]
    fn test_get_session_activity_assistant_with_text() {
        let home_dir = TempDir::new().unwrap();
        let cwd = PathBuf::from("/tmp/test-project");
        let session_id = "test-session-3";

        let lines = vec![
            r#"{"type":"assistant","timestamp":"2025-11-12T19:10:00.000Z","message":{"content":[{"type":"text","text":"Here is my response"}]}}"#,
        ];

        setup_test_jsonl(home_dir.path(), &cwd, session_id, &lines);

        let activity = get_session_activity_with_home(session_id, &cwd, Some(home_dir.path())).unwrap();
        assert!(activity.is_some());

        let activity = activity.unwrap();
        assert_eq!(activity.last_event_type, "assistant");
        assert_eq!(activity.last_content_type, Some("text".to_string()));
    }

    #[test]
    fn test_get_session_activity_multiple_events() {
        let home_dir = TempDir::new().unwrap();
        let cwd = PathBuf::from("/tmp/test-project");
        let session_id = "test-session-4";

        let lines = vec![
            r#"{"type":"user","timestamp":"2025-11-12T19:08:00.000Z","message":{"content":[{"type":"text","text":"hello"}]}}"#,
            r#"{"type":"assistant","timestamp":"2025-11-12T19:08:15.672Z","message":{"content":[{"type":"tool_use","name":"Bash"}]}}"#,
            r#"{"type":"user","timestamp":"2025-11-12T19:08:42.643Z","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_123"}]}}"#,
        ];

        setup_test_jsonl(home_dir.path(), &cwd, session_id, &lines);

        // Should return the LAST event
        let activity = get_session_activity_with_home(session_id, &cwd, Some(home_dir.path())).unwrap();
        assert!(activity.is_some());

        let activity = activity.unwrap();
        assert_eq!(activity.last_event_type, "user");
        assert_eq!(activity.last_content_type, Some("tool_result".to_string()));
    }

    #[test]
    fn test_get_session_activity_file_not_found() {
        let home_dir = TempDir::new().unwrap();
        let cwd = PathBuf::from("/tmp/test-project");
        let session_id = "nonexistent-session";

        let activity = get_session_activity_with_home(session_id, &cwd, Some(home_dir.path())).unwrap();
        assert!(activity.is_none());
    }

    #[test]
    fn test_get_session_activity_invalid_json() {
        let home_dir = TempDir::new().unwrap();
        let cwd = PathBuf::from("/tmp/test-project");
        let session_id = "test-session-invalid";

        let lines = vec![
            r#"invalid json line"#,
            r#"{"type":"assistant","timestamp":"2025-11-12T19:08:15.672Z","message":{"content":[{"type":"text","text":"valid"}]}}"#,
        ];

        setup_test_jsonl(home_dir.path(), &cwd, session_id, &lines);

        // Should skip invalid line and return valid event
        let activity = get_session_activity_with_home(session_id, &cwd, Some(home_dir.path())).unwrap();
        assert!(activity.is_some());

        let activity = activity.unwrap();
        assert_eq!(activity.last_event_type, "assistant");
    }

    #[test]
    fn test_read_last_n_lines() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();

        // Write 100 lines
        for i in 0..100 {
            writeln!(file, "line {}", i).unwrap();
        }

        // Read last 10 lines
        let lines = read_last_n_lines(&file_path, 10).unwrap();
        assert_eq!(lines.len(), 10);
        assert_eq!(lines[0], "line 90");
        assert_eq!(lines[9], "line 99");
    }

    #[test]
    fn test_read_last_n_lines_fewer_than_n() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();

        // Write only 5 lines
        for i in 0..5 {
            writeln!(file, "line {}", i).unwrap();
        }

        // Request last 10 lines
        let lines = read_last_n_lines(&file_path, 10).unwrap();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "line 0");
        assert_eq!(lines[4], "line 4");
    }
}

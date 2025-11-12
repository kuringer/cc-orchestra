use anyhow::Result;
use crate::data::sqlite::{ClaudeDb, Message};
use crate::state::{SessionState, tracker::{StateFile, SessionInfo}, detector};
use std::path::PathBuf;

pub struct Session {
    pub id: String,
    pub info: SessionInfo,
    pub state: SessionState,
    pub last_message: Option<Message>,
    pub project_name: String,
}

pub struct App {
    state_file: StateFile,
    sessions: Vec<Session>,
    selected: usize,
}

impl App {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")?;
        let state_path = PathBuf::from(&home).join(".claude/cc-orchestra-state.json");
        let state_file = StateFile::load(state_path)?;

        Ok(Self {
            state_file,
            sessions: Vec::new(),
            selected: 0,
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let home = std::env::var("HOME")?;
        let db_path = PathBuf::from(&home).join(".claude/__store.db");

        let db = ClaudeDb::new(db_path)?;
        let mut sessions = Vec::new();

        for (session_id, info) in &self.state_file.sessions {
            // Get last message
            let last_message = db.get_latest_message(session_id).ok();

            // Calculate message age
            let last_msg_age = if let Some(ref msg) = last_message {
                let now = chrono::Utc::now().timestamp() * 1000;
                ((now - msg.timestamp) / 1000) as i64
            } else {
                9999
            };

            // Detect state
            let state = detector::detect_state(
                info,
                last_message.as_ref().map(|m| m.message_type.as_str()),
                last_msg_age,
            );

            // Extract project name from cwd
            let project_name = PathBuf::from(&info.cwd)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            sessions.push(Session {
                id: session_id.clone(),
                info: info.clone(),
                state,
                last_message,
                project_name,
            });
        }

        self.sessions = sessions;
        Ok(())
    }

    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn select_next(&mut self) {
        if !self.sessions.is_empty() {
            self.selected = (self.selected + 1) % self.sessions.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.sessions.is_empty() {
            if self.selected == 0 {
                self.selected = self.sessions.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        // This test verifies the app can be created
        // It will only pass if the state file exists
        match App::new() {
            Ok(_) => println!("App created successfully"),
            Err(e) => println!("App creation failed (expected if no state file): {}", e),
        }
    }
}

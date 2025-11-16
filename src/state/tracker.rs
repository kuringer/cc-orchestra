use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionInfo {
    pub pid: u32,
    pub tty: String,
    pub cwd: String,
    pub started_at: i64,
    pub zellij_session: Option<String>,
    pub zellij_tab: Option<u32>,
    pub zellij_pane: Option<u32>,
    pub tmux_pane: Option<String>,      // e.g., "%1"
    pub tmux_session: Option<String>,   // e.g., "main"
    pub tmux_window: Option<u32>,       // window index
    #[serde(default)]
    pub last_activity: i64,             // Updated by Stop hook (Unix timestamp)
    #[serde(default)]
    pub user_input_at: i64,             // Updated by UserPromptSubmit hook (Unix timestamp)
    #[serde(default)]
    pub asking_question_at: i64,        // Updated by PostToolUse[AskUserQuestion] hook (Unix timestamp)
    #[serde(default)]
    pub awaiting_permission_at: i64,    // Updated by Notification[permission_prompt] hook (Unix timestamp)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(skip)]
    path: PathBuf,
    pub sessions: HashMap<String, SessionInfo>,
}

impl StateFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            sessions: HashMap::new(),
        }
    }

    pub fn save(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.sessions)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let sessions: HashMap<String, SessionInfo> = serde_json::from_str(&contents)?;
            Ok(Self { path, sessions })
        } else {
            Ok(Self::new(path))
        }
    }

    pub fn add_session(&mut self, session_id: String, info: SessionInfo) {
        self.sessions.insert(session_id, info);
    }

    pub fn remove_session(&mut self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.remove(session_id)
    }
}

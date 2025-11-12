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

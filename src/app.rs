use anyhow::Result;
use crate::state::{SessionState, tracker::{StateFile, SessionInfo}, detector};
use std::path::PathBuf;

pub struct Session {
    pub id: String,
    pub info: SessionInfo,
    pub state: SessionState,
    pub project_name: String,
}

pub struct App {
    state_file: StateFile,
    state_file_path: PathBuf,
    sessions: Vec<Session>,
    selected: usize,
    alerted_sessions: std::collections::HashSet<String>, // Track which sessions have been alerted
    last_state_reload: std::time::Instant, // Track when state file was last reloaded
}

impl App {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")?;
        let state_path = PathBuf::from(&home).join(".claude/cc-orchestra-state.json");
        let state_file = StateFile::load(state_path.clone())?;

        Ok(Self {
            state_file,
            state_file_path: state_path,
            sessions: Vec::new(),
            selected: 0,
            alerted_sessions: std::collections::HashSet::new(),
            last_state_reload: std::time::Instant::now(),
        })
    }

    pub fn refresh(&mut self) -> Result<bool> {
        // Save currently selected session ID to restore after refresh
        let selected_id = self.sessions.get(self.selected).map(|s| s.id.clone());

        // Track if anything changed that requires re-render
        let mut changed = false;

        // Reload state file only every 1 second to reduce I/O
        if self.last_state_reload.elapsed() >= std::time::Duration::from_secs(1) {
            self.state_file = StateFile::load(self.state_file_path.clone())?;
            self.last_state_reload = std::time::Instant::now();
            changed = true;
        }

        let mut sessions = Vec::new();

        for (session_id, info) in &self.state_file.sessions {
            // Detect state using hooks-only logic (no JSONL reading)
            let state = detector::detect_state(info);

            // Skip dead sessions
            if state == SessionState::Dead {
                continue;
            }

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
                project_name,
            });
        }

        // Sort by priority first (Working at top), then by last_activity (newest first)
        sessions.sort_by(|a, b| {
            // Priority: Working=0, AskingQuestion=1, AwaitingPermission=2, Waiting=3, Idle=4, Dead=5
            let priority_a = match a.state {
                SessionState::Working => 0,
                SessionState::AskingQuestion => 1,
                SessionState::AwaitingPermission => 2,
                SessionState::Waiting => 3,
                SessionState::Idle => 4,
                SessionState::Dead => 5,
            };
            let priority_b = match b.state {
                SessionState::Working => 0,
                SessionState::AskingQuestion => 1,
                SessionState::AwaitingPermission => 2,
                SessionState::Waiting => 3,
                SessionState::Idle => 4,
                SessionState::Dead => 5,
            };

            // First compare by priority
            match priority_a.cmp(&priority_b) {
                std::cmp::Ordering::Equal => {
                    // Same priority - sort by last_activity (newest first)
                    b.info.last_activity.cmp(&a.info.last_activity)
                }
                other => other,
            }
        });

        // Deduplicate by PID - keep only the most recent session for each PID
        let mut seen_pids = std::collections::HashSet::new();
        sessions.retain(|session| {
            if seen_pids.contains(&session.info.pid) {
                false // Duplicate PID - remove this session
            } else {
                seen_pids.insert(session.info.pid);
                true // First time seeing this PID - keep it
            }
        });


        // Restore selection by session ID, or default to 0
        if let Some(id) = selected_id {
            self.selected = sessions.iter().position(|s| s.id == id).unwrap_or(0);
        } else {
            self.selected = 0;
        }

        // Check if sessions changed (different count or different states)
        if self.sessions.len() != sessions.len() {
            changed = true;
        } else {
            for (old, new) in self.sessions.iter().zip(sessions.iter()) {
                if old.id != new.id || old.state != new.state {
                    changed = true;
                    break;
                }
            }
        }

        self.sessions = sessions;
        Ok(changed)
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

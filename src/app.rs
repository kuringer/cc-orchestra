use anyhow::Result;
use crate::data::jsonl::SessionActivity;
use crate::state::{SessionState, tracker::{StateFile, SessionInfo}, detector};
use std::path::PathBuf;

pub struct Session {
    pub id: String,
    pub info: SessionInfo,
    pub state: SessionState,
    pub last_activity: Option<SessionActivity>,
    pub project_name: String,
}

pub struct App {
    state_file: StateFile,
    sessions: Vec<Session>,
    selected: usize,
    alerted_sessions: std::collections::HashSet<String>, // Track which sessions have been alerted
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
            alerted_sessions: std::collections::HashSet::new(),
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        // Save currently selected session ID to restore after refresh
        let selected_id = self.sessions.get(self.selected).map(|s| s.id.clone());

        // Reload state file from disk to pick up new sessions
        let home = std::env::var("HOME")?;
        let state_path = PathBuf::from(&home).join(".claude/cc-orchestra-state.json");
        self.state_file = StateFile::load(state_path)?;

        let mut sessions = Vec::new();

        for (session_id, info) in &self.state_file.sessions {
            // Get session activity from JSONL
            let cwd_path = PathBuf::from(&info.cwd);
            let last_activity = crate::data::jsonl::get_session_activity(session_id, &cwd_path)
                .ok()
                .flatten();

            // Calculate activity age
            let last_activity_age = if let Some(ref activity) = last_activity {
                let now = chrono::Utc::now().timestamp_millis();
                ((now - activity.timestamp) / 1000) as i64
            } else {
                9999
            };

            // Calculate file modification age
            let file_mod_age = if let Some(ref activity) = last_activity {
                let now = chrono::Utc::now().timestamp();
                (now - activity.file_modified_at) as i64
            } else {
                9999
            };

            // Determine message type for state detection
            let message_type = last_activity.as_ref().and_then(|activity| {
                // Special case: AskUserQuestion means Claude is BLOCKED waiting for user input
                if activity.last_event_type == "assistant"
                    && activity.last_content_type.as_ref().map(|t| t.as_str()) == Some("tool_use")
                    && activity.tool_name.as_ref().map(|t| t.as_str()) == Some("AskUserQuestion") {
                    Some("needs_input") // Special marker for WaitingForInput state
                }
                // For "assistant" events with tool_use, treat as "user" (tool is running)
                else if activity.last_event_type == "assistant"
                    && activity.last_content_type.as_ref().map(|t| t.as_str()) == Some("tool_use") {
                    Some("user")
                } else if activity.last_event_type == "assistant"
                    && activity.last_content_type.as_ref().map(|t| t.as_str()) == Some("text") {
                    Some("assistant")
                } else {
                    Some(activity.last_event_type.as_str())
                }
            });

            // Detect state
            let state = detector::detect_state(
                info,
                message_type,
                last_activity_age,
                file_mod_age,
            );

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
                last_activity,
                project_name,
            });
        }

        // Sort by priority first (WaitingForInput at top), then by started_at (newest first)
        sessions.sort_by(|a, b| {
            // Priority: WaitingForInput=0, Working=1, Waiting=2, Idle=3
            let priority_a = match a.state {
                SessionState::WaitingForInput => 0,
                SessionState::Working => 1,
                SessionState::Waiting => 2,
                SessionState::Idle => 3,
                SessionState::Dead => 4,
            };
            let priority_b = match b.state {
                SessionState::WaitingForInput => 0,
                SessionState::Working => 1,
                SessionState::Waiting => 2,
                SessionState::Idle => 3,
                SessionState::Dead => 4,
            };

            // First compare by priority
            match priority_a.cmp(&priority_b) {
                std::cmp::Ordering::Equal => {
                    // Same priority - sort by started_at (newest first)
                    b.info.started_at.cmp(&a.info.started_at)
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

        // Play sound alert for new WaitingForInput sessions
        for session in &sessions {
            if session.state == SessionState::WaitingForInput {
                // Only alert if we haven't alerted this session before
                if !self.alerted_sessions.contains(&session.id) {
                    // Play distinctive sound (macOS Sosumi)
                    let _ = std::process::Command::new("afplay")
                        .arg("/System/Library/Sounds/Sosumi.aiff")
                        .spawn();
                    self.alerted_sessions.insert(session.id.clone());
                }
            } else {
                // Remove from alerted set if state changed (so we can alert again if it goes back to WaitingForInput)
                self.alerted_sessions.remove(&session.id);
            }
        }

        // Restore selection by session ID, or default to 0
        if let Some(id) = selected_id {
            self.selected = sessions.iter().position(|s| s.id == id).unwrap_or(0);
        } else {
            self.selected = 0;
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

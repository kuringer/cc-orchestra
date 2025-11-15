pub mod tracker;
pub mod detector;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Working,          // 🟢 Actively generating/running
    WaitingForInput,  // ⏸️ NEEDS INPUT - blocked on user response
    Waiting,          // ⏸️ Waiting for user input (general)
    Idle,             // 💤 No activity 30+ min
    Dead,             // ❌ Process not found
}

#[cfg(test)]
mod tests {
    use super::tracker::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_empty_state_file() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("test-state.json");

        let state = StateFile::new(state_path.clone());
        state.save().unwrap();

        assert!(state_path.exists());
    }

    #[test]
    fn test_track_and_load_session() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("test-state.json");

        let mut state = StateFile::new(state_path.clone());
        state.add_session("abc-123".to_string(), SessionInfo {
            pid: 12345,
            tty: "s000".to_string(),
            cwd: "/tmp/test".to_string(),
            started_at: 1234567890,
            zellij_session: Some("main".to_string()),
            zellij_tab: Some(1),
            zellij_pane: Some(1),
            tmux_pane: None,
            tmux_session: None,
            tmux_window: None,
        });
        state.save().unwrap();

        let loaded = StateFile::load(state_path).unwrap();
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions.get("abc-123").unwrap().pid, 12345);
    }
}

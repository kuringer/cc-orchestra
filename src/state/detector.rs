use super::{SessionState, tracker::SessionInfo};
use crate::data::process;

pub fn detect_state(session_info: &SessionInfo) -> SessionState {
    // 1. Check if process exists
    if !process::process_exists(session_info.pid) {
        return SessionState::Dead;
    }

    // 2. Check for idle timeout (30+ minutes since last activity)
    let now = chrono::Utc::now().timestamp();
    // Fallback: if last_activity is 0 (old sessions), use started_at
    let last_activity = if session_info.last_activity == 0 {
        session_info.started_at
    } else {
        session_info.last_activity
    };
    let inactive_secs = now - last_activity;
    if inactive_secs > 1800 {
        return SessionState::Idle;
    }

    // 3. Default: Active
    SessionState::Active
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tracker::SessionInfo;
    use chrono::Utc;

    fn mock_session() -> SessionInfo {
        SessionInfo {
            pid: 99999,
            tty: "s000".to_string(),
            cwd: "/tmp".to_string(),
            started_at: Utc::now().timestamp(),
            zellij_session: None,
            zellij_tab: None,
            zellij_pane: None,
            tmux_pane: None,
            tmux_session: None,
            tmux_window: None,
            last_activity: Utc::now().timestamp(),
        }
    }

    #[test]
    fn test_dead_process() {
        let session = mock_session();
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Dead);
    }

    #[test]
    fn test_default_is_active() {
        let mut session = mock_session();
        session.pid = std::process::id();
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Active);
    }

    #[test]
    fn test_old_activity_is_idle() {
        let mut session = mock_session();
        session.pid = std::process::id();
        // Set last_activity to 31 minutes ago
        session.last_activity = Utc::now().timestamp() - 1860;
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Idle);
    }
}

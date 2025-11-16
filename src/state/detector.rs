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

    // 3. Check if Claude is working vs waiting
    // If user gave input AFTER Claude's last Stop, Claude is working
    if session_info.user_input_at > session_info.last_activity {
        return SessionState::Working;
    }

    // 4. Default: Waiting (Claude finished, waiting for user)
    SessionState::Waiting
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tracker::SessionInfo;
    use chrono::Utc;

    fn mock_session() -> SessionInfo {
        let now = Utc::now().timestamp();
        SessionInfo {
            pid: 99999,
            tty: "s000".to_string(),
            cwd: "/tmp".to_string(),
            started_at: now,
            zellij_session: None,
            zellij_tab: None,
            zellij_pane: None,
            tmux_pane: None,
            tmux_session: None,
            tmux_window: None,
            last_activity: now,
            user_input_at: 0,  // Default: no user input yet
        }
    }

    #[test]
    fn test_dead_process() {
        let session = mock_session();
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Dead);
    }

    #[test]
    fn test_default_is_waiting() {
        let mut session = mock_session();
        session.pid = std::process::id();
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Waiting);
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

    #[test]
    fn test_working_when_user_input_after_stop() {
        let mut session = mock_session();
        session.pid = std::process::id();
        let now = Utc::now().timestamp();
        session.last_activity = now - 10;  // Claude finished 10 seconds ago
        session.user_input_at = now - 5;   // User gave input 5 seconds ago
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Working);
    }
}

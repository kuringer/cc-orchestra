use super::{SessionState, tracker::SessionInfo};
use crate::data::process;

pub fn detect_state(session_info: &SessionInfo) -> SessionState {
    // 1. Check if process exists
    if !process::process_exists(session_info.pid) {
        return SessionState::Dead;
    }

    // 2. WaitingForInput: set by PostToolUse hook
    if session_info.waiting_for_input {
        return SessionState::WaitingForInput;
    }

    // 3. Check for idle timeout (30+ minutes since session start)
    let now = chrono::Utc::now().timestamp();
    let age_secs = now - session_info.started_at;
    if age_secs > 1800 {
        return SessionState::Idle;
    }

    // 4. Default: Working (we can't distinguish Working vs Waiting without JSONL)
    SessionState::Working
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
            waiting_for_input: false,
            waiting_since: None,
        }
    }

    #[test]
    fn test_dead_process() {
        let session = mock_session();
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Dead);
    }

    #[test]
    fn test_waiting_for_input_flag() {
        let mut session = mock_session();
        session.pid = std::process::id(); // Use current process as alive
        session.waiting_for_input = true;
        let state = detect_state(&session);
        assert_eq!(state, SessionState::WaitingForInput);
    }

    #[test]
    fn test_default_is_working() {
        let mut session = mock_session();
        session.pid = std::process::id();
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Working);
    }

    #[test]
    fn test_old_session_is_idle() {
        let mut session = mock_session();
        session.pid = std::process::id();
        // Set started_at to 31 minutes ago
        session.started_at = Utc::now().timestamp() - 1860;
        let state = detect_state(&session);
        assert_eq!(state, SessionState::Idle);
    }
}

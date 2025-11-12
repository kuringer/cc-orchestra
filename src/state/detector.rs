use super::{SessionState, tracker::SessionInfo};
use crate::data::process;

pub fn detect_state(session_info: &SessionInfo, last_msg_type: Option<&str>, last_msg_age_secs: i64, file_mod_age_secs: i64) -> SessionState {
    // 1. Check if process exists
    if !process::process_exists(session_info.pid) {
        return SessionState::Dead;
    }

    // 2. Check for idle timeout (30+ minutes)
    if last_msg_age_secs > 1800 {
        return SessionState::Idle;
    }

    // 3. If JSONL file was modified recently (<15s), Claude is likely generating
    if file_mod_age_secs < 15 {
        return SessionState::Working;
    }

    // 4. Determine state based on last message
    match last_msg_type {
        Some("user") => {
            // User just sent message or tool result, Claude is processing
            if last_msg_age_secs < 30 {
                SessionState::Working
            } else {
                SessionState::Waiting // Slow response
            }
        }
        Some("assistant") => {
            // Claude responded, waiting for user
            SessionState::Waiting
        }
        _ => SessionState::Waiting,
    }
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
        }
    }

    #[test]
    fn test_dead_process() {
        let session = mock_session();
        let state = detect_state(&session, Some("user"), 10, 9999);
        assert_eq!(state, SessionState::Dead);
    }

    #[test]
    fn test_recent_user_message_is_working() {
        let mut session = mock_session();
        session.pid = std::process::id(); // Use current process as alive
        let state = detect_state(&session, Some("user"), 3, 9999);
        assert_eq!(state, SessionState::Working);
    }

    #[test]
    fn test_old_assistant_message_is_waiting() {
        let mut session = mock_session();
        session.pid = std::process::id();
        let state = detect_state(&session, Some("assistant"), 120, 9999);
        assert_eq!(state, SessionState::Waiting);
    }

    #[test]
    fn test_very_old_activity_is_idle() {
        let mut session = mock_session();
        session.pid = std::process::id();
        let state = detect_state(&session, Some("assistant"), 2000, 9999);
        assert_eq!(state, SessionState::Idle);
    }
}

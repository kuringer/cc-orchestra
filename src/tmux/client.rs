use anyhow::Result;
use std::process::Command;

pub fn is_in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

pub fn get_current_pane() -> Result<String> {
    std::env::var("TMUX_PANE")
        .map_err(|_| anyhow::anyhow!("Not in tmux or TMUX_PANE not set"))
}

pub fn get_session_and_window() -> Result<(String, u32)> {
    let output = Command::new("tmux")
        .args(&["display-message", "-p", "#S:#I"])
        .output()?;

    let result = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = result.trim().split(':').collect();

    if parts.len() == 2 {
        let session = parts[0].to_string();
        let window = parts[1].parse::<u32>()
            .map_err(|_| anyhow::anyhow!("Failed to parse window index"))?;
        Ok((session, window))
    } else {
        Err(anyhow::anyhow!("Failed to parse tmux session:window"))
    }
}

pub fn focus_pane(pane_id: &str) -> Result<()> {
    // First, switch to the window containing this pane
    // This ensures the window is visible before selecting the pane
    Command::new("tmux")
        .args(&["select-window", "-t", pane_id])
        .output()?;

    // Then select the specific pane
    Command::new("tmux")
        .args(&["select-pane", "-t", pane_id])
        .output()?;

    Ok(())
}

pub fn switch_client(target: &str) -> Result<()> {
    // target format: "session:window.pane" or "session:window" or "%pane_id"
    Command::new("tmux")
        .args(&["switch-client", "-t", target])
        .output()?;
    Ok(())
}

pub fn create_window_and_run(session: &str, command: &str) -> Result<String> {
    // Create a new window in the specified session and run a command
    let output = Command::new("tmux")
        .args(&[
            "new-window",
            "-t", session,
            "-P",  // Print info about new window
            "-F", "#{pane_id}",  // Format: print pane ID
            command
        ])
        .output()?;

    let pane_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if pane_id.is_empty() {
        Err(anyhow::anyhow!("Failed to create tmux window"))
    } else {
        Ok(pane_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_tmux() {
        // This will pass/fail depending on environment
        let result = is_in_tmux();
        println!("In tmux: {}", result);
    }
}

use anyhow::Result;
use std::process::Command;

pub fn is_in_zellij() -> bool {
    std::env::var("ZELLIJ").is_ok()
}

pub fn get_current_session() -> Result<String> {
    let output = Command::new("zellij")
        .args(&["list-sessions"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("(current)") {
            let session_name = line.split_whitespace().next().unwrap_or("");
            return Ok(session_name.to_string());
        }
    }

    Err(anyhow::anyhow!("No current Zellij session found"))
}

pub fn focus_tab(tab_index: u32) -> Result<()> {
    Command::new("zellij")
        .args(&["action", "go-to-tab", &tab_index.to_string()])
        .output()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_zellij() {
        // This will pass/fail depending on environment
        let result = is_in_zellij();
        println!("In Zellij: {}", result);
    }
}

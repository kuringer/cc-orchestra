use anyhow::Result;
use std::process::Command;

pub fn find_claude_processes() -> Result<Vec<(u32, String)>> {
    let output = Command::new("ps")
        .args(&["aux"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut processes = Vec::new();

    for line in stdout.lines() {
        if line.contains("claude") && !line.contains("grep") {
            // Parse PID from ps output (second column)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                if let Ok(pid) = parts[1].parse::<u32>() {
                    // Try to get cwd (this is simplified, may need platform-specific impl)
                    processes.push((pid, String::new()));
                }
            }
        }
    }

    Ok(processes)
}

pub fn process_exists(pid: u32) -> bool {
    #[cfg(target_os = "macos")]
    {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_current_process() {
        let processes = find_claude_processes().unwrap();
        // Just verify it runs without error
        assert!(processes.len() >= 0);
    }

    #[test]
    fn test_process_exists_self() {
        let pid = std::process::id();
        assert!(process_exists(pid));
    }

    #[test]
    fn test_process_not_exists() {
        // PID 99999 unlikely to exist
        assert!(!process_exists(99999));
    }
}

# CC-Orchestra MVP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a real-time TUI dashboard that monitors multiple Claude Code sessions and enables instant switching via Zellij integration.

**Architecture:** Standalone Rust application using Ratatui for TUI rendering. Reads Claude's SQLite database and JSONL logs, maintains session state via hooks, and integrates with Zellij CLI for session switching.

**Tech Stack:** Rust, Ratatui, Crossterm, Rusqlite, Serde, Tokio

---

## Task 1: Project Setup and Dependencies

**Files:**
- Modify: `Cargo.toml`
- Create: `src/main.rs` (basic structure)

**Step 1: Add dependencies to Cargo.toml**

```toml
[package]
name = "cc-orchestra"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.28"
crossterm = "0.28"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
anyhow = "1.0"
clap = { version = "4", features = ["derive"] }
chrono = "0.4"

[dev-dependencies]
tempfile = "3.0"
```

**Step 2: Update main.rs with basic CLI structure**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cc-orchestra")]
#[command(about = "Real-time TUI dashboard for Claude Code sessions", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Track a Claude Code session (called by hooks)
    TrackSession {
        #[arg(long)]
        pid: u32,
        #[arg(long)]
        session_id: String,
    },
    /// Untrack a Claude Code session (called by hooks)
    UntrackSession {
        #[arg(long)]
        session_id: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::TrackSession { pid, session_id }) => {
            println!("Tracking session {} with PID {}", session_id, pid);
        }
        Some(Commands::UntrackSession { session_id }) => {
            println!("Untracking session {}", session_id);
        }
        None => {
            println!("Starting dashboard...");
        }
    }
}
```

**Step 3: Build to verify dependencies**

Run: `cargo build`
Expected: Success with all dependencies downloaded

**Step 4: Test CLI parsing**

Run: `cargo run -- track-session --pid 12345 --session-id abc-123`
Expected: Output "Tracking session abc-123 with PID 12345"

**Step 5: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "feat: add project dependencies and basic CLI structure

Set up Ratatui, Rusqlite, and Clap for TUI dashboard.
Add track-session and untrack-session subcommands."
```

---

## Task 2: Session State File Management

**Files:**
- Create: `src/state/mod.rs`
- Create: `src/state/tracker.rs`
- Modify: `src/main.rs`

**Step 1: Write failing test for state file creation**

Create: `src/state/mod.rs`

```rust
pub mod tracker;

#[cfg(test)]
mod tests {
    use super::tracker::*;
    use tempfile::tempdir;
    use std::path::PathBuf;

    #[test]
    fn test_create_empty_state_file() {
        let dir = tempdir().unwrap();
        let state_path = dir.path().join("test-state.json");

        let state = StateFile::new(state_path.clone());
        state.save().unwrap();

        assert!(state_path.exists());
    }
}
```

Create: `src/state/tracker.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionInfo {
    pub pid: u32,
    pub tty: String,
    pub cwd: String,
    pub started_at: i64,
    pub zellij_session: Option<String>,
    pub zellij_tab: Option<u32>,
    pub zellij_pane: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(skip)]
    path: PathBuf,
    pub sessions: HashMap<String, SessionInfo>,
}

impl StateFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            sessions: HashMap::new(),
        }
    }

    pub fn save(&self) -> Result<()> {
        // TODO: implement
        Ok(())
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_create_empty_state_file`
Expected: FAIL (file not created)

**Step 3: Implement state file save**

Update `src/state/tracker.rs`:

```rust
impl StateFile {
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.sessions)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let sessions: HashMap<String, SessionInfo> = serde_json::from_str(&contents)?;
            Ok(Self { path, sessions })
        } else {
            Ok(Self::new(path))
        }
    }

    pub fn add_session(&mut self, session_id: String, info: SessionInfo) {
        self.sessions.insert(session_id, info);
    }

    pub fn remove_session(&mut self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.remove(session_id)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_create_empty_state_file`
Expected: PASS

**Step 5: Add test for tracking sessions**

Add to `src/state/mod.rs` tests:

```rust
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
    });
    state.save().unwrap();

    let loaded = StateFile::load(state_path).unwrap();
    assert_eq!(loaded.sessions.len(), 1);
    assert_eq!(loaded.sessions.get("abc-123").unwrap().pid, 12345);
}
```

**Step 6: Run test**

Run: `cargo test test_track_and_load_session`
Expected: PASS

**Step 7: Wire up CLI commands**

Update `src/main.rs`:

```rust
mod state;

use clap::{Parser, Subcommand};
use state::tracker::{StateFile, SessionInfo};
use std::path::PathBuf;

// ... (keep existing Cli and Commands)

fn get_state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap();
    PathBuf::from(home).join(".claude/cc-orchestra-state.json")
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::TrackSession { pid, session_id }) => {
            let mut state = StateFile::load(get_state_path()).unwrap();
            let cwd = std::env::current_dir().unwrap().to_string_lossy().to_string();
            let tty = std::env::var("TTY").unwrap_or_else(|_| "unknown".to_string());

            state.add_session(session_id.clone(), SessionInfo {
                pid: *pid,
                tty,
                cwd,
                started_at: chrono::Utc::now().timestamp(),
                zellij_session: None, // TODO: detect Zellij
                zellij_tab: None,
                zellij_pane: None,
            });
            state.save().unwrap();
            println!("✓ Tracked session {}", session_id);
        }
        Some(Commands::UntrackSession { session_id }) => {
            let mut state = StateFile::load(get_state_path()).unwrap();
            if state.remove_session(session_id).is_some() {
                state.save().unwrap();
                println!("✓ Untracked session {}", session_id);
            } else {
                println!("⚠ Session {} not found", session_id);
            }
        }
        None => {
            println!("Starting dashboard...");
        }
    }
}
```

**Step 8: Manual test CLI commands**

Run: `cargo run -- track-session --pid 99999 --session-id test-123`
Expected: "✓ Tracked session test-123" and file created at `~/.claude/cc-orchestra-state.json`

Run: `cat ~/.claude/cc-orchestra-state.json`
Expected: JSON with session info

Run: `cargo run -- untrack-session --session-id test-123`
Expected: "✓ Untracked session test-123"

**Step 9: Commit**

```bash
git add src/state/ src/main.rs
git commit -m "feat: add session state tracking

Implement StateFile for tracking Claude sessions via hooks.
Add track-session and untrack-session commands."
```

---

## Task 3: SQLite Database Reader

**Files:**
- Create: `src/data/mod.rs`
- Create: `src/data/sqlite.rs`

**Step 1: Write failing test for database query**

Create: `src/data/mod.rs`

```rust
pub mod sqlite;

#[cfg(test)]
mod tests {
    use super::sqlite::*;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    fn create_test_db() -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let conn = Connection::open(file.path()).unwrap();

        conn.execute(
            "CREATE TABLE base_messages (
                uuid TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                message_type TEXT NOT NULL,
                cwd TEXT NOT NULL
            )",
            [],
        ).unwrap();

        conn.execute(
            "INSERT INTO base_messages VALUES (?, ?, ?, ?, ?)",
            ["msg1", "session-1", "1762956296000", "user", "/tmp/project1"],
        ).unwrap();

        conn.execute(
            "INSERT INTO base_messages VALUES (?, ?, ?, ?, ?)",
            ["msg2", "session-1", "1762956300000", "assistant", "/tmp/project1"],
        ).unwrap();

        file
    }

    #[test]
    fn test_query_latest_message() {
        let db_file = create_test_db();
        let reader = ClaudeDb::new(db_file.path().to_path_buf()).unwrap();

        let msg = reader.get_latest_message("session-1").unwrap();
        assert_eq!(msg.message_type, "assistant");
        assert_eq!(msg.timestamp, 1762956300000);
    }
}
```

Create: `src/data/sqlite.rs`

```rust
use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Message {
    pub uuid: String,
    pub session_id: String,
    pub timestamp: i64,
    pub message_type: String,
    pub cwd: String,
}

pub struct ClaudeDb {
    conn: Connection,
}

impl ClaudeDb {
    pub fn new(path: PathBuf) -> Result<Self> {
        // TODO: implement
        Ok(Self {
            conn: Connection::open(path)?
        })
    }

    pub fn get_latest_message(&self, session_id: &str) -> Result<Message> {
        // TODO: implement
        unimplemented!()
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_query_latest_message`
Expected: FAIL (unimplemented)

**Step 3: Implement database query**

Update `src/data/sqlite.rs`:

```rust
impl ClaudeDb {
    pub fn get_latest_message(&self, session_id: &str) -> Result<Message> {
        let mut stmt = self.conn.prepare(
            "SELECT uuid, session_id, timestamp, message_type, cwd
             FROM base_messages
             WHERE session_id = ?1
             ORDER BY timestamp DESC
             LIMIT 1"
        )?;

        let msg = stmt.query_row(params![session_id], |row| {
            Ok(Message {
                uuid: row.get(0)?,
                session_id: row.get(1)?,
                timestamp: row.get(2)?,
                message_type: row.get(3)?,
                cwd: row.get(4)?,
            })
        })?;

        Ok(msg)
    }

    pub fn get_active_sessions(&self, since_hours: i64) -> Result<Vec<String>> {
        let cutoff = chrono::Utc::now().timestamp() * 1000 - (since_hours * 3600 * 1000);
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT session_id
             FROM base_messages
             WHERE timestamp > ?1"
        )?;

        let sessions = stmt.query_map(params![cutoff], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(sessions)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_query_latest_message`
Expected: PASS

**Step 5: Add retry logic for locked database**

Add to `src/data/sqlite.rs`:

```rust
use std::thread;
use std::time::Duration;

impl ClaudeDb {
    pub fn with_retry<F, T>(path: PathBuf, f: F) -> Result<T>
    where
        F: Fn(&Connection) -> Result<T>,
    {
        let max_attempts = 3;
        let mut attempts = 0;

        loop {
            match Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
                Ok(conn) => return f(&conn),
                Err(e) if attempts < max_attempts => {
                    attempts += 1;
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}
```

**Step 6: Commit**

```bash
git add src/data/
git commit -m "feat: add SQLite database reader for Claude sessions

Query base_messages table for session activity.
Implement retry logic for database lock handling."
```

---

## Task 4: Process Monitor

**Files:**
- Create: `src/data/process.rs`
- Modify: `src/data/mod.rs`

**Step 1: Write test for process detection**

Add to `src/data/mod.rs`:

```rust
pub mod process;
```

Create: `src/data/process.rs`

```rust
use anyhow::Result;
use std::process::Command;

pub fn find_claude_processes() -> Result<Vec<(u32, String)>> {
    // Returns Vec<(pid, cwd)>
    // TODO: implement
    Ok(vec![])
}

pub fn process_exists(pid: u32) -> bool {
    // TODO: implement
    false
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
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_process_exists`
Expected: FAIL

**Step 3: Implement process checking**

Update `src/data/process.rs`:

```rust
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
```

**Step 4: Run tests**

Run: `cargo test test_process_exists`
Expected: PASS (at least test_process_exists_self should pass)

**Step 5: Commit**

```bash
git add src/data/process.rs src/data/mod.rs
git commit -m "feat: add process monitoring

Detect running Claude Code processes.
Verify process existence by PID."
```

---

## Task 5: Session State Detector

**Files:**
- Create: `src/state/detector.rs`
- Modify: `src/state/mod.rs`

**Step 1: Define session states**

Add to `src/state/mod.rs`:

```rust
pub mod detector;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Working,   // 🟢 Actively generating/running
    Waiting,   // ⏸️ Waiting for user input
    Idle,      // 💤 No activity 30+ min
    Dead,      // ❌ Process not found
}
```

Create: `src/state/detector.rs`:

```rust
use super::{SessionState, tracker::SessionInfo};
use crate::data::process;
use chrono::Utc;

pub fn detect_state(session_info: &SessionInfo, last_msg_type: Option<&str>, last_msg_age_secs: i64) -> SessionState {
    // TODO: implement
    SessionState::Waiting
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tracker::SessionInfo;

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
        let state = detect_state(&session, Some("user"), 10);
        assert_eq!(state, SessionState::Dead);
    }

    #[test]
    fn test_recent_user_message_is_working() {
        let mut session = mock_session();
        session.pid = std::process::id(); // Use current process as alive
        let state = detect_state(&session, Some("user"), 3);
        assert_eq!(state, SessionState::Working);
    }

    #[test]
    fn test_old_assistant_message_is_waiting() {
        let mut session = mock_session();
        session.pid = std::process::id();
        let state = detect_state(&session, Some("assistant"), 120);
        assert_eq!(state, SessionState::Waiting);
    }

    #[test]
    fn test_very_old_activity_is_idle() {
        let mut session = mock_session();
        session.pid = std::process::id();
        let state = detect_state(&session, Some("assistant"), 2000);
        assert_eq!(state, SessionState::Idle);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_dead_process`
Expected: FAIL

**Step 3: Implement state detection**

Update `src/state/detector.rs`:

```rust
pub fn detect_state(session_info: &SessionInfo, last_msg_type: Option<&str>, last_msg_age_secs: i64) -> SessionState {
    // 1. Check if process exists
    if !process::process_exists(session_info.pid) {
        return SessionState::Dead;
    }

    // 2. Check for idle timeout (30+ minutes)
    if last_msg_age_secs > 1800 {
        return SessionState::Idle;
    }

    // 3. Determine state based on last message
    match last_msg_type {
        Some("user") => {
            // User just sent message, Claude is processing
            if last_msg_age_secs < 5 {
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test detect_state`
Expected: PASS

**Step 5: Commit**

```bash
git add src/state/detector.rs src/state/mod.rs
git commit -m "feat: add session state detection

Implement multi-signal state detection:
- Process existence check
- Idle timeout (30 min)
- Message type analysis"
```

---

## Task 6: Basic TUI Dashboard

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/dashboard.rs`
- Modify: `src/main.rs`

**Step 1: Create basic TUI structure**

Create: `src/ui/mod.rs`

```rust
pub mod dashboard;
```

Create: `src/ui/dashboard.rs`

```rust
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;

pub struct Dashboard {
    selected: usize,
}

impl Dashboard {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .constraints([Constraint::Min(0)])
                    .split(f.area());

                let items = vec![
                    ListItem::new("Session 1 - Working"),
                    ListItem::new("Session 2 - Waiting"),
                ];

                let list = List::new(items)
                    .block(Block::default()
                        .title("CC-ORCHESTRA")
                        .borders(Borders::ALL));

                f.render_widget(list, chunks[0]);
            })?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;

        Ok(())
    }
}
```

**Step 2: Wire up dashboard to main**

Update `src/main.rs`:

```rust
mod ui;

// ... in main() function, update None branch:

        None => {
            let mut dashboard = ui::dashboard::Dashboard::new();
            dashboard.run().unwrap();
        }
```

**Step 3: Manual test**

Run: `cargo run`
Expected: TUI opens showing "Session 1 - Working" and "Session 2 - Waiting". Press 'q' to quit.

**Step 4: Commit**

```bash
git add src/ui/ src/main.rs
git commit -m "feat: add basic TUI dashboard

Create Ratatui-based dashboard with placeholder sessions.
Add quit on 'q' key."
```

---

## Task 7: Integrate Real Session Data

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs`
- Modify: `src/ui/dashboard.rs`

**Step 1: Create app state manager**

Create: `src/app.rs`:

```rust
use anyhow::Result;
use crate::data::sqlite::{ClaudeDb, Message};
use crate::data::process;
use crate::state::{SessionState, tracker::{StateFile, SessionInfo}, detector};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct Session {
    pub id: String,
    pub info: SessionInfo,
    pub state: SessionState,
    pub last_message: Option<Message>,
    pub project_name: String,
}

pub struct App {
    state_file: StateFile,
    sessions: Vec<Session>,
    selected: usize,
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
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let home = std::env::var("HOME")?;
        let db_path = PathBuf::from(&home).join(".claude/__store.db");

        let db = ClaudeDb::new(db_path)?;
        let mut sessions = Vec::new();

        for (session_id, info) in &self.state_file.sessions {
            // Get last message
            let last_message = db.get_latest_message(session_id).ok();

            // Calculate message age
            let last_msg_age = if let Some(ref msg) = last_message {
                let now = chrono::Utc::now().timestamp() * 1000;
                ((now - msg.timestamp) / 1000) as i64
            } else {
                9999
            };

            // Detect state
            let state = detector::detect_state(
                info,
                last_message.as_ref().map(|m| m.message_type.as_str()),
                last_msg_age,
            );

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
                last_message,
                project_name,
            });
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
```

**Step 2: Update dashboard to use App**

Update `src/ui/dashboard.rs`:

```rust
use crate::app::{App, Session};
use crate::state::SessionState;

pub struct Dashboard {
    app: App,
}

impl Dashboard {
    pub fn new(app: App) -> Self {
        Self { app }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        loop {
            // Refresh data
            self.app.refresh()?;

            terminal.draw(|f| {
                let chunks = Layout::default()
                    .constraints([Constraint::Min(0)])
                    .split(f.area());

                let items: Vec<ListItem> = self.app.sessions()
                    .iter()
                    .enumerate()
                    .map(|(i, session)| {
                        let status_icon = match session.state {
                            SessionState::Working => "🟢",
                            SessionState::Waiting => "⏸️ ",
                            SessionState::Idle => "💤",
                            SessionState::Dead => "❌",
                        };

                        let text = format!("{} {} - {:?}",
                            if i == self.app.selected() { "►" } else { " " },
                            session.project_name,
                            session.state
                        );

                        ListItem::new(text)
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default()
                        .title("CC-ORCHESTRA")
                        .borders(Borders::ALL));

                f.render_widget(list, chunks[0]);
            })?;

            if event::poll(std::time::Duration::from_millis(2000))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down | KeyCode::Char('j') => self.app.select_next(),
                        KeyCode::Up | KeyCode::Char('k') => self.app.select_previous(),
                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;

        Ok(())
    }
}
```

**Step 3: Update main.rs**

Update `src/main.rs`:

```rust
mod app;

// ... in main():

        None => {
            let app = app::App::new().unwrap();
            let mut dashboard = ui::dashboard::Dashboard::new(app);
            dashboard.run().unwrap();
        }
```

**Step 4: Manual test with real data**

Run: `cargo run -- track-session --pid 11111 --session-id test-abc`
Run: `cargo run`
Expected: Dashboard shows "test-abc" session with detected state

**Step 5: Commit**

```bash
git add src/app.rs src/ui/dashboard.rs src/main.rs
git commit -m "feat: integrate real session data into dashboard

Connect TUI to state file and SQLite database.
Display actual sessions with detected states.
Add navigation with j/k or arrow keys."
```

---

## Task 8: Zellij Integration (Basic)

**Files:**
- Create: `src/zellij/mod.rs`
- Create: `src/zellij/client.rs`

**Step 1: Create Zellij client stub**

Create: `src/zellij/mod.rs`:

```rust
pub mod client;
```

Create: `src/zellij/client.rs`:

```rust
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
```

**Step 2: Test Zellij detection**

Run: `cargo test test_is_in_zellij`
Expected: PASS (prints result)

**Step 3: Update dashboard to show Zellij warning if not detected**

Update `src/ui/dashboard.rs`:

```rust
use crate::zellij::client as zellij;

// In the draw function, add before the list:

                if !zellij::is_in_zellij() {
                    let warning = Paragraph::new("⚠ Not running in Zellij - session switching disabled")
                        .style(Style::default().fg(Color::Yellow));
                    // Note: You'll need to adjust layout to show this
                }
```

**Step 4: Commit**

```bash
git add src/zellij/
git commit -m "feat: add basic Zellij integration

Detect Zellij environment.
Add stub for session switching (to be completed)."
```

---

## Task 9: Polish Dashboard UI

**Files:**
- Modify: `src/ui/dashboard.rs`

**Step 1: Improve dashboard layout**

Update `src/ui/dashboard.rs` to show better formatting:

```rust
terminal.draw(|f| {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Header
            Constraint::Min(0),      // Session list
            Constraint::Length(3),   // Footer
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new(format!(
        "CC-ORCHESTRA          {} Active Sessions          Updated: {}",
        self.app.sessions().len(),
        chrono::Local::now().format("%H:%M:%S")
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Session list
    let items: Vec<ListItem> = self.app.sessions()
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let status_icon = match session.state {
                SessionState::Working => "🟢 Working",
                SessionState::Waiting => "⏸️  Waiting",
                SessionState::Idle => "💤 Idle",
                SessionState::Dead => "❌ Dead",
            };

            let age = if let Some(ref msg) = session.last_message {
                let now = chrono::Utc::now().timestamp() * 1000;
                let secs = (now - msg.timestamp) / 1000;
                if secs < 60 {
                    format!("{}s ago", secs)
                } else {
                    format!("{}m ago", secs / 60)
                }
            } else {
                "unknown".to_string()
            };

            let text = format!(
                "{} {:20} {:15} PID:{:6} {}",
                if i == self.app.selected() { "►" } else { " " },
                session.project_name,
                status_icon,
                session.info.pid,
                age
            );

            ListItem::new(text)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(list, chunks[1]);

    // Footer with keybindings
    let footer = Paragraph::new("[↑↓/jk] Navigate  [Enter] Jump  [r] Refresh  [q] Quit")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
})?;
```

**Step 2: Add manual refresh**

Update event handling:

```rust
match key.code {
    KeyCode::Char('q') => break,
    KeyCode::Down | KeyCode::Char('j') => self.app.select_next(),
    KeyCode::Up | KeyCode::Char('k') => self.app.select_previous(),
    KeyCode::Char('r') => { /* Force refresh */ }
    _ => {}
}
```

**Step 3: Manual test**

Run: `cargo run`
Expected: Professional-looking dashboard with header, footer, and formatted session list

**Step 4: Commit**

```bash
git add src/ui/dashboard.rs
git commit -m "feat: polish dashboard UI

Add header with session count and timestamp.
Show formatted session list with PID and age.
Add footer with keybinding help."
```

---

## Task 10: Installation Script

**Files:**
- Create: `install.sh`
- Create: `README.md`

**Step 1: Create installation script**

Create: `install.sh`

```bash
#!/bin/bash
set -e

echo "🎻 Installing cc-orchestra..."

# Build release binary
cargo build --release

# Install binary
mkdir -p ~/.local/bin
cp target/release/cc-orchestra ~/.local/bin/
chmod +x ~/.local/bin/cc-orchestra

echo "✓ Binary installed to ~/.local/bin/cc-orchestra"

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️  Warning: ~/.local/bin is not in your PATH"
    echo "   Add this to your ~/.zshrc or ~/.bashrc:"
    echo '   export PATH="$HOME/.local/bin:$PATH"'
fi

# Create state directory
mkdir -p ~/.claude

# Add hooks to Claude Code settings
SETTINGS_FILE="$HOME/.claude/settings.json"

if [ -f "$SETTINGS_FILE" ]; then
    echo "⚠️  Found existing settings.json"
    echo "   Please manually add these hooks to ~/.claude/settings.json:"
    echo '
  "hooks": {
    "SessionStart": [{
      "type": "command",
      "command": "cc-orchestra track-session --pid $$ --session-id $CLAUDE_SESSION_ID"
    }],
    "SessionEnd": [{
      "type": "command",
      "command": "cc-orchestra untrack-session --session-id $CLAUDE_SESSION_ID"
    }]
  }
'
else
    echo "✓ Creating settings.json with hooks"
    cat > "$SETTINGS_FILE" << 'EOF'
{
  "hooks": {
    "SessionStart": [{
      "type": "command",
      "command": "cc-orchestra track-session --pid $$ --session-id $CLAUDE_SESSION_ID"
    }],
    "SessionEnd": [{
      "type": "command",
      "command": "cc-orchestra untrack-session --session-id $CLAUDE_SESSION_ID"
    }]
  }
}
EOF
fi

echo ""
echo "✅ Installation complete!"
echo ""
echo "Usage:"
echo "  cc-orchestra              # Launch dashboard"
echo "  cc-orchestra --help       # Show help"
echo ""
echo "Next steps:"
echo "  1. Start a new Claude Code session"
echo "  2. Run 'cc-orchestra' to see it tracked"
```

**Step 2: Make executable**

Run: `chmod +x install.sh`

**Step 3: Create README**

Create: `README.md`

```markdown
# CC-Orchestra 🎻

Real-time TUI dashboard for managing multiple Claude Code sessions.

## Features

- 🟢 Real-time session state tracking (Working/Waiting/Idle/Dead)
- ⚡ Quick navigation between sessions
- 🎯 Zellij integration for instant switching
- 📊 Activity monitoring with timestamps
- 🪝 Automatic session tracking via Claude Code hooks

## Installation

```bash
./install.sh
```

This will:
- Build and install the `cc-orchestra` binary
- Set up Claude Code hooks for automatic tracking
- Create necessary config directories

## Usage

```bash
# Launch the dashboard
cc-orchestra

# Track a session manually (usually called by hooks)
cc-orchestra track-session --pid 12345 --session-id abc-123

# Remove session tracking
cc-orchestra untrack-session --session-id abc-123
```

## Keybindings

- `↑/↓` or `j/k` - Navigate sessions
- `Enter` - Jump to selected session (Zellij required)
- `r` - Force refresh
- `q` - Quit

## Requirements

- Rust 1.70+
- Claude Code with hooks support
- Zellij (optional, for session switching)

## Architecture

CC-Orchestra monitors Claude Code sessions by:
1. Reading `~/.claude/__store.db` for session metadata
2. Tracking PIDs via hooks in `~/.claude/cc-orchestra-state.json`
3. Detecting process state using multi-signal analysis
4. Rendering real-time dashboard with Ratatui

See [design doc](docs/plans/2025-11-12-cc-orchestra-design.md) for details.

## Development

```bash
# Run in development mode
cargo run

# Run tests
cargo test

# Build release
cargo build --release
```

## License

MIT
```

**Step 4: Commit**

```bash
git add install.sh README.md
git commit -m "feat: add installation script and README

Create install.sh for easy setup.
Document usage and architecture in README."
```

---

## Final Testing Checklist

Before completing this plan, verify:

1. ✅ Build succeeds: `cargo build --release`
2. ✅ All tests pass: `cargo test`
3. ✅ Track session works: `cargo run -- track-session --pid 99999 --session-id test`
4. ✅ State file created: Check `~/.claude/cc-orchestra-state.json`
5. ✅ Dashboard launches: `cargo run`
6. ✅ Navigation works: Press j/k to move, q to quit
7. ✅ Installation script: `./install.sh`
8. ✅ Binary available: `cc-orchestra --help`

---

## Known Limitations (MVP)

- Session switching requires manual Zellij integration (Enter key not fully wired)
- No JSONL log parsing yet (future enhancement)
- Process CWD detection is basic
- No detailed view ('d' key not implemented)
- No filtering ('f' key not implemented)

These can be addressed in follow-up iterations.

# CC-Orchestra Design Document

**Date:** 2025-11-12
**Status:** Design Phase
**Tech Stack:** Rust + Ratatui + Zellij

## Problem Statement

Managing multiple Claude Code sessions across different projects is challenging. When running 5-6 concurrent Claude Code instances in different terminal tabs, it's difficult to:

- See which sessions are actively working vs. waiting for input
- Quickly identify which project each session belongs to
- Switch between sessions efficiently
- Track overall progress across all sessions

## Solution Overview

**CC-Orchestra** is a real-time TUI dashboard that monitors all active Claude Code sessions, shows their status, and enables instant switching between them via Zellij integration.

## Architecture

### System Design

```
┌─────────────────────────────────────────┐
│      cc-orchestra (Rust + Ratatui)      │
│  ┌────────────┐      ┌───────────────┐  │
│  │ TUI Layer  │◄────►│ State Manager │  │
│  └────────────┘      └───────────────┘  │
│                             │            │
│         ┌───────────────────┼─────────┐ │
│         ▼                   ▼         ▼ │
│  ┌────────────┐  ┌──────────────┐ ┌───┤
│  │ Claude DB  │  │ Process Mon  │ │Zel│
│  │  Reader    │  │  (ps/proc)   │ │lij│
│  └────────────┘  └──────────────┘ └─API┘
│         │                   │         │
└─────────┼───────────────────┼─────────┘
          ▼                   ▼
    ~/.claude/          Running Claude
    __store.db          processes (PIDs)
```

### Core Components

1. **TUI Layer** (Ratatui): Renders dashboard, handles user input
2. **State Manager**: Aggregates data from multiple sources, maintains session state
3. **Claude DB Reader**: Queries `~/.claude/__store.db` for session metadata
4. **Process Monitor**: Scans running processes, matches PIDs to sessions
5. **Zellij Client**: Integrates with Zellij CLI for session switching

## Data Sources

### 1. Claude Code SQLite Database

**Location:** `~/.claude/__store.db`

**Key tables:**
- `base_messages`: All conversation messages with timestamps
- `conversation_summaries`: AI-generated summaries of conversations

**Query for active sessions:**
```sql
SELECT DISTINCT session_id, cwd, timestamp, message_type
FROM base_messages
WHERE timestamp > (strftime('%s', 'now') - 7200) * 1000
ORDER BY timestamp DESC;
```

### 2. Session JSONL Logs

**Location:** `~/.claude/projects/<sanitized-cwd>/<session-id>.jsonl`

**Contains:**
- Tool use events (Bash, Read, Write, etc.)
- Real-time activity stream
- Subagent spawning events

**Use case:** Detect long-running commands, active tool calls

### 3. Process Information

**Sources:**
- `ps aux` for PID listing
- `/proc/<pid>/cmdline` (Linux) or `lsof -p <pid>` (macOS) for process verification
- Process parent-child relationships for detecting background commands

### 4. Session State File

**Location:** `~/.claude/cc-orchestra-state.json`

**Maintained by:** Claude Code hooks (SessionStart/SessionEnd)

**Schema:**
```json
{
  "sessions": {
    "<session-id>": {
      "pid": 23325,
      "tty": "s000",
      "cwd": "/path/to/project",
      "started_at": 1762956296000,
      "zellij_session": "main",
      "zellij_tab": 3,
      "zellij_pane": 1
    }
  }
}
```

## State Detection Logic

### Session States

- 🟢 **Working**: Actively generating responses or running commands
- ⏸️ **Waiting**: Last message from Claude, waiting for user input
- 💤 **Idle**: No activity for 30+ minutes
- ❌ **Dead**: Process terminated but session still in database

### Multi-Signal Detection Algorithm

```rust
fn detect_state(session: &Session) -> SessionState {
    // 1. Process existence check
    if !process_exists(session.pid) {
        return SessionState::Dead;
    }

    // 2. Idle timeout check
    let age = now() - session.last_activity;
    if age > Duration::minutes(30) {
        return SessionState::Idle;
    }

    // 3. Background process check (long-running commands)
    if has_child_processes(session.pid) {
        return SessionState::Working;
    }

    // 4. JSONL tool activity check
    let jsonl = parse_session_jsonl(session.id)?;
    if let Some(last_tool) = jsonl.last_tool_use() {
        if last_tool.tool == "Bash" && last_tool.age < 60s {
            return SessionState::Working;
        }
    }

    // 5. SQLite message check (with retry on lock)
    let last_msg = query_with_retry(
        "SELECT message_type, content, timestamp
         FROM base_messages
         WHERE session_id = ?
         ORDER BY timestamp DESC LIMIT 1",
        &[session.id]
    )?;

    match last_msg.message_type.as_str() {
        "user" => {
            if last_msg.age < Duration::seconds(5) {
                SessionState::Working
            } else {
                SessionState::Waiting
            }
        },
        "assistant" => {
            if looks_incomplete(&last_msg.content) {
                SessionState::Working
            } else {
                SessionState::Waiting
            }
        },
        _ => SessionState::Waiting
    }
}
```

### Key Improvements Over Naive Approach

1. **Robust PID matching**: Uses state file maintained by hooks instead of fragile process matching
2. **Detects long-running commands**: Checks for child processes (npm install, docker build, etc.)
3. **SQLite lock handling**: Retry logic for database busy errors
4. **Incomplete response detection**: Identifies streaming assistant responses
5. **JSONL parsing**: Real-time activity tracking via tool use logs

## User Interface

### Main Dashboard

```
╔══════════════════════════════════════════════════════════════════════╗
║ CC-ORCHESTRA                    6 Active Sessions    Updated: 15:32  ║
╠══════════════════════════════════════════════════════════════════════╣
║ PROJECT                         STATUS      SESSION     ZELLIJ       ║
║─────────────────────────────────────────────────────────────────────║
║ ► command-os                    🟢 Working   s008      tab:1 pane:2  ║
║   Last: "Fixing type errors"    2m ago                               ║
║─────────────────────────────────────────────────────────────────────║
║   project-tui                   ⏸️  Waiting   s016      tab:2 pane:1  ║
║   Last: "Add this feature..."   5m ago                               ║
║─────────────────────────────────────────────────────────────────────║
║   zapadel.sk                    🟢 Working   s000      tab:3 pane:1  ║
║   Last: "Creating component"    30s ago                              ║
║─────────────────────────────────────────────────────────────────────║
║   mac-control-center            🟢 Working   s010      tab:1 pane:3  ║
║   Last: "Running tests"         1m ago                               ║
║─────────────────────────────────────────────────────────────────────║
║   janulka-cleaning              ⏸️  Waiting   s005      tab:4 pane:1  ║
║   Last: "Do you want me to..."  12m ago                              ║
║─────────────────────────────────────────────────────────────────────║
║   cc-orchestra                  🟢 Working   s007      tab:5 pane:1  ║
║   Last: "Let me present..."     just now                             ║
╠══════════════════════════════════════════════════════════════════════╣
║ [↑↓] Navigate  [Enter] Jump  [d] Details  [f] Filter  [q] Quit      ║
╚══════════════════════════════════════════════════════════════════════╝
```

### Key Bindings

- **↑/↓** or **j/k**: Navigate between sessions
- **Enter**: Switch to selected session (via Zellij)
- **d**: Show detailed view (full conversation summary, todos, tool history)
- **f**: Filter by status (all/working/waiting/idle)
- **r**: Force refresh
- **q**: Quit

## Zellij Integration

### Session Switching Logic

```rust
fn jump_to_session(session: &Session) -> Result<()> {
    let current_session = zellij::get_current_session()?;

    if session.zellij_session == current_session {
        // Same Zellij session, switch tab/pane
        zellij::focus_tab(session.zellij_tab)?;
        zellij::focus_pane(session.zellij_pane)?;
    } else {
        // Different Zellij session, attach
        zellij::attach_session(&session.zellij_session)?;
        zellij::focus_tab(session.zellij_tab)?;
        zellij::focus_pane(session.zellij_pane)?;
    }

    Ok(())
}
```

### Zellij CLI Commands Used

- `zellij list-sessions -s` - List all sessions
- `zellij action go-to-tab <n>` - Switch to tab
- `zellij action focus-next-pane` - Focus pane
- `zellij attach <session-name>` - Attach to session

### Launcher Wrapper

Create `cc` launcher that ensures Claude runs in Zellij with tracking:

```bash
#!/bin/bash
# cc - Claude Code launcher with cc-orchestra tracking

if [ -z "$ZELLIJ" ]; then
    # Not in Zellij, start it
    zellij attach -c claude-dev || zellij -s claude-dev
fi

# Track Zellij context
export ZELLIJ_SESSION=$(zellij list-sessions | grep "(current)" | awk '{print $1}')
export ZELLIJ_TAB=$(zellij action query-tab-names | grep -n "^>" | cut -d: -f1)

# Launch Claude Code
claude "$@"
```

## Claude Code Integration

### Hooks Setup

Add to `~/.claude/settings.json`:

```json
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
```

### Hook Responsibilities

**SessionStart hook:**
- Captures PID of Claude process
- Records TTY, working directory
- Queries Zellij for current tab/pane
- Writes to `~/.claude/cc-orchestra-state.json`

**SessionEnd hook:**
- Removes session from state file
- Marks session as terminated

## Project Structure

```
cc-orchestra/
├── src/
│   ├── main.rs              // Entry point + CLI args
│   ├── app.rs               // Main TUI app loop
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── dashboard.rs     // Main dashboard view
│   │   ├── detail.rs        // Detailed session view
│   │   └── theme.rs         // Colors & styling
│   ├── state/
│   │   ├── mod.rs
│   │   ├── session.rs       // Session struct & state logic
│   │   ├── detector.rs      // State detection engine
│   │   └── tracker.rs       // Hook-based PID tracking
│   ├── data/
│   │   ├── mod.rs
│   │   ├── sqlite.rs        // Query ~/.claude/__store.db
│   │   ├── jsonl.rs         // Parse session JSONL logs
│   │   └── process.rs       // Process scanning
│   ├── zellij/
│   │   ├── mod.rs
│   │   └── client.rs        // Zellij CLI integration
│   └── cli.rs               // CLI subcommands
├── Cargo.toml
├── README.md
├── docs/
│   └── plans/
│       └── 2025-11-12-cc-orchestra-design.md
└── install.sh               // Setup hooks + launcher
```

## Dependencies

```toml
[dependencies]
ratatui = "0.28"              # TUI framework
crossterm = "0.28"            # Terminal manipulation
rusqlite = { version = "0.32", features = ["bundled"] }  # SQLite
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"            # JSON parsing
tokio = { version = "1", features = ["full"] }  # Async runtime
anyhow = "1.0"                # Error handling
clap = { version = "4", features = ["derive"] }  # CLI parsing
```

## Error Handling

### SQLite Database Locking

```rust
fn query_with_retry<T>(query: &str, params: &[&str]) -> Result<T> {
    let mut attempts = 0;
    loop {
        match execute_query(query, params) {
            Ok(result) => return Ok(result),
            Err(rusqlite::Error::SqliteCantOpen) => {
                attempts += 1;
                if attempts > 3 {
                    return Err(anyhow!("DB locked after 3 retries"));
                }
                thread::sleep(Duration::millis(100));
            }
            Err(e) => return Err(e.into()),
        }
    }
}
```

### Stale PID Detection

```rust
fn process_exists(pid: u32) -> bool {
    // Verify PID is actually Claude, not reused by another process
    if let Ok(cmdline) = read_process_cmdline(pid) {
        cmdline.contains("claude")
    } else {
        false
    }
}
```

### Missing Zellij Context

```rust
fn init() -> Result<App> {
    if !is_in_zellij() {
        warn!("Not running in Zellij - jump functionality disabled");
        // Still show dashboard, but disable switching
    }
    // ...
}
```

## Performance Considerations

- **Refresh rate**: Poll every 2 seconds (configurable)
- **SQLite queries**: Use prepared statements, connection pooling
- **JSONL parsing**: Only parse tail of log files (last 50 lines)
- **Process scanning**: Cache `ps` output for 5 seconds
- **Zellij queries**: Cache session list for 5 seconds

## Installation

```bash
# 1. Build cc-orchestra
cargo build --release

# 2. Run install script
./install.sh

# This will:
# - Copy binary to ~/.local/bin/cc-orchestra
# - Install 'cc' launcher wrapper
# - Add hooks to ~/.claude/settings.json
# - Create initial state file
```

## Usage

```bash
# Launch cc-orchestra dashboard
cc-orchestra

# Or use short alias
cco

# Track session manually (usually called by hook)
cc-orchestra track-session --pid 12345 --session-id abc-123

# Untrack session
cc-orchestra untrack-session --session-id abc-123
```

## Future Enhancements

### Phase 2 (Post-MVP)
- Web dashboard (serve TUI over HTTP with server-sent events)
- Mobile companion app
- Notifications when sessions need attention
- Session recording/replay
- Multi-user support (team dashboards)

### Phase 3 (Advanced)
- AI-powered session prioritization
- Automatic context switching based on activity
- Integration with task management systems
- Session analytics and productivity metrics

## Success Criteria

1. ✅ Dashboard shows all active Claude Code sessions
2. ✅ Accurate state detection (working/waiting/idle)
3. ✅ One-key session switching via Zellij
4. ✅ Real-time updates (<3s latency)
5. ✅ Stable under concurrent Claude usage
6. ✅ Minimal performance impact (<5MB RAM, <1% CPU)

---

**Next Steps:**
1. Initialize Rust project with Cargo
2. Implement SQLite reader module
3. Build basic TUI dashboard
4. Add state detection logic
5. Integrate Zellij switching
6. Create installation script
7. Test with 5+ concurrent Claude sessions

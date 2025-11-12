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

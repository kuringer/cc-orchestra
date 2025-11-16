# Hooks-Only Architecture Migration

**Date**: 2025-11-16
**Status**: Approved
**Approach**: Minimal hooks-only architecture

## Overview

Migrating cc-orchestra from hybrid architecture (hooks + JSONL file reading) to pure hooks-only architecture. This eliminates JSONL file reading which was causing conversation history corruption issues.

## Problem Statement

Current implementation reads JSONL files (`~/.claude/projects/<cwd>/<session_id>.jsonl`) on every dashboard refresh to detect session state. This caused:
- Conversation history corruption
- Performance overhead (I/O on every refresh)
- Complexity (parsing JSONL, handling file modifications, etc.)

## Solution: Minimal Hooks-Only Architecture

Use only Claude Code hooks for state tracking, eliminating all JSONL file reading.

### Hooks Configuration

**Three hooks**:

1. **SessionStart** (existing) - Track new sessions
2. **SessionEnd** (existing) - Remove sessions from tracking
3. **PostToolUse** (NEW) - Detect AskUserQuestion calls → mark session as waiting
4. **UserPromptSubmit** (NEW) - Clear waiting flag when user responds

### State Detection Logic

Simplified state detection without JSONL:

```rust
pub fn detect_state(session_info: &SessionInfo) -> SessionState {
    // 1. Dead: process doesn't exist
    if !process::process_exists(session_info.pid) {
        return SessionState::Dead;
    }

    // 2. WaitingForInput: set by PostToolUse hook
    if session_info.waiting_for_input {
        return SessionState::WaitingForInput;
    }

    // 3. Idle: session older than 30 minutes
    let now = chrono::Utc::now().timestamp();
    let age_secs = now - session_info.started_at;
    if age_secs > 1800 {
        return SessionState::Idle;
    }

    // 4. Default: Working
    SessionState::Working
}
```

### Data Model

**Kept**:
- `SessionInfo` struct (pid, session_id, tmux info, started_at)
- `waiting_for_input: bool` flag
- `waiting_since: Option<i64>` timestamp

**Removed**:
- `SessionActivity` struct (last_event_type, last_content_type, tool_name, timestamp, file_modified_at)
- All JSONL reading code
- File modification time checks
- Last message type detection

### Trade-offs

**Lost capabilities**:
- Cannot distinguish Working vs Waiting states (when Claude is working vs waiting for user)
- No last activity timestamp
- No tool name tracking
- No file modification time detection

**Gained benefits**:
- No JSONL reading → no history corruption
- Simpler code
- Better performance (no I/O on every refresh)
- Instant state updates via hooks (no polling delay)

## Implementation Flow

### Session Lifecycle

```
SessionStart → track-session → state.json (waiting_for_input=false)
                                     ↓
Claude works → Dashboard shows "Working" (default)
                                     ↓
Claude calls AskUserQuestion → PostToolUse → mark-waiting
                                     ↓
                            state.json (waiting_for_input=true)
                                     ↓
                    Dashboard shows "WaitingForInput" + sound alert
                                     ↓
User responds → UserPromptSubmit → clear-waiting
                                     ↓
                            state.json (waiting_for_input=false)
                                     ↓
                         Dashboard shows "Working" again
                                     ↓
SessionEnd → untrack-session → remove from state.json
```

## Files Changed

### install.sh
- Add PostToolUse hook wrapper (`cc-orchestra-post-tool`)
- Add UserPromptSubmit hook wrapper (`cc-orchestra-user-prompt`)
- Update settings.json template to include both new hooks

### src/main.rs
- Add `ClearWaiting` command to enum
- Implement `ClearWaiting` command handler

### src/app.rs
- Remove `use crate::data::jsonl::SessionActivity`
- Remove `last_activity` field from `Session` struct
- Remove entire JSONL reading block in `refresh()` method
- Simplify state detection - call `detector::detect_state(info)` directly
- Remove hook override logic (now handled in detector)

### src/state/detector.rs
- Simplify `detect_state()` signature - remove JSONL parameters
- Implement new simplified logic
- Fix tests - add missing fields

### src/state/mod.rs
- Fix test - add missing fields (`waiting_for_input`, `waiting_since`)

### src/data/jsonl.rs
- Keep file (may be useful for future features)
- Mark as unused / dead code

## Testing Plan

1. **Compilation**: Fix test errors (missing fields)
2. **Hook installation**: Run `install.sh` and verify hooks in settings.json
3. **SessionStart**: Launch Claude Code session, verify tracking
4. **AskUserQuestion detection**: Trigger AskUserQuestion, verify sound alert
5. **UserPromptSubmit clearing**: Respond to question, verify state clears
6. **SessionEnd**: End session, verify cleanup

## Rollback Plan

If hooks-only approach has issues:
1. Git revert to previous commit
2. Re-enable JSONL reading
3. Investigate alternative approaches (extended hooks, process monitoring)

## Future Enhancements

If we need more state accuracy:
- Add PreToolUse hook to detect when Claude starts working
- Add process monitoring (CPU/IO) for Working vs Waiting distinction
- Track tool execution via PostToolUse for all tools (not just AskUserQuestion)

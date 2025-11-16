#!/bin/bash
set -e

echo "🎻 Installing cc-orchestra..."

# Build release binary
echo "Building release binary..."
cargo build --release

# Install binary and hook wrappers
mkdir -p ~/.local/bin
cp target/release/cc-orchestra ~/.local/bin/
chmod +x ~/.local/bin/cc-orchestra

# Install tmux wrapper script
cp scripts/ccode ~/.local/bin/ccode
chmod +x ~/.local/bin/ccode

# Create SessionStart hook wrapper
cat > ~/.local/bin/cc-orchestra-session-start << 'WRAPPER_EOF'
#!/bin/bash
# Read hook input JSON from stdin
input=$(cat)

# Extract session_id from JSON
session_id=$(echo "$input" | jq -r '.session_id // empty')

if [ -z "$session_id" ]; then
    echo "Error: No session_id in hook input" >&2
    exit 1
fi

# Build track-session command with tmux info if available
cmd="cc-orchestra track-session --pid $PPID --session-id \"$session_id\""

# Capture tmux info if in tmux
if [ -n "$TMUX_PANE" ]; then
    cmd="$cmd --tmux-pane \"$TMUX_PANE\""

    # Get session and window from tmux
    session_window=$(tmux display-message -p '#S:#I' 2>/dev/null)
    if [ -n "$session_window" ]; then
        session_name=$(echo "$session_window" | cut -d: -f1)
        window_index=$(echo "$session_window" | cut -d: -f2)
        cmd="$cmd --tmux-session \"$session_name\" --tmux-window $window_index"
    fi
fi

# Execute the command
eval exec $cmd
WRAPPER_EOF
chmod +x ~/.local/bin/cc-orchestra-session-start

# Create SessionEnd hook wrapper
cat > ~/.local/bin/cc-orchestra-session-end << 'WRAPPER_EOF'
#!/bin/bash
# Read hook input JSON from stdin
input=$(cat)

# Extract session_id from JSON
session_id=$(echo "$input" | jq -r '.session_id // empty')

if [ -z "$session_id" ]; then
    echo "Error: No session_id in hook input" >&2
    exit 1
fi

# Untrack the session
exec cc-orchestra untrack-session --session-id "$session_id"
WRAPPER_EOF
chmod +x ~/.local/bin/cc-orchestra-session-end

# Create Stop hook wrapper
cat > ~/.local/bin/cc-orchestra-stop << 'WRAPPER_EOF'
#!/bin/bash
# Read hook input JSON from stdin
input=$(cat)

# Extract session_id from JSON
session_id=$(echo "$input" | jq -r '.session_id // empty')

if [ -n "$session_id" ]; then
    exec cc-orchestra update-activity --session-id "$session_id"
fi
WRAPPER_EOF
chmod +x ~/.local/bin/cc-orchestra-stop

# Create UserPromptSubmit hook wrapper
cat > ~/.local/bin/cc-orchestra-user-input << 'WRAPPER_EOF'
#!/bin/bash
# Read hook input JSON from stdin
input=$(cat)

# Extract session_id from JSON
session_id=$(echo "$input" | jq -r '.session_id // empty')

if [ -n "$session_id" ]; then
    exec cc-orchestra update-user-input --session-id "$session_id"
fi
WRAPPER_EOF
chmod +x ~/.local/bin/cc-orchestra-user-input

# Create PostToolUse hook wrapper for AskUserQuestion
cat > ~/.local/bin/cc-orchestra-asking-question << 'WRAPPER_EOF'
#!/bin/bash
# Read hook input JSON from stdin
input=$(cat)

# Extract session_id and tool_name from JSON
session_id=$(echo "$input" | jq -r '.session_id // empty')
tool_name=$(echo "$input" | jq -r '.tool_name // empty')

# Only track if it's AskUserQuestion tool
if [ -n "$session_id" ] && [ "$tool_name" = "AskUserQuestion" ]; then
    exec cc-orchestra update-asking-question --session-id "$session_id"
fi
WRAPPER_EOF
chmod +x ~/.local/bin/cc-orchestra-asking-question

# Create Notification hook wrapper for permission prompts
cat > ~/.local/bin/cc-orchestra-permission-prompt << 'WRAPPER_EOF'
#!/bin/bash
# Read hook input JSON from stdin
input=$(cat)

# Extract session_id and notification_type from JSON
session_id=$(echo "$input" | jq -r '.session_id // empty')
notification_type=$(echo "$input" | jq -r '.notification_type // empty')

# Only track if it's a permission prompt
if [ -n "$session_id" ] && [ "$notification_type" = "permission_prompt" ]; then
    exec cc-orchestra update-awaiting-permission --session-id "$session_id"
fi
WRAPPER_EOF
chmod +x ~/.local/bin/cc-orchestra-permission-prompt

echo "✓ Binary and hook wrappers installed to ~/.local/bin/"

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
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-session-start"
      }]
    }],
    "SessionEnd": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-session-end"
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-stop"
      }]
    }],
    "UserPromptSubmit": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-user-input"
      }]
    }],
    "PostToolUse": [{
      "matcher": "AskUserQuestion",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-asking-question"
      }]
    }],
    "Notification": [{
      "matcher": "permission_prompt",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-permission-prompt"
      }]
    }]
  }
'
else
    echo "✓ Creating settings.json with hooks"
    cat > "$SETTINGS_FILE" << 'EOF'
{
  "hooks": {
    "SessionStart": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-session-start"
      }]
    }],
    "SessionEnd": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-session-end"
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-stop"
      }]
    }],
    "UserPromptSubmit": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-user-input"
      }]
    }],
    "PostToolUse": [{
      "matcher": "AskUserQuestion",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-asking-question"
      }]
    }],
    "Notification": [{
      "matcher": "permission_prompt",
      "hooks": [{
        "type": "command",
        "command": "cc-orchestra-permission-prompt"
      }]
    }]
  }
}
EOF
fi

echo ""
echo "✅ Installation complete!"
echo ""
echo "Usage:"
echo "  ccode [args]              # Launch Claude Code in tmux window (recommended)"
echo "  cc-orchestra              # Launch dashboard"
echo "  cc-orchestra --help       # Show help"
echo ""
echo "Next steps:"
echo "  1. Start a new session: ccode"
echo "  2. Run dashboard: cc-orchestra"
echo "  3. Press Enter to switch to sessions"
echo ""
echo "Note: Sessions started with 'ccode' will be tracked in tmux"
echo "      Sessions outside tmux will show ⚠️ indicator"

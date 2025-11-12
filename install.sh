#!/bin/bash
set -e

echo "🎻 Installing cc-orchestra..."

# Build release binary
echo "Building release binary..."
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

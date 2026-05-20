#!/usr/bin/env bash
# HipCortex MCP server installer
# Supports: Linux, macOS, Windows (Git Bash / WSL)
# Usage: curl -fsSL https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/mcp/install.sh | bash

set -e

INSTALL_DIR="${HOME}/.hipcortex-mcp"
REPO="https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/mcp/server.py"

# Resolve full path (handles ~ on Windows Git Bash)
INSTALL_DIR_FULL="$(cd "$(dirname "$INSTALL_DIR")" 2>/dev/null && pwd || echo "$HOME")/.hipcortex-mcp"
mkdir -p "$INSTALL_DIR_FULL"

echo "Installing HipCortex MCP server to $INSTALL_DIR_FULL..."

# Download server.py
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$REPO" -o "$INSTALL_DIR_FULL/server.py"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$REPO" -O "$INSTALL_DIR_FULL/server.py"
else
    echo "Error: curl or wget required" >&2; exit 1
fi

chmod +x "$INSTALL_DIR_FULL/server.py" 2>/dev/null || true

# Install requests — handle PEP 668 (Raspberry Pi OS / Ubuntu 23+) and Windows
echo "Installing Python 'requests' package..."
if python3 -m pip install requests --quiet 2>/dev/null; then
    : # success
elif python3 -m pip install requests --quiet --break-system-packages 2>/dev/null; then
    : # PEP 668 system Python
elif python -m pip install requests --quiet 2>/dev/null; then
    : # Windows / fallback
else
    echo "Warning: could not install 'requests'. Install manually: pip install requests" >&2
fi

# Detect Python binary
PYTHON_BIN="python3"
command -v python3 >/dev/null 2>&1 || PYTHON_BIN="python"

echo ""
echo "✓ HipCortex MCP server installed!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "CURSOR — add to .cursor/mcp.json in your project root:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cat <<JSON
{
  "mcpServers": {
    "hipcortex": {
      "command": "$PYTHON_BIN",
      "args": ["$INSTALL_DIR_FULL/server.py"],
      "env": { "HIPCORTEX_URL": "http://localhost:3030" }
    }
  }
}
JSON

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "CLAUDE CODE — add to ~/.claude/settings.json → mcpServers:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cat <<JSON
"hipcortex": {
  "command": "$PYTHON_BIN",
  "args": ["$INSTALL_DIR_FULL/server.py"],
  "env": { "HIPCORTEX_URL": "http://localhost:3030" }
}
JSON

echo ""
echo "Tip: Set HIPCORTEX_URL=https://hipcortex.fly.dev to use the managed free tier."
echo "     Set HIPCORTEX_API_KEY=<your-key> if using a Pro/Team tier with API keys."

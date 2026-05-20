#!/usr/bin/env bash
set -e
INSTALL_DIR="${HOME}/.hipcortex-mcp"
REPO="https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/mcp/server.py"
echo "Installing HipCortex MCP server to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
curl -fsSL "$REPO" -o "$INSTALL_DIR/server.py"
chmod +x "$INSTALL_DIR/server.py"
pip install requests -q
echo ""
echo "✓ Installed to $INSTALL_DIR/server.py"
echo ""
echo "Cursor (.cursor/mcp.json):"
echo "{\"mcpServers\":{\"hipcortex\":{\"command\":\"python\",\"args\":[\"$INSTALL_DIR/server.py\"],\"env\":{\"HIPCORTEX_URL\":\"http://localhost:3030\"}}}}"
echo ""
echo "Claude Code (~/.claude/settings.json → mcpServers):"
echo "\"hipcortex\":{\"command\":\"python\",\"args\":[\"$INSTALL_DIR/server.py\"],\"env\":{\"HIPCORTEX_URL\":\"http://localhost:3030\"}}"

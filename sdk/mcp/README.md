# HipCortex MCP Server

Persistent memory for AI coding assistants — Cursor, Claude Code, Windsurf, Zed AI.

**Protocol:** MCP 2024-11-05 (JSON-RPC 2.0 over stdio)  
**Dependencies:** Python 3.9+ · `pip install requests`

## Quick install

```bash
curl -fsSL https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/mcp/install.sh | bash
```

## Connect to Cursor

Add to `.cursor/mcp.json` in your project root:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["~/.hipcortex-mcp/server.py"],
      "env": { "HIPCORTEX_URL": "http://localhost:3030" }
    }
  }
}
```

## Connect to Claude Code

Add to `~/.claude/settings.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["~/.hipcortex-mcp/server.py"],
      "env": {
        "HIPCORTEX_URL": "http://localhost:3030",
        "HIPCORTEX_API_KEY": "sk-your-key"
      }
    }
  }
}
```

## Tools

| Tool | Description |
|------|-------------|
| `add_memory` | Store a decision, finding, or code note |
| `search_memory` | Recall relevant past context by keyword |
| `forget_actor` | Delete all memories for a project scope |
| `get_stats` | Show memory store statistics |

## Start HipCortex server

```bash
# Pre-built binary (no Rust needed)
curl -L https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-linux-arm64 \
  -o hipcortex && chmod +x hipcortex && ./hipcortex

# Managed instance (free tier, always on)
# Set HIPCORTEX_URL=https://hipcortex.fly.dev in mcp.json
```

## Example workflow

In Cursor/Claude Code:
```
Remember that we chose PostgreSQL over SQLite for multi-user support.
→ [add_memory] actor=my-app action=decided target="PostgreSQL over SQLite — multi-user"

What database decisions have we made?
→ [search_memory] query="database"
• [decided] PostgreSQL over SQLite — multi-user (score: 0.94)
```

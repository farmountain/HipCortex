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
| `search_code` | Search the code graph for relevant symbols, functions, or classes |

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

## Shell integration (`hc` commands)

Add to your `~/.bashrc` or `~/.zshrc` to use HipCortex from any terminal:

```bash
# HipCortex shell wrapper — cross-agent memory for Claude Code, Codex, Aider
export HIPCORTEX_URL="${HIPCORTEX_URL:-http://localhost:3030}"

hc-remember() {
    curl -s -X POST "$HIPCORTEX_URL/memory/add" \
      -H "Content-Type: application/json" \
      -d "{\"actor\":\"${HC_ACTOR:-shell}\",\"action\":\"noted\",\"target\":\"$*\"}" \
      | python3 -c "import sys,json; d=json.load(sys.stdin); print('✓' if d.get('success') else '✗', d.get('record_id',''))"
}

hc-recall() {
    curl -s "$HIPCORTEX_URL/memory/search-flat?query=$(python3 -c "import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1]))" "$*")&limit=10" \
      | python3 -c "import sys,json; d=json.load(sys.stdin); [print(m) for m in d.get('memories',[])]"
}

hc-forget() {
    curl -s -X DELETE "$HIPCORTEX_URL/memory/forget/$1" \
      | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'Deleted {d.get(\"records_deleted\",0)} records')"
}
```

Usage:
```bash
hc-remember "We chose JWT over session cookies for stateless auth"
hc-recall "authentication"
hc-forget my-project
```

Works with: Claude Code · OpenAI Codex CLI · Aider · any terminal coding agent.
Same memory server, same data — language-agnostic persistent AI memory.

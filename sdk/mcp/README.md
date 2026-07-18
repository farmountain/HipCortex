# HipCortex Universal MCP Server & Multi-Agent Setup Guide (`v0.5.0`)

HipCortex exposes a native **Model Context Protocol (MCP)** server (`hipcortex.mcp.server`) and REST API (`http://127.0.0.1:3030`) that gives autonomous AI agents multi-tier causal memory, world-model rollout prediction (`POST /worldmodel/rollout`), and `Headroom Mode` token reduction (`59–88% savings`).

---

## 🚀 Instant CLI Setup (`hipcortex setup`)

If you have Python installed, our CLI automatically detects your installed agents and writes the exact MCP configurations:

```bash
pip install hipcortex
hipcortex setup --mode headroom --url http://127.0.0.1:3030
```

---

## Channel status (honesty)

Not every host below has a **wizard installer**. Official matrix: [docs/channels.md](../../docs/channels.md) · `hipcortex channels`.

| Hosts | Status |
|-------|--------|
| Claude Code, Cursor, Windsurf, VS Code MCP, Cline, RooCode | **native / mcp** (wizard) |
| Continue, Copilot, Codex, Aider, Gemini, Amazon Q, Flowise | **guide** |
| Grok Code, Hermes, OpenClaw, Antigravity-specific paths | **claimed** — example JSON only; paths unverified; no `hipcortex install` entry |

---

## 📋 Exact Copy-Paste Configurations

### 1. Claude Code (`claude mcp add`)
Run in your terminal:
```bash
claude mcp add hipcortex python -m hipcortex.mcp.server --mode headroom
```
Or edit `~/.claude/mcp.json` / `~/.claude/settings.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 2. Cursor IDE (`.cursor/mcp.json`)
Create or edit `.cursor/mcp.json` in your workspace root (or globally in `~/.cursor/mcp.json`):
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 3. Windsurf IDE (`~/.codeium/windsurf/mcp_config.json`)
Add to `~/.codeium/windsurf/mcp_config.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 4. Grok Code (`~/.grok/mcp.json`) — **claimed / unverified**
> Status: **claimed** — not in `hipcortex install` registry. Path is a community guess; verify against your Grok host docs before relying on it.

Example (if your host uses standard MCP JSON):
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030", "OPTIMIZATION_MODE": "headroom" }
    }
  }
}
```

### 5. Hermes Agent (`~/.hermes/mcp_config.json`) — **claimed / unverified**
> Status: **claimed** — no installer; config path not CI-verified.
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 6. OpenClaw Orchestrator (`~/.openclaw/mcp.json`) — **claimed / unverified**
> Status: **claimed** — no installer; config path not CI-verified.
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 7. Cline / RooCode (VS Code Extension MCP Settings)
Open VS Code $\rightarrow$ `Cline Settings` (or `RooCode Settings`) $\rightarrow$ `MCP Servers` $\rightarrow$ Add New:
- **Server Name**: `hipcortex`
- **Command**: `python`
- **Arguments**: `["-m", "hipcortex.mcp.server", "--mode", "headroom"]`
- **Environment**: `{"HIPCORTEX_URL": "http://127.0.0.1:3030"}`

### 8. OpenAI Codex CLI (`codex --mcp-server`)
Pass via command line or `~/.codex/config.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"]
    }
  }
}
```

### 9. Aider AI Pair Programmer (`--mcp-server`)
Launch Aider with HipCortex MCP:
```bash
aider --mcp-server "python -m hipcortex.mcp.server --mode headroom"
```

### 10. Gemini CLI & Antigravity IDE — **guide / claimed**
> **Gemini:** guide-only (manual MCP). **Antigravity:** **claimed** as a distinct product path — prefer the **VS Code VSIX** (`hipcortex-memory-0.5.4.vsix`) if the host is VS Code–compatible. The path below is unverified.

Example MCP fragment:
```json
{
  "command": "python",
  "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
  "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
}
```

### 11. Amazon Q Developer (`~/.amazonq/mcp.json`)
Add to `~/.amazonq/mcp.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["-m", "hipcortex.mcp.server", "--mode", "headroom"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
}
```

### 12. Direct HTTP JSON-RPC / REST Mode
For lightweight custom harnesses or shell wrappers (`hc` commands), call the local server directly over loopback (`127.0.0.1:3030`):
```bash
curl -X POST http://127.0.0.1:3030/memory/add \
  -H "Content-Type: application/json" \
  -d '{"actor": "agent", "action": "decided", "target": "use sqlite over postgres", "record_type": "Symbolic"}'
```

---

## 🛠️ Available MCP Tools

| Tool Name | Arguments | Description |
| :--- | :--- | :--- |
| `add_memory` | `actor`, `action`, `target`, `record_type` (`Working`/`ShortTerm`/`LongTerm`/`Causal`/`Procedural`), `confidence` | Ingests causal memory node into topological graph and Merkle chain. |
| `search_memory` | `query`, `actor`, `limit` | Retrieves Top-K memory context using Personalized PageRank topological scoring. |
| `forget_actor` | `actor` | Deletes all memory records and causal edges for a specific actor (`GDPR Right to Forget`). |
| `get_stats` | *none* | Displays server health, Merkle chain integrity, Top-5 Headroom mode status, and tier breakdowns. |
| `search_code` | `query`, `symbol_type` | Queries code intelligence graph (`GitNexus` integration). |

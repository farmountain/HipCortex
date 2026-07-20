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
| Antigravity, Hermes, OpenClaw, Grok Code / Grok Build | **mcp** — `hipcortex install` ([docs/hosts/README.md](../../docs/hosts/README.md)) |
| VS Code / Antigravity VSIX | **native** — `hipcortex-memory-0.5.7.vsix` ([release](https://github.com/farmountain/HipCortex/releases/download/v0.5.7/hipcortex-memory-0.5.7.vsix)) |

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

### 4. Grok Code / Grok Build — **mcp**
> Status: **mcp** — `hipcortex install` writes `~/.grok/config.toml` `[mcp_servers.hipcortex]`. Notes: [docs/hosts/grok-build.md](../../docs/hosts/grok-build.md). Uninstall: `hipcortex uninstall --channel grok`.

Example (if your host uses standard MCP JSON; path may vary):
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

### 5. Hermes Agent (`~/.hermes/config.yaml`) — **mcp**
> Status: **mcp** — `hipcortex install` merges `mcp_servers.hipcortex` (skips if `~/.hermes` missing). See [docs/hosts/README.md](../../docs/hosts/README.md).

```yaml
mcp_servers:
  hipcortex:
    command: python
    args: ["/home/YOU/.hipcortex-mcp/server.py"]
    env:
      HIPCORTEX_URL: http://127.0.0.1:3030
```

### 6. OpenClaw Orchestrator (`~/.openclaw/openclaw.json`) — **mcp**
> Status: **mcp** — `hipcortex install` merges `mcp.servers.hipcortex` (JSON5 → sidecar + `openclaw mcp add` hint). See [docs/hosts/README.md](../../docs/hosts/README.md).

```json
{
  "mcp": {
    "servers": {
      "hipcortex": {
        "command": "python",
        "args": ["/home/YOU/.hipcortex-mcp/server.py"],
        "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
      }
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

### 10. Gemini CLI & Antigravity IDE — **guide / mcp**
> **Gemini CLI:** guide-only (manual MCP). **Antigravity:** **mcp** — `hipcortex install` writes `~/.gemini/antigravity/mcp_config.json`. Also VSIX (`hipcortex-memory-0.5.7.vsix`) if VS Code–compatible. See [docs/hosts/README.md](../../docs/hosts/README.md).

Example MCP fragment (`mcpServers` shape):
```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["/home/YOU/.hipcortex-mcp/server.py"],
      "env": { "HIPCORTEX_URL": "http://127.0.0.1:3030" }
    }
  }
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

## Available MCP Tools (18)

`serverInfo.version` = **0.5.0** (product / VERSION). Full capability matrix: [docs/capabilities.md](../../docs/capabilities.md).

| Tool Name | Arguments | Description |
| :--- | :--- | :--- |
| `add_memory` | `actor`, `action`, `target`, `record_type`, `ttl_seconds` | Store memory (Temporal/Symbolic/…) into store + graph |
| `search_memory` | `query`, `actor`, `limit` | Keyword search; prefer `get_live_beliefs` first for project-state |
| `forget_actor` | `actor` | GDPR wipe all memories for actor |
| `get_stats` | *none* | Store stats (counts by type/actor) |
| `search_code` | `query`, `limit` | Code knowledge graph symbols (after `hipcortex index`) |
| `link_memories` | `source_id`, `target_id`, `relation` | Directed edge on CausalTopoGraph (safety-gated) |
| `get_neighbors` | `record_id`, `limit` | Local graph neighbors of a memory UUID |
| `search_related` | `seed_id`, `limit` | PPR-ranked related memories from seed UUID |
| `graph_ppr` | `seed`, `limit` | PPR over live topo symbolic nodes (`GET /topo/ppr`) |
| `deconstruct_hypothesis` | `text`, `llm_json?`, `apply?` | Parse hypothesis → edges; optional apply (`/topo/*`) |
| `check_topo_edge` | `from`, `to` | Contradiction / cycle check before linking |
| `rollout` | `initial_state`, `actions?`, `mode`, `goal_state?`, `iterations?`, `max_depth?` | World-model multi-step (dirichlet / mcts / ensemble) |
| `can_execute` | `operation` | SelfModel capability gate |
| `delete_memory` | `record_id` | Delete single memory by UUID |
| `get_live_beliefs` | `actor?`, `limit` | Latest unique beliefs per actor+action (call first) |
| `purge_expired` | *none* | Purge TTL-expired records |
| `reflect` | `query` | AureusBridge CoT / hypothesis sampling |
| `predict` | `state`, `action` | Single-step `P(s'|s,a)` world-model predict |

**VS Code LM tools (parallel, 10):** see [vscode-extension/README.md](../../vscode-extension/README.md) — includes health + causal + topo suite without full MCP set.

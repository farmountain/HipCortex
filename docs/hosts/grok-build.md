# Grok Build / Grok Code — HipCortex MCP guide

**Status:** `guide` (no `hipcortex install` target yet)  
**Matrix:** [docs/channels.yaml](../channels.yaml) · [docs/channels.md](../channels.md)  
**Why guide:** xAI Grok Build documents MCP “out of the box,” but a stable user/project config path is not product-locked for auto-write. Use copy-paste until Phase 6c detects a confirmed path.

## Prerequisites

1. HipCortex HTTP server reachable (default `http://127.0.0.1:3030`).
2. MCP stdio script installed:

```bash
pip install hipcortex
hipcortex install   # copies ~/.hipcortex-mcp/server.py among other hosts
```

Or ensure:

```text
~/.hipcortex-mcp/server.py
```

## Sample MCP JSON (stdio)

Many hosts use a Cursor-shaped `mcpServers` object. If Grok Build / Grok Code accepts standard MCP client JSON, merge:

```json
{
  "mcpServers": {
    "hipcortex": {
      "command": "python",
      "args": ["C:/Users/YOU/.hipcortex-mcp/server.py"],
      "env": {
        "HIPCORTEX_URL": "http://127.0.0.1:3030"
      }
    }
  }
}
```

Unix example args:

```json
"args": ["/home/YOU/.hipcortex-mcp/server.py"]
```

Prefer absolute path to `server.py` (same entry shape as `hipcortex install` for Cursor/Antigravity). Use your real Python if `python` is not on PATH:

```json
"command": "C:/Python313/python.exe"
```

Optional env:

| Env | Purpose |
|-----|---------|
| `HIPCORTEX_URL` | REST base (required for memory tools) |
| `OPTIMIZATION_MODE` | e.g. `headroom` if host passes through |

## Project-local option

If the host reads project MCP (Claude/Cursor-style), place the same block under the path your Grok host documents (for example project `.mcp.json` or AGENTS.md-adjacent MCP file). Prefer **project-local** when working across machines so paths can be adjusted per OS.

## Verify

1. Start HipCortex: `hipcortex doctor` (or webserver on 3030).
2. Restart Grok Build / host so it reloads MCP.
3. Confirm tools such as memory search / `live_beliefs` appear.
4. Smoke: add a memory via tool or `curl` to `/memory/add`, then recall.

## Not first-class yet

| Action | Supported? |
|--------|------------|
| `hipcortex install` auto-write | **No** (guide only) |
| `hipcortex uninstall --channel grok-*` | **No** |
| Sample JSON in this doc | **Yes** |
| Antigravity / Hermes / OpenClaw installers | **Yes** — `hipcortex install` |

When official config path stabilizes, bump status `guide` → `mcp` in `channels.yaml` and add `_install_grok_*` next to other hosts in `sdk/python/hipcortex/cli.py`.

## Related

- [docs/hosts/README.md](README.md) — all Phase 6 hosts  
- [sdk/mcp/README.md](../../sdk/mcp/README.md) — multi-agent MCP samples  
- Install code: commit `3cd4359` (Antigravity / Hermes / OpenClaw)

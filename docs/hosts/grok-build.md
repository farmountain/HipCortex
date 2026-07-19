# Grok Build / Grok Code — HipCortex MCP

**Status:** `mcp`  
**Matrix:** [docs/channels.yaml](../channels.yaml) · [docs/channels.md](../channels.md)  
**Config:** `~/.grok/config.toml` (`GROK_CONFIG_PATH` override)  
**Shape:** `[mcp_servers.hipcortex]` stdio table (official Grok Build user-guide `07-mcp-servers.md`)

## Prerequisites

1. HipCortex HTTP server reachable (default `http://127.0.0.1:3030`).
2. Grok Build installed (so `~/.grok` exists), **or** set `GROK_CONFIG_PATH` to a config file path.
3. MCP stdio script present:

```bash
pip install hipcortex
hipcortex install   # select Grok Build; also copies ~/.hipcortex-mcp/server.py
```

## Install / uninstall

```bash
hipcortex install
# → merge [mcp_servers.hipcortex] into ~/.grok/config.toml

hipcortex uninstall --channel grok
# aliases: grok-build, grok-code
```

| Action | Supported? |
|--------|------------|
| `hipcortex install` auto-write | **Yes** |
| Idempotent created / updated / unchanged | **Yes** |
| Skip if `~/.grok` missing (unless `GROK_CONFIG_PATH`) | **Yes** |
| `hipcortex uninstall --channel grok` | **Yes** |
| Preserve other MCP servers (kaggle, headroom, …) | **Yes** |

## Written config shape

```toml
[mcp_servers.hipcortex]
command = "<sys.executable>"
args = ["<home>/.hipcortex-mcp/server.py"]
env = { HIPCORTEX_URL = "http://127.0.0.1:3030" }
enabled = true
```

Override config path:

```bash
# Windows PowerShell
$env:GROK_CONFIG_PATH = "D:\path\to\config.toml"
hipcortex install
```

## Manual equivalent

If you prefer not to use the wizard, append the table above to `~/.grok/config.toml`. Use absolute path to `server.py` and your real Python executable.

Optional env:

| Env | Purpose |
|-----|---------|
| `HIPCORTEX_URL` | REST base (required for memory tools) |
| `GROK_CONFIG_PATH` | Non-default Grok config file for install/uninstall |
| `OPTIMIZATION_MODE` | e.g. `headroom` if host passes through |

## Verify

1. Start HipCortex: `hipcortex doctor` (or webserver on 3030).
2. Restart Grok Build so it reloads MCP.
3. Confirm tools (memory search / `live_beliefs`) appear.
4. Smoke: add a memory via tool or `curl` to `/memory/add`, then recall.

## Related

- [docs/hosts/README.md](README.md) — all Phase 6 hosts  
- [sdk/mcp/README.md](../../sdk/mcp/README.md) — multi-agent MCP samples  
- Installer: `sdk/python/hipcortex/cli.py` (`_install_grok` / `_uninstall_grok`)

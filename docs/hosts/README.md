# Host install notes (Phase 6)

HipCortex MCP / wizard targets beyond Claude / Cursor / Windsurf.  
**Registry:** [docs/channels.yaml](../channels.yaml) · table [docs/channels.md](../channels.md) · CLI `hipcortex channels`.

```bash
pip install hipcortex
hipcortex install          # interactive — pick Antigravity / Hermes / OpenClaw
hipcortex uninstall --channel antigravity|hermes|openclaw
```

---

## Antigravity IDE — status **mcp**

| | |
|--|--|
| **Wizard** | `hipcortex install` → writes `~/.gemini/antigravity/mcp_config.json` |
| **Shape** | `mcpServers.hipcortex` (same as Cursor) |
| **Also** | VS Code–compatible VSIX: `hipcortex-memory-0.5.4.vsix` / Open VSX |
| **Skip** | Permission errors only (creates parent dirs) |

```bash
hipcortex install   # select Antigravity
# → ~/.gemini/antigravity/mcp_config.json
```

Manual equivalent: merge stdio entry pointing at `~/.hipcortex-mcp/server.py` with `HIPCORTEX_URL`.

---

## Hermes Agent — status **mcp**

| | |
|--|--|
| **Wizard** | `hipcortex install` → merge `mcp_servers.hipcortex` into `~/.hermes/config.yaml` |
| **Shape** | YAML map under `mcp_servers:` (`command` / `args` / `env`) |
| **Skip** | If `~/.hermes` directory does not exist (install Hermes first) |
| **CLI** | Host may also expose `hermes mcp` for probe |

```yaml
mcp_servers:
  hipcortex:
    command: python
    args:
      - /home/YOU/.hipcortex-mcp/server.py
    env:
      HIPCORTEX_URL: http://127.0.0.1:3030
```

---

## OpenClaw — status **mcp**

| | |
|--|--|
| **Wizard** | `hipcortex install` → `mcp.servers.hipcortex` in `~/.openclaw/openclaw.json` |
| **Override** | `OPENCLAW_CONFIG_PATH` for non-default config file |
| **JSON5** | If file is not plain JSON: writes sidecar `openclaw.hipcortex.mcp.json` + prints `openclaw mcp add …` hint |
| **Skip** | If `~/.openclaw` missing (unless `OPENCLAW_CONFIG_PATH` set) |

```bash
# after wizard, or manual:
openclaw mcp add hipcortex --command python --arg ~/.hipcortex-mcp/server.py --env HIPCORTEX_URL=http://127.0.0.1:3030
```

---

## Grok Build / Grok Code — status **guide**

No auto-installer. Sample MCP JSON and verify steps:

→ **[grok-build.md](grok-build.md)**

---

## Code reference

| Host | Installer | Commit |
|------|-----------|--------|
| Antigravity / Hermes / OpenClaw | `sdk/python/hipcortex/cli.py` | `3cd4359` |
| Grok | docs only | Phase 6B |

Tests: `sdk/python/tests/test_cli_install.py` (host write / idempotent / uninstall).

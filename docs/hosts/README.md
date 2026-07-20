# Host install notes (Phase 6)

HipCortex MCP / wizard targets beyond Claude / Cursor / Windsurf.  
**Registry:** [docs/channels.yaml](../channels.yaml) · table [docs/channels.md](../channels.md) · CLI `hipcortex channels`.

**Cursor (Windows global):** correct path is `%APPDATA%/Cursor/mcp.json`. Install migrates hipcortex from legacy `%APPDATA%/mcp.json` into `Cursor/`; uninstall also strips that legacy key.

```bash
pip install hipcortex
hipcortex install          # interactive — pick Antigravity / Hermes / OpenClaw / Grok
hipcortex uninstall --channel antigravity|hermes|openclaw|grok
```

---

## Antigravity IDE — status **mcp**

| | |
|--|--|
| **Wizard** | `hipcortex install` → writes `~/.gemini/antigravity/mcp_config.json` |
| **Shape** | `mcpServers.hipcortex` (same as Cursor) |
| **Also** | VS Code–compatible VSIX: `hipcortex-memory-0.5.8.vsix` / Open VSX |
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

## Grok Build / Grok Code — status **mcp**

| | |
|--|--|
| **Wizard** | `hipcortex install` → `[mcp_servers.hipcortex]` in `~/.grok/config.toml` |
| **Override** | `GROK_CONFIG_PATH` for non-default config file |
| **Shape** | TOML table: `command` / `args` / `env` / `enabled` (stdio) |
| **Skip** | If `~/.grok` missing (unless `GROK_CONFIG_PATH` set) |
| **Uninstall** | `hipcortex uninstall --channel grok` (aliases: `grok-build`, `grok-code`) |

```toml
[mcp_servers.hipcortex]
command = "python"
args = ["/home/YOU/.hipcortex-mcp/server.py"]
env = { HIPCORTEX_URL = "http://127.0.0.1:3030" }
enabled = true
```

Deep dive → **[grok-build.md](grok-build.md)**

---

## Code reference

| Host | Installer | Notes |
|------|-----------|--------|
| Antigravity / Hermes / OpenClaw | `sdk/python/hipcortex/cli.py` | Phase 6a (`3cd4359`) |
| Grok Build / Code | `_install_grok` / `_uninstall_grok` | Phase 6c · `~/.grok/config.toml` |

Tests: `sdk/python/tests/test_cli_install.py` (host write / idempotent / uninstall) · E2E smoke `sdk/python/tests/test_host_install_e2e.py` · CI job **Host install E2E smoke** (`host-install-e2e` in `.github/workflows/ci.yml`) runs on **ubuntu-latest + windows-latest**:

```bash
pytest sdk/python/tests/test_host_install_e2e.py \
  sdk/python/tests/test_cli_install.py \
  -k "antigravity or hermes or openclaw or grok" -q --tb=line
```

# Channel cohesion update (docs + install surfaces)

**Done** (2026-07-20)

- T1 README VSIX URL → `/releases/download/v0.5.7/`
- T2 vscode-extension/README → 0.5.7, 10 LM tools, dual health
- T3 sdk/mcp/README → 18 tools, Grok mcp, VSIX 0.5.7
- T4 docs/hosts, channels.yaml/md, capabilities.md
- T5 cli `_fallback_channels` VSIX 0.5.7
- T6 stale 0.5.4/0.5.5 user-facing refs fixed
- T7 Bundle `hipcortex/install/mcp_server.py` for PyPI wizard; stamp_versions `--mcp` syncs it

**Version policy kept:** product 0.5.0 / VSIX 0.5.7

**Not done (needs credentials + version bump):** live `pip publish` / `npm publish` of 0.5.0 (immutable on registries — needs 0.5.1+ or re-release plan)

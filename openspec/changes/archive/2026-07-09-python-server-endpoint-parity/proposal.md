## Why

The Python MCP server (`sdk/mcp/server.py`) is the primary interface for Claude Desktop and macOS agent deployments. The agent harness comment in `search_memory` directs agents to call `get_live_beliefs` first, but that tool is not registered — causing a `ValueError: Unknown tool` at runtime. Additionally, there is no way for agents using the Python server to trigger TTL-expired record cleanup (the Rust server's background eviction runs automatically but agents cannot trigger it on-demand). Three endpoints missing from the MCP tool registry: `get_live_beliefs`, `purge_expired`, and alignment of `record_type` enum to include `Perception` (currently omitted from the schema).

## What Changes

- **Register `get_live_beliefs` MCP tool**: Proxies `GET /memory/live_beliefs?actor=&limit=` — returns the latest unique belief per `actor+action` pair. This is the "current world state" surface agents should query before answering project-state questions.
- **Register `purge_expired` MCP tool**: Proxies `POST /memory/consolidate` with `dry_run=false` to trigger on-demand cleanup of TTL-expired records.
- **Fix `add_memory` tool schema**: Add `"Perception"` to the `record_type` enum (currently missing from the 4-value list in server.py:92).
- **Add `get_live_beliefs` to harness comment**: Update the `search_memory` description to correctly reference `get_live_beliefs` as a registered tool.

## Capabilities

### New Capabilities

- `mcp-live-beliefs`: `get_live_beliefs` MCP tool exposing the unified live beliefs surface to Claude Desktop agents.
- `mcp-purge-expired`: `purge_expired` MCP tool allowing agents to trigger on-demand TTL eviction.

### Modified Capabilities

- `mcp-add-memory`: `record_type` schema corrected to include `Perception` (5th tier).

## Impact

- **File**: `sdk/mcp/server.py` only — no Rust changes, no TypeScript changes.
- **No breaking changes**: existing 9 tools unchanged, 2 new tools added.
- **Deployment**: Python MCP server restart required to pick up new tool registrations.
- **Risk**: The `/memory/consolidate` endpoint body format must be verified before shipping `purge_expired`.

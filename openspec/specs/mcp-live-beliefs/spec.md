# mcp-live-beliefs Specification

## Purpose
TBD - created by archiving change python-server-endpoint-parity. Update Purpose after archive.
## Requirements
### Requirement: MCP get_live_beliefs Tool
The Python MCP server SHALL expose a `get_live_beliefs` tool to fetch the latest unique beliefs per actor+action pair from the backend.

#### Scenario: Call get_live_beliefs
- **WHEN** the `get_live_beliefs` tool is called via JSON-RPC
- **THEN** it calls the GET /memory/live_beliefs endpoint and returns the unique beliefs formatted for the agent

### Requirement: Deterministic Newest-First Truncation on Beliefs Endpoint
The `GET /memory/beliefs` (`handle_memory_live_beliefs`) endpoint SHALL sort candidate `pinned` beliefs descending by `timestamp` before applying result count limits (`take(limit)` or `.truncate(limit)`). When the total number of matching pinned beliefs exceeds `limit`, the returned JSON `pinned` array MUST contain the `limit` newest beliefs rather than arbitrary or oldest insertion-order beliefs.

#### Scenario: Pinned beliefs truncation on high-volume store
- **WHEN** the store contains `50` records with `priority == "pinned"` across sequential timestamps and `handle_memory_live_beliefs` (`/memory/beliefs`) is invoked with `limit=10`
- **THEN** the returned `pinned` items SHALL consist exactly of the `10` most recently timestamped pinned beliefs


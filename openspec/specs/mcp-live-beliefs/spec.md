# mcp-live-beliefs Specification

## Purpose
TBD - created by archiving change python-server-endpoint-parity. Update Purpose after archive.
## Requirements
### Requirement: MCP get_live_beliefs Tool
The Python MCP server SHALL expose a `get_live_beliefs` tool to fetch the latest unique beliefs per actor+action pair from the backend.

#### Scenario: Call get_live_beliefs
- **WHEN** the `get_live_beliefs` tool is called via JSON-RPC
- **THEN** it calls the GET /memory/live_beliefs endpoint and returns the unique beliefs formatted for the agent


# mcp-purge-expired Specification

## Purpose
TBD - created by archiving change python-server-endpoint-parity. Update Purpose after archive.
## Requirements
### Requirement: MCP purge_expired Tool
The Python MCP server SHALL expose a `purge_expired` tool to clean up expired TTL memories in the backend.

#### Scenario: Call purge_expired
- **WHEN** the `purge_expired` tool is called via JSON-RPC
- **THEN** it triggers consolidation (which purges expired records) and returns the count of deleted records


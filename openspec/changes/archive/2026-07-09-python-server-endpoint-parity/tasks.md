## 1. Schema alignments

- [x] 1.1 Add `Perception` value to `record_type` enum in `add_memory` tool schema in `sdk/mcp/server.py`
- [x] 1.2 Update harness comment in `search_memory` tool description to mention `get_live_beliefs`

## 2. Live Beliefs and Purge tools

- [x] 2.1 Add `get_live_beliefs` tool schema and handler in `sdk/mcp/server.py`
- [x] 2.2 Add `purge_expired` tool schema and handler in `sdk/mcp/server.py`
- [x] 2.3 Add dispatch bindings for both new tools in `dispatch_tool`

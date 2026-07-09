## Approach

Modify `sdk/mcp/server.py` to add `get_live_beliefs` and `purge_expired` MCP tools. This is a pure Python change to align the MCP interface with the capabilities already present in the Rust server.

## Design Decisions

### D1: Proxy to Rust server endpoints

The Python MCP server doesn't execute memory logic or store state; it proxies requests to the running Rust server (default: `http://localhost:3000`).
- `get_live_beliefs` maps to `GET /memory/live_beliefs?actor=&limit=`
- `purge_expired` maps to `POST /memory/consolidate` (which has a side effect of purging expired memories when consolidation runs, or we can use any specific expired route if one is available). Since consolidation filters and cleans up expired records, calling `POST /memory/consolidate` with default parameters is the standard way to trigger it.

### D2: Include "Perception" in `add_memory` tool schema

Align the `record_type` enum field in the `add_memory` tool registration to match the 5 tiers supported by the Rust backend: `Temporal`, `Symbolic`, `Procedural`, `Reflexion`, `Perception`.

## Component Design

### 1. Tool Registrations (in `TOOLS` list in `server.py`)

**`get_live_beliefs`**:
```python
{
    "name": "get_live_beliefs",
    "description": "Get the latest unique beliefs per actor+action pair. Returns the most recent value for each fact the system has observed. Call this FIRST before any project-state question.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "actor": {"type": "string", "description": "Filter to a specific actor (optional)"},
            "limit": {"type": "integer", "default": 20, "description": "Max beliefs to return"},
        },
    },
}
```

**`purge_expired`**:
```python
{
    "name": "purge_expired",
    "description": "Trigger purge of all TTL-expired memory records from the store.",
    "inputSchema": {"type": "object", "properties": {}},
}
```

### 2. Handler functions (in `server.py`)

```python
def handle_get_live_beliefs(args: dict) -> str:
    params: dict = {"limit": args.get("limit", 20)}
    if "actor" in args:
        params["actor"] = args["actor"]
    qs = urllib.parse.urlencode(params)
    result = _get(f"/memory/live_beliefs?{qs}")
    beliefs = result.get("beliefs", [])
    if not beliefs:
        return "No live beliefs found."
    lines = [
        f"• [{b.get('action', '?')}] {b.get('target', '')} (actor: {b.get('actor', '?')})"
        for b in beliefs
    ]
    return f"Live beliefs ({result.get('total', len(beliefs))}):\n" + "\n".join(lines)

def handle_purge_expired(_args: dict) -> str:
    # Trigger consolidation, which purges expired records in the backend
    result = _post("/memory/consolidate", {"dry_run": False})
    deleted = result.get("deleted", 0)
    return f"✓ Purge completed. Deleted {deleted} expired records."
```

### 3. Dispatch Integration

Add mappings in `dispatch_tool`:
```python
"get_live_beliefs": handle_get_live_beliefs,
"purge_expired": handle_purge_expired,
```

## File Map

```
sdk/mcp/server.py
  ├─ TOOLS array (add registrations)
  ├─ + handle_get_live_beliefs()
  ├─ + handle_purge_expired()
  └─ dispatch_tool() dictionary (add mappings)
```

## Verification

1. Run Python syntax checks on `server.py`.
2. Spin up the Rust server on port 3000.
3. Test using `mcp-cli` or custom Python test script sending JSON-RPC requests for the new tools.

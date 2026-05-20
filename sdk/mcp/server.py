#!/usr/bin/env python3
"""HipCortex MCP Server — Model Context Protocol (2024-11-05) over stdio.

Exposes HipCortex memory as tools for Cursor, Claude Code, Windsurf, Zed AI,
and any MCP-compatible AI coding assistant.

Protocol: JSON-RPC 2.0 over stdin/stdout
Dependencies: Python 3.9+ stdlib + requests (pip install requests)

Usage:
    python server.py

Environment variables:
    HIPCORTEX_URL      Base URL (default: http://localhost:3030)
    HIPCORTEX_API_KEY  Optional X-Api-Key for managed SaaS tiers
    HIPCORTEX_TIMEOUT  Request timeout in seconds (default: 10)
"""

from __future__ import annotations

import json
import os
import sys
import urllib.parse
from typing import Any

import requests

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

HIPCORTEX_URL = os.getenv("HIPCORTEX_URL", "http://localhost:3030").rstrip("/")
API_KEY       = os.getenv("HIPCORTEX_API_KEY", "")
TIMEOUT       = int(os.getenv("HIPCORTEX_TIMEOUT", "10"))

# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------

def _headers() -> dict:
    h = {"Content-Type": "application/json"}
    if API_KEY:
        h["X-Api-Key"] = API_KEY
    return h

def _get(path: str) -> dict:
    resp = requests.get(f"{HIPCORTEX_URL}{path}", headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return resp.json()

def _post(path: str, body: dict) -> dict:
    resp = requests.post(f"{HIPCORTEX_URL}{path}", json=body, headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return resp.json()

def _delete(path: str) -> dict:
    resp = requests.delete(f"{HIPCORTEX_URL}{path}", headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return resp.json()

# ---------------------------------------------------------------------------
# MCP tool definitions
# ---------------------------------------------------------------------------

TOOLS = [
    {
        "name": "add_memory",
        "description": (
            "Store a memory record in HipCortex. "
            "Use to remember decisions, code patterns, bug fixes, architectural choices, "
            "or any context that should persist across sessions."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["actor", "action", "target"],
            "properties": {
                "actor": {
                    "type": "string",
                    "description": "Scope identifier — use project name or 'global' (e.g. 'my-app', 'user-42')",
                },
                "action": {
                    "type": "string",
                    "description": "What happened (e.g. 'decided', 'implemented', 'fixed', 'noted')",
                },
                "target": {
                    "type": "string",
                    "description": "The content to remember — be specific and self-contained",
                },
                "record_type": {
                    "type": "string",
                    "enum": ["Temporal", "Symbolic", "Procedural", "Reflexion"],
                    "default": "Temporal",
                },
                "ttl_seconds": {
                    "type": "integer",
                    "description": "Auto-expire after N seconds (omit for permanent memory)",
                },
            },
        },
    },
    {
        "name": "search_memory",
        "description": (
            "Search stored memories by keyword. "
            "Use before starting a task to recall relevant past decisions or context."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {"type": "string", "description": "What to search for"},
                "actor": {"type": "string", "description": "Filter by actor/scope (optional)"},
                "limit": {"type": "integer", "default": 10},
            },
        },
    },
    {
        "name": "forget_actor",
        "description": "Delete all memories for an actor (GDPR right-to-forget / fresh start).",
        "inputSchema": {
            "type": "object",
            "required": ["actor"],
            "properties": {
                "actor": {"type": "string", "description": "Actor whose memories to delete"},
            },
        },
    },
    {
        "name": "get_stats",
        "description": "Get memory store statistics: total records, types, unique actors.",
        "inputSchema": {"type": "object", "properties": {}},
    },
]

# ---------------------------------------------------------------------------
# Tool execution
# ---------------------------------------------------------------------------

def handle_add_memory(args: dict) -> str:
    body = {"actor": args["actor"], "action": args["action"], "target": args["target"]}
    if "record_type" in args:
        body["record_type"] = args["record_type"]
    if "ttl_seconds" in args:
        body["ttl_seconds"] = args["ttl_seconds"]
    result = _post("/memory/add", body)
    return f"✓ Memory stored (id: {result.get('record_id', 'unknown')})\n  [{args['action']}] {args['target']}"


def handle_search_memory(args: dict) -> str:
    body = {"query": args["query"], "limit": args.get("limit", 10)}
    result = _post("/memory/search", body)
    search_results = result.get("results", [])

    if search_results:
        lines = [
            f"• [{r.get('record', {}).get('action', '?')}] {r.get('record', {}).get('target', '')} "
            f"(actor: {r.get('record', {}).get('actor', '?')}, score: {r.get('score', 0):.2f})"
            for r in search_results
        ]
        return f"Found {len(lines)} result(s):\n" + "\n".join(lines)

    # Fallback: keyword query
    params: dict = {"limit": args.get("limit", 10)}
    if "actor" in args:
        params["actor"] = args["actor"]
    qs = urllib.parse.urlencode(params)
    result2 = _get(f"/memory/query?{qs}")
    records = result2.get("records", [])
    if not records:
        return "No memories found."
    lines = [
        f"• [{r.get('action', '?')}] {r.get('target', '')} (actor: {r.get('actor', '?')})"
        for r in records
    ]
    return f"Found {len(lines)} record(s):\n" + "\n".join(lines)


def handle_forget_actor(args: dict) -> str:
    actor = args["actor"]
    result = _delete(f"/memory/forget/{actor}")
    deleted  = result.get("records_deleted", 0)
    symbolic = result.get("symbolic_nodes_deleted", 0)
    return f"✓ Deleted {deleted} records and {symbolic} symbolic nodes for '{actor}'."


def handle_get_stats(_args: dict) -> str:
    result = _get("/stats")
    total   = result.get("total_records", 0)
    actors  = result.get("unique_actors", 0)
    by_type = result.get("by_type", {})
    metered = result.get("metering_enabled", False)
    lines = [
        "HipCortex memory store:",
        f"  Total records:  {total}",
        f"  Unique actors:  {actors}",
        f"  Metering:       {'enabled' if metered else 'open mode'}",
    ]
    if by_type:
        lines.append("  By type:")
        for t, count in sorted(by_type.items()):
            lines.append(f"    {t}: {count}")
    return "\n".join(lines)


def dispatch_tool(name: str, args: dict) -> str:
    handlers = {
        "add_memory":    handle_add_memory,
        "search_memory": handle_search_memory,
        "forget_actor":  handle_forget_actor,
        "get_stats":     handle_get_stats,
    }
    handler = handlers.get(name)
    if handler is None:
        raise ValueError(f"Unknown tool: {name}")
    return handler(args)

# ---------------------------------------------------------------------------
# JSON-RPC 2.0 transport
# ---------------------------------------------------------------------------

def respond(id_: Any, result: Any = None, error: Any = None) -> None:
    msg: dict = {"jsonrpc": "2.0", "id": id_}
    if error is not None:
        msg["error"] = {"code": -32000, "message": str(error)}
    else:
        msg["result"] = result
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def main() -> None:
    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError as e:
            respond(None, error=f"JSON parse error: {e}")
            continue

        method = req.get("method", "")
        id_    = req.get("id")
        params = req.get("params", {})

        if method == "initialize":
            respond(id_, {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "hipcortex", "version": "0.2.0"},
            })
        elif method == "initialized":
            pass  # notification — no response
        elif method == "tools/list":
            respond(id_, {"tools": TOOLS})
        elif method == "tools/call":
            tool_name = params.get("name", "")
            tool_args = params.get("arguments", {})
            try:
                content = dispatch_tool(tool_name, tool_args)
                respond(id_, {"content": [{"type": "text", "text": content}]})
            except requests.RequestException as e:
                respond(id_, error=f"HipCortex server error: {e}")
            except Exception as e:
                respond(id_, error=str(e))
        elif method == "ping":
            respond(id_, {})
        else:
            if id_ is not None:
                respond(id_, error=f"Unknown method: {method}")


if __name__ == "__main__":
    main()

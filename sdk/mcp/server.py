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

# Session harness state (one MCP process lifetime). Soft substrate-first nudge:
# prefer get_live_beliefs / reflect before search_memory. Never hard-blocks.
# Disable: HIPCORTEX_HARNESS_SOFT=0
_live_beliefs_seen = False

_HARNESS_SEARCH_WARN = (
    "[harness] Prefer get_live_beliefs FIRST before search (substrate-first). "
    "Continuing with search..."
)

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
                    "enum": ["Temporal", "Symbolic", "Procedural", "Reflexion", "Perception", "Episodic", "Semantic", "LongTerm", "ShortTerm", "Working", "Reflexive", "Perceptual"],
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
            "Use before starting a task to recall relevant past decisions or context. "
            "Prefer get_live_beliefs first for project-state questions "
            "(substrate-first soft harness may warn if search is used first). "
            "Search remains available."
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
    {
        "name": "search_code",
        "description": (
            "Search the HipCortex code knowledge graph for relevant symbols, functions, classes. "
            "Use this BEFORE reading files — it returns targeted symbol info in ~100 tokens instead of "
            "reading entire files (~10k tokens). Works after running 'hipcortex index' on the codebase."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Symbol name, function, class, or concept to find in the codebase",
                },
                "limit": {
                    "type": "integer",
                    "default": 5,
                    "description": "Max symbols to return",
                },
            },
        },
    },
    {
        "name": "link_memories",
        "description": "Create a directed graph edge between two memory records in the CausalTopoGraph. Use to model causal, temporal, or semantic relationships between memories.",
        "inputSchema": {
            "type": "object",
            "required": ["source_id", "target_id"],
            "properties": {
                "source_id": {"type": "string", "description": "UUID of the source memory record"},
                "target_id": {"type": "string", "description": "UUID of the target memory record"},
                "relation": {"type": "string", "default": "related", "description": "Edge label (e.g. \"caused\", \"follows\", \"related\")"},
            },
        },
    },
    {
        "name": "get_neighbors",
        "description": "Return memory records directly linked to a seed record via the CausalTopoGraph. Use to explore local context around a known memory.",
        "inputSchema": {
            "type": "object",
            "required": ["record_id"],
            "properties": {
                "record_id": {"type": "string", "description": "UUID of the seed memory record"},
                "limit": {"type": "integer", "default": 10, "description": "Max neighbors to return"},
            },
        },
    },
    {
        "name": "search_related",
        "description": "PPR-ranked related memory search. Uses Personalized PageRank (alpha=0.85, 20 rounds) to surface the most contextually relevant memories seeded from a given record. Call after search_memory to expand context graph-first.",
        "inputSchema": {
            "type": "object",
            "required": ["seed_id"],
            "properties": {
                "seed_id": {"type": "string", "description": "UUID of the seed memory record"},
                "limit": {"type": "integer", "default": 10, "description": "Max results to return"},
            },
        },
    },
    {
        "name": "delete_memory",
        "description": "Delete a single memory record by UUID. Use for targeted removal without forgetting an entire actor.",
        "inputSchema": {
            "type": "object",
            "required": ["record_id"],
            "properties": {
                "record_id": {"type": "string", "description": "UUID of the memory record to delete"},
            },
        },
    },
    {
        "name": "get_live_beliefs",
        "description": (
            "Get the latest unique beliefs per actor+action pair. "
            "Returns the most recent value for each fact the system has observed. "
            "Per Claude Agent Harness: call this FIRST before any project-state question."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "actor": {"type": "string", "description": "Filter to a specific actor (optional)"},
                "limit": {"type": "integer", "default": 20, "description": "Max beliefs to return"},
            },
        },
    },
    {
        "name": "purge_expired",
        "description": "Trigger purge of all TTL-expired memory records from the store.",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "reflect",
        "description": (
            "Substrate-first CoT / Bayesian hypothesis sampling via AureusBridge. "
            "POST /memory/reflect — search memory context, sample hypotheses with confidence. "
            "Use for uncertain architectural decisions or counterfactual reasoning before final answer."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to reflect on (e.g. 'Postgres vs RocksDB for session store')",
                },
            },
        },
    },
    {
        "name": "predict",
        "description": (
            "World-model single-step prediction P(s'|s,a). "
            "POST /worldmodel/predict with state + action; returns next-state probabilities + entropy."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["state", "action"],
            "properties": {
                "state": {
                    "type": "string",
                    "description": "Current world-model state label",
                },
                "action": {
                    "type": "string",
                    "description": "Action applied from that state",
                },
            },
        },
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


def handle_search_code(args: dict) -> str:
    query = args.get("query", "")
    limit = args.get("limit", 5)
    try:
        resp = requests.get(
            f"{HIPCORTEX_URL}/graph/search",
            params={"q": query, "limit": limit},
            headers=_headers(),
            timeout=TIMEOUT,
        )
        resp.raise_for_status()
        data = resp.json()
        nodes = data.get("nodes", [])
        if not nodes:
            return (
                f"No code symbols found for '{query}'. "
                "Run 'hipcortex index .' to index the codebase first."
            )
        lines = []
        for n in nodes:
            props = n.get("properties", {})
            label = n.get("label", "?")
            line = props.get("line", "?")
            sig = props.get("signature", props.get("name", label))
            fpath = props.get("file", "")
            lines.append(f"• {sig}  [{fpath}:{line}]")
        return f"Code symbols matching '{query}':\n" + "\n".join(lines)
    except Exception as e:
        return f"Error searching code graph: {e}. Run 'hipcortex index .' first."


def handle_link_memories(args: dict) -> str:
    body = {
        "source_id": args["source_id"],
        "target_id": args["target_id"],
        "relation": args.get("relation", "related"),
    }
    result = _post("/memory/link", body)
    ok = result.get("success", False)
    rel = result.get("relation", "related")
    return (
        f"✓ Link created: {args['source_id']} --[{rel}]--> {args['target_id']}"
        if ok else f"✗ Link failed: {result}"
    )


def handle_get_neighbors(args: dict) -> str:
    record_id = args["record_id"]
    limit = args.get("limit", 10)
    qs = urllib.parse.urlencode({"limit": limit})
    result = _get(f"/memory/neighbors/{record_id}?{qs}")
    records = result.get("records", [])
    if not records:
        return f"No neighbors found for record {record_id}."
    lines = [
        f"• [{r.get('action', '?')}] {r.get('target', '')} (actor: {r.get('actor', '?')}, id: {r.get('id', '?')})"
        for r in records
    ]
    return f"{len(lines)} neighbor(s) of {record_id}:\n" + "\n".join(lines)


def handle_search_related(args: dict) -> str:
    seed_id = args["seed_id"]
    limit = args.get("limit", 10)
    qs = urllib.parse.urlencode({"seed_id": seed_id, "limit": limit})
    result = _get(f"/memory/search/related?{qs}")
    results = result.get("results", [])
    if not results:
        return f"No related memories found for seed {seed_id}. Ensure the record is linked to others."
    lines = [
        f"• [{r.get('record', {}).get('action', '?')}] {r.get('record', {}).get('target', '')} "
        f"(score: {r.get('score', 0):.3f})"
        for r in results
    ]
    return f"{len(lines)} PPR-related result(s):\n" + "\n".join(lines)


def handle_delete_memory(args: dict) -> str:
    record_id = args["record_id"]
    result = _delete(f"/memory/{record_id}")
    ok = result.get("success", False)
    return (
        f"✓ Deleted record {record_id}."
        if ok else f"✗ Delete failed: {result}"
    )


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
    result = _post("/memory/consolidate", {"dry_run": False})
    deleted = result.get("deleted", 0)
    return f"✓ Purge completed. Deleted {deleted} expired records."


def handle_reflect(args: dict) -> str:
    query = args["query"]
    result = _post("/memory/reflect", {"query": query})
    if "error" in result and "hypothesis" not in result:
        return f"✗ Reflect failed: {result.get('error')}"
    hyp = result.get("hypothesis", "")
    conf = result.get("confidence", 0)
    evidence = result.get("evidence", [])
    loops = result.get("loops_run", 0)
    llm = result.get("llm_available", False)
    fallback = result.get("is_fallback", False)
    lines = [
        f"Reflect on: {query}",
        f"  Hypothesis:  {hyp}",
        f"  Confidence:  {conf}",
        f"  Loops run:   {loops}",
        f"  LLM:         {'available' if llm else 'unavailable'}",
        f"  Fallback:    {fallback}",
    ]
    if evidence:
        lines.append("  Evidence:")
        for e in evidence if isinstance(evidence, list) else [evidence]:
            lines.append(f"    • {e}")
    return "\n".join(lines)


def handle_predict(args: dict) -> str:
    state = args["state"]
    action = args["action"]
    result = _post("/worldmodel/predict", {"state": state, "action": action})
    if "error" in result and "probabilities" not in result:
        return f"✗ Predict failed: {result.get('error')}"
    probs = result.get("probabilities", {})
    entropy = result.get("entropy", 0)
    obs = result.get("observation_count", 0)
    from_state = result.get("from_state", state)
    lines = [
        f"Predict P(s'|{from_state}, {result.get('action', action)}):",
        f"  Entropy:            {entropy}",
        f"  Observation count:  {obs}",
    ]
    if probs:
        lines.append("  Next-state probabilities:")
        if isinstance(probs, dict):
            for s, p in sorted(probs.items(), key=lambda x: -float(x[1]) if x[1] is not None else 0):
                lines.append(f"    • {s}: {p}")
        else:
            lines.append(f"    {probs}")
    else:
        lines.append("  (no probability mass — observe transitions first)")
    return "\n".join(lines)


def dispatch_tool(name: str, args: dict) -> str:
    global _live_beliefs_seen
    handlers = {
        "add_memory":       handle_add_memory,
        "search_memory":    handle_search_memory,
        "forget_actor":     handle_forget_actor,
        "get_stats":        handle_get_stats,
        "search_code":      handle_search_code,
        "link_memories":    handle_link_memories,
        "get_neighbors":    handle_get_neighbors,
        "search_related":   handle_search_related,
        "delete_memory":    handle_delete_memory,
        "get_live_beliefs": handle_get_live_beliefs,
        "purge_expired":    handle_purge_expired,
        "reflect":          handle_reflect,
        "predict":          handle_predict,
    }
    handler = handlers.get(name)
    if handler is None:
        raise ValueError(f"Unknown tool: {name}")
    result = handler(args)
    if name in ("get_live_beliefs", "reflect"):
        _live_beliefs_seen = True
    elif name == "search_memory":
        soft_on = os.getenv("HIPCORTEX_HARNESS_SOFT", "1") != "0"
        if soft_on and not _live_beliefs_seen:
            result = _HARNESS_SEARCH_WARN + "\n" + result
    return result

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
                "serverInfo": {"name": "hipcortex", "version": "0.5.0"},
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

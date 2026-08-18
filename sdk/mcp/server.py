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
        "name": "graph_ppr",
        "description": (
            "Personalized PageRank over the live CausalTopoGraph substrate. "
            "Use for topology-first exploration of symbolic graph nodes (not only memory UUIDs). "
            "Prefer after link_memories or deconstruct_hypothesis."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "seed": {"type": "string", "description": "Optional seed node symbolic_id"},
                "limit": {"type": "integer", "default": 10},
            },
        },
    },
    {
        "name": "deconstruct_hypothesis",
        "description": (
            "Parse free text into candidate causal nodes/edges (rule-based; optional llm_json). "
            "Use before applying structure to the topo graph via apply_hypothesis."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {"type": "string", "description": "Hypothesis text, e.g. 'rain causes floods'"},
                "llm_json": {
                    "type": "string",
                    "description": "Optional LLM JSON: {\"edges\":[{\"from\",\"to\",\"relation\"?}]}",
                },
                "apply": {
                    "type": "boolean",
                    "default": False,
                    "description": "If true, also apply edges to live topo graph (POST /topo/apply-hyp)",
                },
            },
        },
    },
    {
        "name": "check_topo_edge",
        "description": (
            "Check whether adding a causal edge from→to would contradict the CausalTopoGraph "
            "(cycle or high-strength reverse)."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["from", "to"],
            "properties": {
                "from": {"type": "string"},
                "to": {"type": "string"},
            },
        },
    },
    {
        "name": "rollout",
        "description": (
            "World-model multi-step rollout. Default Dirichlet MAP after observe; "
            "mode=mcts uses goal-shaped UCB1 search. Requires prior worldmodel observations "
            "or mode=mcts with observed transitions."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["initial_state"],
            "properties": {
                "initial_state": {"type": "string"},
                "actions": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Required unless mode=mcts",
                },
                "mode": {
                    "type": "string",
                    "enum": ["dirichlet", "mcts", "ensemble"],
                    "default": "dirichlet",
                },
                "goal_state": {"type": "string", "description": "Goal for MCTS reward shaping"},
                "iterations": {"type": "integer", "default": 50},
                "max_depth": {"type": "integer", "default": 3},
            },
        },
    },
    {
        "name": "can_execute",
        "description": "Query SelfModel gate: should the agent execute this operation?",
        "inputSchema": {
            "type": "object",
            "required": ["operation"],
            "properties": {
                "operation": {
                    "type": "string",
                    "description": "Operation name (e.g. add_memory, rollout, search_memory)",
                },
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
                "min_conf": {"type": "number", "default": 0.0, "description": "Minimum confidence threshold [0.0, 1.0]"},
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
    {
        "name": "compute_state_diff",
        "description": (
            "Compute a causal StateDiff over a tx-log range [from_tx, to_tx]. "
            "Returns MemoryDelta (added/archived/updated counts), WorldModelDelta, "
            "and CausalAttributionPaths. Range cap: 10,000 entries."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["from_tx", "to_tx"],
            "properties": {
                "from_tx": {"type": "integer", "description": "Start tx id (inclusive)"},
                "to_tx":   {"type": "integer", "description": "End tx id (inclusive)"},
            },
        },
    },
    {
        "name": "simulate_rollout",
        "description": (
            "Simulate a k-step (k≤5) world-model rollout from an initial state "
            "over a sequence of actions using Dirichlet-MAP transitions. "
            "Returns predicted_state, mode, and uncertainty. "
            "Use before committing to a multi-step action plan."
        ),
        "inputSchema": {
            "type": "object",
            "required": ["initial_state", "actions"],
            "properties": {
                "initial_state": {"type": "string", "description": "Starting state label"},
                "actions": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Action sequence (max 5 steps)",
                },
                "max_depth": {"type": "integer", "description": "Override max depth (≤5, default 5)"},
            },
        },
    },
    {
        "name": "get_system_health",
        "description": (
            "Get SelfModel health + CalibrationTracker metrics: overall health score, "
            "calibration_score, prediction_error_ewma, consolidation_pressure, "
            "epistemic_entropy, and whether the system is healthy."
        ),
        "inputSchema": {"type": "object", "properties": {}},
    },
    # Phase 5: Agent Surfaces
    {
        "name": "cognitive_transact",
        "description": "Apply a CognitiveDelta to the cognitive substrate. Supports AddMemory, Consolidate, ForgetActor, ArchiveRecord.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "delta": {"type": "object", "description": "CognitiveDelta (must include 'type' field)"},
                "actor": {"type": "string", "description": "Actor performing the transaction"},
            },
            "required": ["delta", "actor"],
        },
    },
    {
        "name": "cognitive_diff",
        "description": "Get causal StateDiff between two tx-log cursors.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "from_tx": {"type": "integer"},
                "to_tx": {"type": "integer"},
            },
            "required": ["from_tx", "to_tx"],
        },
    },
    {
        "name": "self_health",
        "description": "GET /v1/self/health — SelfModel health metrics with healthy boolean.",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "cognitive_snapshot",
        "description": "GET /v1/cognitive/snapshot — full cognitive state snapshot for an actor.",
        "inputSchema": {
            "type": "object",
            "properties": {"actor": {"type": "string", "default": ""}},
        },
    },
    {
        "name": "fork_create",
        "description": "POST /v1/fork — create a SimulationFork (CoW snapshot of cognitive state).",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "fork_step",
        "description": "POST /v1/fork/{fork_id}/step — advance a fork by one action.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "fork_id": {"type": "string"},
                "action": {"type": "string"},
            },
            "required": ["fork_id", "action"],
        },
    },
    {
        "name": "fork_snapshot",
        "description": "GET /v1/fork/{fork_id}/snapshot — read fork cognitive snapshot.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "fork_id": {"type": "string"},
                "actor": {"type": "string", "default": ""},
            },
            "required": ["fork_id"],
        },
    },
    {
        "name": "fork_delete",
        "description": "DELETE /v1/fork/{fork_id} — discard a SimulationFork.",
        "inputSchema": {
            "type": "object",
            "properties": {"fork_id": {"type": "string"}},
            "required": ["fork_id"],
        },
    },
    {
        "name": "fork_rollout",
        "description": "POST /v1/fork/{fork_id}/rollout — k-step (k≤5) hybrid Kalman rollout.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "fork_id": {"type": "string"},
                "actions": {"type": "array", "items": {"type": "string"}},
                "sigma2_max": {"type": "number", "default": 0.25},
            },
            "required": ["fork_id", "actions"],
        },
    },
    {
        "name": "consolidate_memory",
        "description": "Mine recurring causal subgraphs and induce Skill+Belief records via AutoConsolidate.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "min_frequency": {"type": "integer", "default": 3, "description": "Minimum causal path frequency to qualify for induction."},
            },
            "required": [],
        },
    },
    {
        "name": "forget_actor",
        "description": "GDPR hard-delete all records for an actor via ForgetActor delta.",
        "inputSchema": {
            "type": "object",
            "properties": {"actor_id": {"type": "string"}},
            "required": ["actor_id"],
        },
    },
    {
        "name": "archive_record",
        "description": "Archive or delete a single record via ArchiveRecord delta.",
        "inputSchema": {
            "type": "object",
            "properties": {"id": {"type": "string"}},
            "required": ["id"],
        },
    },
    {
        "name": "workspace_open",
        "description": "Open a new workspace (Private or Shared) for scoped multi-agent cognition.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "workspace_id": {"type": "string", "description": "UUID for the workspace."},
                "mode": {"type": "string", "enum": ["Private", "Shared"], "default": "Private"},
            },
            "required": ["workspace_id"],
        },
    },
    {
        "name": "workspace_merge",
        "description": "Merge one Shared workspace into another via OR-Set CRDT convergence.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "from_id": {"type": "string", "description": "Source workspace UUID."},
                "into_id": {"type": "string", "description": "Target workspace UUID."},
            },
            "required": ["from_id", "into_id"],
        },
    },
    {
        "name": "retract_belief",
        "description": "Retract a belief record, triggering JTMS dependency propagation.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "belief_id": {"type": "string"},
                "reason": {"type": "string", "default": "retracted"},
            },
            "required": ["belief_id"],
        },
    },
    {
        "name": "get_state_export",
        "description": "Export full versioned Cognitive State snapshot (schema_version=0.8.0, tx_cursor, beliefs, goals, world state). Use for agent handover, substrate audit, or cross-session state transfer.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": [],
        },
    },
]

RESOURCES = [
    {
        "uri": "hipcortex://context/relevant",
        "name": "Relevant Memory Context",
        "description": "Top-k memories relevant to the current session, auto-injected.",
        "mimeType": "text/plain",
    },
    {
        "uri": "hipcortex://beliefs/current",
        "name": "Current Beliefs",
        "description": "Active belief records from HipCortex symbolic store.",
        "mimeType": "application/json",
    },
    {
        "uri": "hipcortex://beliefs/live",
        "name": "Live Beliefs",
        "description": "Active Belief records with confidence + JTMS labels (alias of beliefs/current).",
        "mimeType": "application/json",
    },
    {
        "uri": "hipcortex://context/conversation",
        "name": "Conversation History",
        "description": "Recent temporal memory traces for this session.",
        "mimeType": "text/plain",
    },
    {
        "uri": "hipcortex://state/diff",
        "name": "Recent State Diff",
        "description": "ΔS for last 5 transactions — epistemic value of recent turns.",
        "mimeType": "application/json",
    },
    {
        "uri": "hipcortex://self/health",
        "name": "Self-Model Health",
        "description": "Current calibration score, epistemic entropy, consolidation pressure.",
        "mimeType": "application/json",
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
    if "min_conf" in args:
        params["min_conf"] = args["min_conf"]
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


def handle_simulate_rollout(args: dict) -> str:
    actions = args["actions"]
    effective_depth = min(args.get("max_depth", 5), 5)
    if len(actions) > effective_depth:
        return f"✗ Rollout rejected: {len(actions)} actions exceed max_depth={effective_depth} (k≤5 limit)"
    payload = {
        "initial_state": args["initial_state"],
        "actions": actions,
        "max_depth": effective_depth,
    }
    result = _post("/worldmodel/rollout", payload)
    if "error" in result:
        return f"✗ Rollout failed: {result['error']}"
    uncertainty = result.get("uncertainty")
    unc_str = f"{uncertainty:.3f}" if isinstance(uncertainty, (int, float)) else "?"
    return (
        f"Rollout: {result.get('predicted_state', '?')} "
        f"(mode={result.get('mode', '?')}, uncertainty={unc_str})"
    )


def handle_get_system_health(_args: dict) -> str:
    result = _get("/self/health")
    if "error" in result:
        return f"✗ Health check failed: {result['error']}"
    healthy = result.get("healthy", False)
    overall = result.get("overall", 0.0)
    cal = result.get("calibration_score", None)
    ewma = result.get("prediction_error_ewma", None)
    pressure = result.get("consolidation_pressure", None)
    entropy = result.get("epistemic_entropy", None)
    lines = [f"System healthy: {healthy} | overall={overall:.3f}"]
    if cal is not None:
        lines.append(f"calibration_score={cal:.3f} | prediction_error_ewma={ewma:.3f}")
    if pressure is not None:
        lines.append(f"consolidation_pressure={pressure:.3f} | epistemic_entropy={entropy:.3f}")
    return "\n".join(lines)


def handle_cognitive_transact(args: dict) -> str:
    result = _post("/v1/cognitive/transact", {"delta": args["delta"], "actor": args.get("actor", "mcp")})
    return json.dumps(result)

def handle_cognitive_diff(args: dict) -> str:
    params = f"from_tx={args['from_tx']}&to_tx={args['to_tx']}"
    resp = requests.get(f"{HIPCORTEX_URL}/v1/cognitive/diff?{params}", headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return json.dumps(resp.json())

def handle_self_health(_args: dict) -> str:
    return json.dumps(_get("/v1/self/health"))

def handle_cognitive_snapshot(args: dict) -> str:
    actor = args.get("actor", "")
    resp = requests.get(f"{HIPCORTEX_URL}/v1/cognitive/snapshot", params={"actor": actor}, headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return json.dumps(resp.json())

def handle_fork_create(_args: dict) -> str:
    return json.dumps(_post("/v1/fork", {}))

def handle_fork_step(args: dict) -> str:
    return json.dumps(_post(f"/v1/fork/{args['fork_id']}/step", {"action": args["action"]}))

def handle_fork_snapshot(args: dict) -> str:
    actor = args.get("actor", "")
    resp = requests.get(f"{HIPCORTEX_URL}/v1/fork/{args['fork_id']}/snapshot", params={"actor": actor}, headers=_headers(), timeout=TIMEOUT)
    resp.raise_for_status()
    return json.dumps(resp.json())

def handle_fork_delete(args: dict) -> str:
    return json.dumps(_delete(f"/v1/fork/{args['fork_id']}"))

def handle_fork_rollout(args: dict) -> str:
    return json.dumps(_post(f"/v1/fork/{args['fork_id']}/rollout", {"actions": args["actions"], "sigma2_max": args.get("sigma2_max", 0.25)}))

def handle_p5_consolidate(args: dict) -> str:
    delta = {"type": "AutoConsolidate", "min_frequency": args.get("min_frequency", 3)}
    return json.dumps(_post("/v1/cognitive/transact", {"delta": delta, "actor": "mcp"}))

def handle_get_state_export(_args: dict) -> str:
    data = _get("/v1/state/export")
    if "error" in data:
        return f"State export error: {data['error']}"
    sv = data.get("schema_version", "unknown")
    tx = data.get("tx_cursor", 0)
    beliefs = len(data.get("beliefs", {}).get("beliefs", []))
    goals = len(data.get("goals", []))
    return (
        f"State export OK (schema={sv}, tx={tx}, beliefs={beliefs}, goals={goals})\n"
        + json.dumps(data, indent=2)[:3000]
    )

def handle_forget_actor(args: dict) -> str:
    delta = {"type": "ForgetActor", "actor": args["actor_id"]}
    return json.dumps(_post("/v1/cognitive/transact", {"delta": delta, "actor": "mcp"}))

def handle_archive_record(args: dict) -> str:
    delta = {"type": "ArchiveRecord", "id": args["id"]}
    return json.dumps(_post("/v1/cognitive/transact", {"delta": delta, "actor": "mcp"}))


def handle_workspace_open(args: dict) -> str:
    delta = {"type": "WorkspaceOpen", "id": args["workspace_id"], "mode": args.get("mode", "Private")}
    return json.dumps(_post("/v1/cognitive/transact", {"delta": delta, "actor": "mcp"}))


def handle_workspace_merge(args: dict) -> str:
    delta = {"type": "WorkspaceMerge", "from": args["from_id"], "into": args["into_id"]}
    return json.dumps(_post("/v1/cognitive/transact", {"delta": delta, "actor": "mcp"}))


def handle_retract_belief(args: dict) -> str:
    delta = {"type": "RetractBelief", "id": args["belief_id"], "reason": args.get("reason", "retracted")}
    return json.dumps(_post("/v1/cognitive/transact", {"delta": delta, "actor": "mcp"}))


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


def handle_graph_ppr(args: dict) -> str:
    params: dict = {"limit": args.get("limit", 10)}
    if args.get("seed"):
        params["seed"] = args["seed"]
    qs = urllib.parse.urlencode(params)
    result = _get(f"/topo/ppr?{qs}")
    results = result.get("results", [])
    if not results:
        return f"No PPR results (seed={args.get('seed')}). Link nodes or apply a hypothesis first."
    lines = [f"• {r.get('id')} (score: {r.get('score', 0):.4f})" for r in results]
    return f"Topo PPR ({len(lines)}):\n" + "\n".join(lines)


def handle_deconstruct_hypothesis(args: dict) -> str:
    body: dict = {"text": args["text"]}
    if args.get("llm_json"):
        body["llm_json"] = args["llm_json"]
    apply = bool(args.get("apply", False))
    path = "/topo/apply-hyp" if apply else "/topo/deconstruct"
    result = _post(path, body)
    if not result.get("success", True) and result.get("error"):
        return f"✗ Deconstruct failed: {result.get('error')}"
    edges = result.get("edges") or result.get("hypothesis", {}).get("edges") or []
    nodes = result.get("nodes") or result.get("hypothesis", {}).get("nodes") or []
    lines = [f"Nodes: {', '.join(nodes) if nodes else '(none)'}"]
    if edges:
        lines.append("Edges:")
        for e in edges:
            if isinstance(e, dict):
                lines.append(
                    f"  • {e.get('from')} --[{e.get('relation', '?')}]--> {e.get('to')} "
                    f"(conf={e.get('confidence', '?')})"
                )
            else:
                lines.append(f"  • {e}")
    if apply:
        lines.append(
            f"Applied: nodes+={result.get('added_nodes', 0)} edges+={result.get('added_edges', 0)}"
        )
        if result.get("skipped"):
            lines.append(f"Skipped: {result.get('skipped')}")
    return "\n".join(lines)


def handle_check_topo_edge(args: dict) -> str:
    result = _post("/topo/check-edge", {"from": args["from"], "to": args["to"]})
    if result.get("error") and not result.get("success", True):
        return f"✗ check failed: {result.get('error')}"
    bad = result.get("would_contradict", False)
    report = result.get("report")
    if bad:
        reason = (report or {}).get("reason", "contradiction") if isinstance(report, dict) else "contradiction"
        return f"Would contradict: {args['from']} → {args['to']} ({reason})"
    return f"OK to add: {args['from']} → {args['to']} (no contradiction detected)"


def handle_rollout(args: dict) -> str:
    body: dict = {
        "initial_state": args["initial_state"],
        "mode": args.get("mode", "dirichlet"),
    }
    if "actions" in args:
        body["actions"] = args["actions"]
    if "goal_state" in args:
        body["goal_state"] = args["goal_state"]
    if "iterations" in args:
        body["iterations"] = args["iterations"]
    if "max_depth" in args:
        body["max_depth"] = args["max_depth"]
    result = _post("/worldmodel/rollout", body)
    if result.get("error") and "predicted_state" not in result:
        return f"✗ Rollout failed: {result.get('error')}"
    lines = [
        f"Rollout mode={result.get('mode', body.get('mode'))}",
        f"  initial: {result.get('initial_state', args['initial_state'])}",
        f"  predicted: {result.get('predicted_state', '?')}",
        f"  confidence: {result.get('confidence', '?')}",
    ]
    if result.get("best_action"):
        lines.append(f"  best_action: {result.get('best_action')}")
    if result.get("goal_state"):
        lines.append(f"  goal: {result.get('goal_state')}")
    return "\n".join(lines)


def handle_can_execute(args: dict) -> str:
    op = args["operation"]
    qs = urllib.parse.urlencode({"operation": op})
    result = _get(f"/self/can-execute?{qs}")
    if result.get("error") and "should_execute" not in result:
        return f"✗ can_execute failed: {result.get('error')}"
    ok = result.get("should_execute")
    conf = result.get("confidence", "?")
    rat = result.get("rationale", "")
    return f"can_execute({op}) = {ok} (confidence={conf})\n  {rat}".rstrip()


def handle_compute_state_diff(args: dict) -> str:
    from_tx = int(args.get("from_tx", 0))
    to_tx   = int(args.get("to_tx",   0))
    result = _post("/v1/state/diff", {"from_tx": from_tx, "to_tx": to_tx})
    if "error" in result:
        return f"✗ StateDiff error: {result['error']}"
    md = result.get("memory_delta", {})
    wm = result.get("world_model_delta", {})
    lines = [
        f"StateDiff tx[{from_tx}..{to_tx}]  count={result.get('tx_count', 0)}",
        f"  memory: +{len(md.get('added', []))} archived={len(md.get('archived', []))} "
        f"updated={len(md.get('updated', []))} net={md.get('net_delta', 0)}",
        f"  world_model: obs+{wm.get('observations_added', 0)} "
        f"dist_upd={wm.get('distributions_updated', 0)}",
        f"  causal_attributions: {len(result.get('causal_attributions', []))}",
    ]
    return "\n".join(lines)


def handle_consolidate_memory(args: dict) -> str:
    body: dict = {}
    if "min_group_size" in args:
        body["min_group_size"] = int(args["min_group_size"])
    result = _post("/v1/memory/consolidate", body)
    if "error" in result:
        return f"✗ Consolidate error: {result['error']}"
    return (
        f"Consolidated {result.get('groups_consolidated', 0)} groups — "
        f"archived {result.get('records_archived', 0)} records, "
        f"created {result.get('summary_records_created', 0)} summaries"
    )


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
        "graph_ppr":        handle_graph_ppr,
        "deconstruct_hypothesis": handle_deconstruct_hypothesis,
        "check_topo_edge":  handle_check_topo_edge,
        "rollout":          handle_rollout,
        "can_execute":      handle_can_execute,
        "delete_memory":    handle_delete_memory,
        "get_live_beliefs": handle_get_live_beliefs,
        "purge_expired":    handle_purge_expired,
        "reflect":          handle_reflect,
        "predict":              handle_predict,
        "compute_state_diff":   handle_compute_state_diff,
        "simulate_rollout":     handle_simulate_rollout,
        "get_system_health":    handle_get_system_health,
        "cognitive_transact":   handle_cognitive_transact,
        "cognitive_diff":       handle_cognitive_diff,
        "self_health":          handle_self_health,
        "cognitive_snapshot":   handle_cognitive_snapshot,
        "fork_create":          handle_fork_create,
        "fork_step":            handle_fork_step,
        "fork_snapshot":        handle_fork_snapshot,
        "fork_delete":          handle_fork_delete,
        "fork_rollout":         handle_fork_rollout,
        "consolidate_memory":   handle_p5_consolidate,
        "get_state_export":     handle_get_state_export,
        "forget_actor":         handle_forget_actor,
        "archive_record":       handle_archive_record,
        "workspace_open":       handle_workspace_open,
        "workspace_merge":      handle_workspace_merge,
        "retract_belief":       handle_retract_belief,
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


def handle_resources_list() -> dict:
    return {"resources": RESOURCES}


def handle_resource_read(uri: str) -> dict:
    try:
        if uri == "hipcortex://context/relevant":
            data = _get("/memory/search?q=context&limit=5")
            lines = []
            for item in (data if isinstance(data, list) else []):
                actor = item.get("actor", "")
                action = item.get("action", "")
                target = item.get("target", "")
                lines.append(f"[{actor}] {action} → {target}")
            text = "\n".join(lines) if lines else "(no memories yet)"
            return {"contents": [{"uri": uri, "mimeType": "text/plain", "text": text}]}
        elif uri in ("hipcortex://beliefs/current", "hipcortex://beliefs/live"):
            data = _get("/v1/beliefs")
            return {
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": json.dumps(data if isinstance(data, list) else data),
                    }
                ]
            }
        elif uri == "hipcortex://state/diff":
            # Get current tx, compute diff over last 5 transactions
            try:
                snap = _get("/v1/cognitive/snapshot")
                head = snap.get("tx_cursor", 0)
                from_tx = max(0, head - 5)
                data = _get(f"/v1/state/diff?from_tx={from_tx}&to_tx={head}")
            except Exception:
                data = {}
            return {
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": json.dumps(data),
                    }
                ]
            }
        elif uri == "hipcortex://self/health":
            data = _get("/v1/self/health")
            return {
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": json.dumps(data),
                    }
                ]
            }
        elif uri == "hipcortex://context/conversation":
            actor = os.environ.get("HIPCORTEX_ACTOR", "mcp-session")
            data = _get(f"/memory/query?actor={actor}&limit=20")
            lines = []
            for item in (data if isinstance(data, list) else []):
                lines.append(f"{item.get('action', '')} → {item.get('target', '')}")
            text = "\n".join(lines) if lines else "(no history yet)"
            return {"contents": [{"uri": uri, "mimeType": "text/plain", "text": text}]}
        else:
            return {"contents": [{"uri": uri, "mimeType": "text/plain", "text": ""}]}
    except Exception:
        return {"contents": [{"uri": uri, "mimeType": "text/plain", "text": ""}]}


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
                "capabilities": {"tools": {}, "resources": {}},
                "serverInfo": {"name": "hipcortex", "version": "0.8.0"},
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
        elif method == "resources/list":
            respond(id_, handle_resources_list())
        elif method == "resources/read":
            respond(id_, handle_resource_read(params.get("uri", "")))
        elif method == "ping":
            respond(id_, {})
        else:
            if id_ is not None:
                respond(id_, error=f"Unknown method: {method}")


if __name__ == "__main__":
    main()

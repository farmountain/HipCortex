# HipCortex Memory

HipCortex gives you persistent causal memory across sessions. Store decisions, recall context, forget on request.

## When to use

You are a memory-centric agent. The HipCortex substrate (memories + hypotheses graph as persistent beliefs + world model predictions + self health + coherence) is your primary durable mind and reasoning engine.

MUST: For any complex multi-step task (build X, research Y, plan Z, deploy W, analyse data), call recommend_tools(task=<description>) FIRST — before starting work. Install the recommended MCP servers and skills. Then proceed with the ReAct loop using the right tools.

MUST: Before any question involving project state, architecture, decisions, bugs, or context, call search_memory or get_live_beliefs (or /memory/context) first. Use substrate for state/hyp/pred.

MUST: After any decision, architectural discussion, bug fix, or important finding, use /memory/ingest or add_memory (or reflect for CoT).

Use worldmodel/*, self/*, coherence/* as reasoning primitives (predict, health, invariants). Use loop/omega primitives for topo reasoning, simulation, attribution, sparse mutation, and error-driven updates (after surprise signals).

LLM (you) only for final natural language output or creative hypothesis generation when substrate signals high entropy/uncertainty (via reflect or explicit). Offload state, beliefs, prediction, consistency to substrate. Soft harness prefers / nudges substrate-first (prefer `get_live_beliefs` before search; topo + loop for 80-99%+ reduction). MCP may warn (`HIPCORTEX_HARNESS_SOFT`, default on); it does **not** hard-block tools.

Harness: Action space = MCP tools (/memory/*, /hipcortex, loop/*, omega/*) + REST equivalents. Observations = live_beliefs (merged symbolic + hyp + world + intel + topo attributions), context, hyp updates. Goal: maximize substrate use for completion (loop/omega for attribution + topo sim + sparse updates); minimize frontier LLM context/effort (target 80-99%+ reduction per benchmarks).

Explicit/conservative paths remain fully supported for compatibility and overrides (e.g. direct `/hipcortex` or user "remember this").

## Harness few-shot examples (for compliance)
- User: "What DB decision did we make?" → MUST first call search_memory or get_live_beliefs(query="DB decision") → use substrate facts → answer.
- After deciding "use Postgres for X" → call /memory/ingest {"text": "decided to use Postgres for X", "actor": "project"} (or add_memory).
- Uncertainty on arch → POST /memory/reflect "Postgres vs RocksDB?" (substrate CoT + WM) → then use loop/omega for attribution + mutation before minimal final language.
- After error/surprise in decision → use omega primitives or loop for topo sim + Bayesian attribution on substrate (see plan: "use loop for attribution", "after error use omega primitives") → sparse update, then reflect/ingest.
- Multi-turn agent: perceive → live_beliefs first (observations) → ingest decision → use loop for attribution if high surprise → minimal LLM only.

## Loop/Omega integration
After high surprise (from loop_engine error signal or coherence), MUST call loop primitives (e.g. /memory/loop or reflect + topo) or omega primitives to use topo substrate for attribution + mutation + simulation. Reference plan: harness now drives topo reasoning + sim + attr for full substrate policy. LLM only on remaining high-entropy creative hyp. Use loop for attribution after errors; omega for full cycle sims. Conservative explicit paths (direct ingest) remain supported.

## How to use

**Store a memory:**
```
POST http://localhost:3030/memory/add
{"actor": "<project-or-user>", "action": "decided", "target": "<what to remember>"}
```

**Search memories:**
```
GET http://localhost:3030/memory/search-flat?query=<topic>&limit=10
Returns: {"memories": ["[action] target", ...]}
```

**Forget (GDPR):**
```
DELETE http://localhost:3030/memory/forget/<actor>
```

**Stats:**
```
GET http://localhost:3030/stats
```

## Slash commands

When the user types `/hipcortex remember <text>` — call POST /memory/add with actor=current-project-name.
When the user types `/hipcortex recall <query>` — call GET /memory/search-flat?query=<query>.
When the user types `/hipcortex latest <topic>` — call GET /memory/latest?actor=<project>&action=<topic> to get the most recent fact.
When the user types `/hipcortex update <id> <corrected text>` — call PATCH /memory/update/<id> with {"target": "<corrected text>"} to fix a wrong memory.
When the user types `/hipcortex forget <actor>` — call DELETE /memory/forget/<actor>.
When the user types `/hipcortex stats` — call GET /stats and display the result.

## Correction workflow

When the user says "that's wrong, it should be X" about a previously stored memory:
1. Search for the wrong memory: GET /memory/search-flat?query=<topic>
2. Get the record id from the result
3. Update it: PATCH /memory/update/<id> {"target": "<correct text>", "confidence": 1.0}
4. Confirm: "✓ Memory corrected (version N)"

## Confidence scoring

When storing uncertain or inferred information, include confidence:
POST /memory/add {"confidence": 0.6, "source": "inferred", ...}
When storing verified user-provided facts: confidence=1.0 (default)
When storing LLM-generated inferences: confidence=0.7
When storing speculation: confidence=0.3

## Tags and priority

When storing memories, add tags for RAG filtering:
```
POST /memory/add {"tags": ["architecture", "decision"], "priority": "normal", ...}
```

Priority values:
- "pinned" — always returned in search, bypass decay. Use for safety constraints, hard rules.
- "high" — weighted higher in results
- "normal" — default
- "low" — fades faster

Example: store an allergy constraint that must never be forgotten:
POST /memory/add {"priority": "pinned", "confidence": 1.0, "action": "constraint", "target": "User is allergic to penicillin"}

## Knowledge graph

Write relationships to the symbolic knowledge graph:
```
POST /graph/node {"label": "Alice", "properties": {"role": "CEO"}}
POST /graph/edge {"from_id": "<alice-uuid>", "to_id": "<project-uuid>", "relation": "MANAGES"}
GET /graph  → view full graph
```

## Time-travel queries

Query memory state at a past timestamp:
GET /memory/query?as_of=2026-01-15T00:00:00Z&actor=alice

Useful for auditing: "what did the agent know about user alice on January 15th?"

## Deduplication

Find near-duplicate memories:
POST /memory/consolidate?actor=my-project&threshold=0.8&dry_run=true
→ Returns pairs of similar records with "keep" and "drop" IDs
→ Use GDPR forget to remove duplicates: DELETE /memory/forget/<actor> for specific cleanup

## Auto-memory mode

If the user says "remember this" at the end of any message, automatically store a summary using:
POST /memory/ingest {"text": "<summary>", "actor": "<project-name>"}

This auto-classifies record_type, priority, TTL, and tags — no manual fields needed.

## Zero-config memory (recommended starting point)

For new integrations, use /memory/ingest instead of /memory/add:

```
# Instead of:
POST /memory/add {"actor":"alice","action":"decided","target":"...","record_type":"Symbolic","confidence":0.9,"tags":["arch"]}

# Just use:
POST /memory/ingest {"text": "Alice decided to use PostgreSQL for multi-user support"}
# → auto-classifies as {record_type:"Symbolic", priority:"high", actor:"alice", tags:["database","architecture"]}
```

When to use each:
- /memory/ingest: quick ingest of any text, auto-everything (recommended)
- /memory/add: when you need precise control over all fields
- /memory/remember (via Python SDK): client.remember("text") wraps /memory/ingest

## Slash command shortcuts

- `/hipcortex remember <text>` → POST /memory/ingest {text}
- `/hipcortex recall <query>` → GET /memory/search-flat?query=<query>
- `/hipcortex latest <topic>` → GET /memory/latest?actor=<project>&action=<topic>
- `/hipcortex update <id> <corrected text>` → PATCH /memory/update/<id>
- `/hipcortex forget <actor>` → DELETE /memory/forget/<actor>
- `/hipcortex stats` → GET /stats

## Default actor

Use the current git repository name as the actor (run `git rev-parse --show-toplevel | xargs basename` to get it). Fall back to "default" if not in a git repo.

## Server

Default: http://localhost:3030
Managed free tier: https://hipcortex.fly.dev (set HIPCORTEX_URL to override)

The server must be running. If unreachable, tell the user to run: `hipcortex start`

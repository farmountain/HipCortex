# Claude Agent Harness — HipCortex v1.3.0

HipCortex turns Claude Code (or any MCP-capable LLM) into a **substrate-first autonomous agent**: the cognitive state (memories, beliefs, world model, coherence) is the primary durable mind; the LLM is a thin language surface used only for final output or high-entropy creative hypotheses.

## Architecture

```
User instruction
       │
       ▼
Claude Code (LLM) — language surface only
       │
       ├─ MUST: get_live_beliefs() ──────────┐
       │                                      │
       ├─ ReAct loop (goal-driven):           │
       │   observe → add_memory(Temporal)     │
       │   reflect → POST /memory/reflect     │
       │   act → tools + file writes          │ HipCortex Substrate
       │   check success_factors              │ (primary cognitive mind)
       │          │                           │
       │   LoopEngine.run_omega_loop() ───────┤
       │   (on surprise/gap)                  │
       │                                      │
       └─ minimal LLM output ◄────────────────┘
```

## Harness Installation

```bash
# Standard (conservative) — explicit invocation
hipcortex install

# Proactive (substrate-first) — MUST substrate before every response
hipcortex install --mode proactive

# Per-actor (multi-agent) — scoped actor namespace
hipcortex install --mode proactive --actor my-agent
```

Sets `HIPCORTEX_HARNESS_SOFT` mode and installs the proactive SKILL template into `~/.claude/CLAUDE.md`.

## Action Space

All 45 MCP tools + REST equivalents. Key primitives:

| Action | MCP Tool | REST |
|--------|----------|------|
| Observe substrate state | `get_live_beliefs` | `GET /memory/live_beliefs` |
| Store decision/observation | `add_memory` | `POST /memory/add` |
| Substrate chain-of-thought | `reflect` | `POST /memory/reflect` |
| World-model prediction | `predict` | `POST /worldmodel/predict` |
| Goal-driven ReAct loop | `cognitive_transact` | `POST /v1/cognitive/transact` |
| Omega gap + attribution cycle | `run_omega_loop` | `POST /v1/loop/omega` |
| Execution gate | `can_execute` | `POST /worldmodel/can-execute` |
| Causal attribution | `causal_credit_assign` | `POST /causal/credit-assign` |

## Observations (Substrate Outputs)

`GET /memory/live_beliefs?actor=<actor>&limit=<n>` returns a merged surface:

```json
{
  "symbolic_facts": { "nodes": [...], "edges": [...] },
  "code_facts": [...],
  "current_hypotheses": [...],
  "world_state": { "entities": [...], "transitions_sample": [...] },
  "intel": {
    "self_health": { "calibration_score": 0.87, "health_score": 0.92 },
    "coherence": { "invariants_ok": true, "last_check_ms": 12 },
    "pinned_memories": [...]
  }
}
```

Use this as the **first call** before any task. It replaces fetching context, beliefs, predictions, and health separately.

## Substrate Chain-of-Thought (trigger_reflexion)

`POST /memory/reflect` invokes `AureusBridge.trigger_reflexion` — runs a hypothesis update with:
- World-model prior on the current state
- HypothesesGraph weighted by evidence
- Coherence checker invariants

Use this instead of asking the LLM to reason over raw context. Substrate reasoning is faster and cheaper.

```bash
curl -X POST http://localhost:3030/memory/reflect \
  -H "Content-Type: application/json" \
  -d '{"actor": "my-agent", "query": "Which approach satisfies the auth requirement?"}'
```

Returns: ranked hypotheses with confidence scores. LLM uses the top result as its reasoning basis.

## Memory-Centric ReAct Loop

```
for each iteration until success_factors satisfied:
    OBSERVE:  live_beliefs = GET /memory/live_beliefs
    REFLECT:  POST /memory/reflect (substrate CoT)
    ACT:      execute tool / write file / call API
    STORE:    POST /memory/add (Temporal observation)
              → auto-fires: WMUpdater, BeliefInvalidator, EmergenceDetector
    GATE:     POST /worldmodel/can-execute (before irreversible actions)
    OMEGA:    POST /v1/loop/omega (when surprise signal > threshold)
```

`src/modules/loop_engine.rs:ReactEngine::run` implements this loop. Wire it via `POST /v1/cognitive/transact` with `CognitiveDelta::AddGoal(goal_payload)`.

## Goal-Driven Acceptance Criteria

```python
goal = {
    "actor": "claude-code",
    "description": "Build full-stack Facebook replica",
    "success_factors": [
        "auth_system_tested",
        "news_feed_working",
        "user_profiles_working",
        "deployment_manifests_written",
    ],
    "max_react_iterations": 50,
}
```

`ReactEngine` terminates when all `success_factors` appear in stored Temporal records linked to the goal, or when `max_react_iterations` is exhausted. Status: `Pending → InProgress → Succeeded | Failed`.

## Worked Example: Autonomous Full-Stack Build

```
Task: "Build a full-stack Facebook replica"

Iteration 1 (OBSERVE):
  GET /memory/live_beliefs?actor=claude-code
  → no prior context → scaffold requirements as Beliefs

Iteration 2-5 (ACT):
  Write auth module → store Temporal("scaffolded auth")
  G1a: WMUpdater observes (auth_module, scaffold) → world model learns
  G1b: BeliefInvalidator checks for contradicting architecture beliefs
  G1c: EmergenceDetector (every 10th write) scans patterns → new Skill belief

Iteration 6 (REFLECT on test failure):
  Tests fail for auth → POST /memory/reflect → substrate CoT
  Omega: POST /v1/loop/omega → PageRank localizes failure to JWT config
  Attribution: causal_credit_assign → broken structural equation identified
  Sparse update: only JWT config belief mutated

...continues until all success_factors satisfied
```

## Worked Example: Kyoto Trip Planning with Playwright

```
Task: "7-day Kyoto sakura trip from Singapore with real costs"

Goal success_factors:
  ["hotel_costs_7nights", "flights_SIN_KIX", "transport_kyoto", "total_budget_SGD"]

ReAct loop:
  OBSERVE: live_beliefs → no prior trip data
  ACT:     Playwright → hotel.com, trip.com, booking.com
  STORE:   Temporal("hotel_ibis_kyoto_7nights_SGD_280", metadata={action:"scrape_hotel"})
           → G1a: WMUpdater learns (kyoto_hotel, scrape) → price distribution
           → G2a: entropy in prices → calibration score updated
  REFLECT: /memory/reflect → which hotels satisfy ≤ SGD 350/night constraint?
  ACT:     Playwright → flights on skyscanner/google flights
  STORE:   Temporal("SQ_SIN_KIX_return_SGD_520")
  ...
  OMEGA:   budget overrun detected → omega loop attributes to hotel cost
           → sparse mutation: hotel budget belief revised
  FINAL:   substrate holds full itinerary; LLM formats as markdown output
```

## Multi-Agent Actor Scoping

Each agent gets its own actor namespace in the shared substrate:

```bash
hipcortex install --actor frontend-agent   # Claude working on UI
hipcortex install --actor backend-agent    # Claude working on API
hipcortex install --actor orchestrator     # Top-level planner
```

Actor scoping in queries:
```
GET /memory/live_beliefs?actor=frontend-agent   → only frontend context
GET /memory/search?query=auth&actor=backend-agent
DELETE /memory/forget/frontend-agent            → GDPR wipe per agent
```

All agents share the global symbolic graph (SymbolicStore) and world model (WorldModelEnhanced), but Temporal observations and Goals are actor-scoped.

## Harness Compliance Targets

| Mode | Expected substrate calls/turn | LLM token reduction |
|------|-----------------------------|---------------------|
| Conservative | 1-2 (explicit only) | 30-50% |
| Proactive | 3-6 (get_live_beliefs + reflect + predict) | 70-99% |
| Proactive + omega | 6-10 (+ loop on surprise) | 80-99% |

Token reduction source: substrate carries state/beliefs/predictions so LLM prompt needs only the question + substrate answer, not full conversation history.

## Configuration

| Env var | Default | Effect |
|---------|---------|--------|
| `HIPCORTEX_AGENT_DEFAULTS` | off | Wire PerceptionSession for all AgentMessage paths; enable auto-ingest |
| `HIPCORTEX_HARNESS_SOFT` | on | MCP warns on non-substrate-first calls; does not hard-block |
| `HIPCORTEX_ACTOR` | `mcp-session` | Default actor for MCP server session |
| `HIPCORTEX_URL` | `http://localhost:3030` | Server URL |

## Related

- `src/modules/loop_engine.rs` — `ReactEngine` + `LoopEngine.run_omega_loop()`
- `src/web_server.rs:handle_memory_live_beliefs` — unified beliefs surface
- `src/modules/integration_layer.rs:trigger_reflexion` — substrate CoT
- `sdk/python/hipcortex/install/SKILL.md` — installed harness policy
- `sdk/mcp/server.py` — 45 MCP tools
- `docs/usage.md` — CLI and API reference
- `docs/integration.md` — framework adapters (LangChain, CrewAI, AutoGen)

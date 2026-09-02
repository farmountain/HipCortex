# Claude Agent Harness — HipCortex v1.4.0

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

Background (SubstrateDaemon):
  Observe → Reflect → Plan → CriticVeto → Predict → Act → Update → ExitCheck
  (runs every interval_secs; independent of LLM turns)
```

## Tool Discovery (start here for complex tasks)

Before starting any complex multi-step task, call `recommend_tools` to get the right MCP servers, skills, and setup commands for your use case:

```bash
# Via MCP tool (preferred)
recommend_tools(task="Build a full-stack Facebook replica")

# Via REST
curl -X POST http://localhost:3030/agent/recommend-tools \
  -H "Content-Type: application/json" \
  -d '{"task": "Plan a 7-day Kyoto trip with hotel and flight costs"}'
```

Returns:
```json
{
  "task_category": "web_research",
  "mcp_servers": [
    {"name": "playwright", "install": "npx @playwright/mcp", "use_for": "Browser automation"},
    {"name": "fetch", "install": "npx @modelcontextprotocol/server-fetch", "use_for": "Fetch HTML/JSON"}
  ],
  "setup_commands": ["pip install hipcortex", "hipcortex install --mode proactive", "npx @playwright/mcp install"],
  "react_goal_template": "{\"success_factors\":[\"data_collected\",\"cost_computed\"],\"max_react_iterations\":30}"
}
```

Use `react_goal_template` as the starting `GoalPayload` for the ReAct loop. Install the `mcp_servers` before starting work.

**Supported categories:** `web_research` · `full_stack_dev` · `data_analysis` · `devops` · `code_review` · `agent_orchestration` · `content_creation` · `general`

## Harness Installation

```bash
# Standard (conservative) — explicit invocation
hipcortex install

# Proactive (substrate-first) — strongly recommends substrate before every response
hipcortex install --mode proactive

# Per-actor (multi-agent) — scoped actor namespace
hipcortex install --mode proactive --actor my-agent
```

Sets `HIPCORTEX_HARNESS_SOFT` mode and installs the proactive SKILL template into `~/.claude/CLAUDE.md`.

## Action Space

All MCP tools + REST equivalents. Key primitives:

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
| Spawn background daemon | — | `POST /v1/loop/subscribe` |
| Stop background daemon | — | `POST /v1/loop/stop/:handle` |
| Daemon handle status | — | `GET /v1/loop/status/:handle` |
| Renew workspace lease | — | `POST /v1/workspace/:id/renew` |
| Authorized WM ops | `list_authorized_actions` | `GET /v1/actions/authorized-wm` |
| Clarify goal requirements | — | `POST /goal/:id/clarify` |

## Clarify Gate

**Every GoalPayload must pass through the clarify gate before the ReAct loop starts.**

`POST /goal/:id/clarify` checks whether the goal's `acceptance_criteria` are empty. If so, it returns a `GoalNotClarified` error with the clarifying questions that must be answered first. This prevents the engine from iterating against underspecified goals.

```bash
curl -X POST http://localhost:3030/goal/<goal-id>/clarify
# 400 if criteria are empty → answer returned questions, update goal, retry
# 200 {"ok": true} → proceed to ReAct loop
```

Clarification questions are self-prompted from the goal `target_state`. Only call the user when the substrate cannot resolve the ambiguity itself.

## CriticVeto and VerifierGate

`src/loop_gates.rs` implements two pre-action gates that run inside `ReactEngine::run()` on every iteration:

### CriticGate (pre-action veto)

Evaluates the proposed action before it is executed. Decision rules:
- **Iteration 0**: always passes — no history to score against.
- **Iteration N > 0**: scores prior observations; if success fraction < 0.25, returns `Rejected { rationale }`.
  - A `Decision{action="rejected"}` record is written and the iteration is skipped (no act, no observation stored).

### VerifierGate (post-observe, pre-commit)

Compares the world-model's predicted next state against the actual observation target:
- `None` prediction (no WM data yet) → `Consistent` — observation is committed normally.
- Mismatch → `Mismatch { predicted, observed }` — a `Belief{action="verifier_mismatch"}` is written and the iteration is skipped.

The WM prediction is updated after each successful observation commit so the next iteration has fresh context.

## Cognitive Daemon — 8-Stage Loop

`SubstrateDaemon::subscribe_with_config(actor, cognitive, config)` spawns a background thread that runs indefinitely (or until `max_iterations` is reached for testing).

**Stage sequence per iteration:**

| # | Stage | What happens |
|---|-------|-------------|
| 0 | **Observe** | `purge_expired()` removes decayed records; snapshot taken |
| 1 | **Reflect** | Consolidation pressure computed from `SelfModel` |
| 2 | **Plan** | Decide: consolidate if `pressure > pressure_threshold` |
| 3 | **CriticVeto** | `CriticGate::evaluate("daemon_step", iter)` — iter 0 always passes |
| 4 | **Predict** | `WorldModelEnhanced::predict_next_state(actor, "daemon_step")` |
| 5 | **Act** | If not vetoed + pressure high: `AutoConsolidate { min_frequency }` |
| 6 | **Update** | Write `Temporal{action="daemon_step", metadata={vetoed, consolidated}}` |
| 7 | **ExitCheck** | Increment counter; check stop signal and `max_iterations`; sleep |

**Configuration** (`CognitiveLoopConfig`):

| Field | Default | Effect |
|-------|---------|--------|
| `interval_secs` | 30 | Sleep between iterations |
| `pressure_threshold` | 0.7 | Act stage consolidation threshold |
| `min_consolidation_frequency` | 3 | AutoConsolidate motif minimum |
| `max_iterations` | None | Iteration cap (use in tests) |

**REST API:**

```bash
# Spawn daemon (config is optional — defaults apply)
curl -X POST http://localhost:3030/v1/loop/subscribe \
  -d '{"actor": "my-agent", "config": {"interval_secs": 60, "pressure_threshold": 0.8}}'
# → {"ok": true, "handle_id": "<uuid>"}

# Stop daemon
curl -X POST http://localhost:3030/v1/loop/stop/<handle_id>
# → {"ok": true, "stopped": true}

# Check status
curl http://localhost:3030/v1/loop/status/<handle_id>
# → {"id": "...", "actor": "...", "iterations": 12, "status": "Running", "stage_counts": [12,12,...]}
```

`stage_counts` is a `Vec<u32>` of length 8. After N iterations each entry equals N.

## Workspace Lease

`Workspace::open()` returns a workspace with `lease_until: Option<SystemTime>`. By default no lease is set and the workspace never expires. To bound a workspace's TTL:

```bash
# Renew for 3600 seconds from now
curl -X POST http://localhost:3030/v1/workspace/<id>/renew \
  -d '{"secs": 3600}'
# → {"ok": true}
```

`is_expired()` returns `false` if `lease_until` is `None` (backward compatible). Expired workspaces are pruned in the daemon's Observe stage.

## WM Authorization Table

Not all world-model operations are permitted unconditionally. `GET /v1/actions/authorized-wm` returns the subset allowed given current `SelfModel` health:

```json
[
  {"op": "world_model_rollout", "requires_wm": true, "max_depth": 10, "max_iterations": 200, "authorized": true},
  {"op": "counterfactual",       "requires_wm": true, "max_depth": 5,  "max_iterations": 50,  "authorized": true},
  {"op": "intervene",            "requires_wm": true, "max_depth": 3,  "max_iterations": 10,  "authorized": false}
]
```

`authorized: false` means `can_execute` would veto that op. Check before calling rollout/counterfactual endpoints.

## OLS Drift Isolation

`PredictionMonitor::feed_with_obs(error, x, y)` accumulates feature/target pairs. `fit_ols()` returns coordinate-wise OLS weights that identify which input dimensions drive prediction error drift. Use `SelfModel::prediction_drift_weights()` to retrieve these weights; high-magnitude entries indicate drifting sensors or stale beliefs.

## Motif Contraction

`mine_and_consolidate(store, archive, wm, log, min_frequency, actor)` consolidates recurring `derived_from` chains into `Skill` and `Belief` records. v1.4.0 additions:

- **Cycle guard**: motifs whose member IDs form a `derived_from` cycle are skipped (prevents corrupted provenance chains from compacting into skills).
- **Causal validity**: if a WM is provided, the motif's first action is validated against the WM transition model; causally implausible motifs are skipped.
- **Archive before delete**: source records are appended to `ArchiveStore` before removal from the hot store, preserving cold-store audit trail.

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

## Lifecycle Self-Prompting

Self-prompting occurs at **every stage** — not just task start. Each phase has a dedicated substrate call:

| Phase | Tool | When |
|-------|------|------|
| 1. Tool discovery | `recommend_tools(task)` | Before anything |
| 2. Goal clarification | `POST /goal/:id/clarify` | Before creating GoalPayload |
| 3. Validation planning | `plan_validation(success_factors)` | Before loop starts |
| 4. Per-iteration observe | `get_live_beliefs()` | Start of every iteration |
| 5. Per-iteration reflect | `POST /memory/reflect` | When state is ambiguous |
| 6. Per-iteration progress | `check_progress(factors, obs, iter, max)` | End of every iteration |
| 7. Per-iteration exit | `should_exit(iter, max, progress, surprise)` | End of every iteration |

## Memory-Centric ReAct Loop (with exit gate)

```
# ── Setup (once) ──────────────────────────────────────────
recommend_tools(task)              → install MCP servers + skills
goal = POST /v1/cognitive/transact (GoalPayload)
POST /goal/:id/clarify             → 400 if criteria empty; answer + retry
plan = plan_validation(goal.success_factors) → test plan per factor

# ── Loop (iterate until should_exit ≠ continue) ───────────
for iteration in 1..max_iterations:
    OBSERVE:   live_beliefs = GET /memory/live_beliefs
    REFLECT:   POST /memory/reflect          # substrate CoT
    CRITIC:    CriticGate checked internally by ReactEngine
               → iter 0: always passes
               → iter N: score < 0.25 → Decision{rejected} written, iteration skipped
    ACT:       execute tool / write file / call API
    VERIFY:    VerifierGate: WM prediction vs actual target
               → mismatch → Belief{verifier_mismatch} written, iteration skipped
    STORE:     POST /memory/add (Temporal)
               → auto-fires: WMUpdater, BeliefInvalidator, EmergenceDetector
    GATE:      POST /worldmodel/can-execute  # before irreversible actions
    PROGRESS:  check = check_progress(success_factors, observations, iteration, max)
               if check.uncertainty_detected:
                   POST /memory/reflect
                   POST /v1/loop/omega       # attribution + sparse mutation
    EXIT:      decision = should_exit(iteration, max, check.progress_ratio, surprise)
               if decision.action == "succeed":  store Reflexion summary; break
               if decision.action == "fail":     store partial; report pending; break
               if decision.action == "escalate": omega + ask user; break
               # else: continue
```

### Exit hard rules — never bypass
- `should_exit` called every iteration, no exceptions
- `max_iterations` is an absolute cap — no silent extension
- Zero progress after 25% budget → `fail` + call `POST /goal/:id/clarify` to reframe
- High surprise (>0.8) + >50% budget + <50% progress → `escalate`, not `continue`

`src/agent_guidance.rs:should_exit` implements the 5-rule decision tree. `src/modules/loop_engine.rs:ReactEngine::run` implements the Rust-side loop including CriticVeto and VerifierGate.

## Goal-Driven Acceptance Criteria

```python
goal = {
    "actor": "claude-code",
    "target_state": "Build full-stack Facebook replica",
    "acceptance_criteria": [
        "auth module has passing tests",
        "news feed renders 20+ posts",
        "user profiles editable",
        "deployment manifests present",
    ],
    "success_factors": [
        {"name": "auth_system_tested",         "weight": 1.0, "satisfied": false},
        {"name": "news_feed_working",           "weight": 1.0, "satisfied": false},
        {"name": "user_profiles_working",       "weight": 1.0, "satisfied": false},
        {"name": "deployment_manifests_written","weight": 1.0, "satisfied": false},
    ],
    "max_react_iterations": 50,
}
```

`ReactEngine` terminates when all `success_factors` have `satisfied: true`, or when `max_react_iterations` is exhausted. Status: `Pending → InProgress → Succeeded | Failed`.

**Note:** `acceptance_criteria` must be non-empty or `POST /goal/:id/clarify` will block the loop.

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
- `src/loop_gates.rs` — `CriticGate` + `VerifierGate` (pre-action veto + post-observe gate)
- `src/substrate_daemon.rs` — `SubstrateDaemon` + `CognitiveLoopConfig` (8-stage background loop)
- `src/workspace.rs` — `Workspace` with `lease_until` and `WorkspaceRegistry::renew()`
- `src/action_registry.rs` — `WM_CONSTRAINTS` + `list_authorized_world_model()`
- `src/consolidation.rs` — `mine_and_consolidate` with cycle guard + archive-before-delete
- `src/modules/self_model/prediction_monitor.rs` — OLS drift weights
- `src/web_server.rs:handle_memory_live_beliefs` — unified beliefs surface
- `src/modules/integration_layer.rs:trigger_reflexion` — substrate CoT
- `sdk/python/hipcortex/install/SKILL.md` — installed harness policy
- `sdk/mcp/server.py` — MCP tool surface
- `docs/usage.md` — CLI and API reference
- `docs/integration.md` — framework adapters (LangChain, CrewAI, AutoGen)

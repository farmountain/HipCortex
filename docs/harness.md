# Claude Agent Harness — HipCortex v2.3.0

HipCortex turns Claude Code (or any MCP-capable LLM) into a **substrate-first autonomous agent**: the cognitive state (memories, beliefs, world model, coherence) is the primary durable mind; the LLM is a thin language surface used only for final output or high-entropy creative hypotheses.

## Architecture

```
User instruction
     │
     ▼
Claude Code (LLM)  — language surface only
     │
     ├─ MUST: get_live_beliefs() ──────────────┐
     │                                          │
     ├─ GroundingGate check (Stage 1c):         │
     │    if entity ungrounded:                 │
     │      open_intent(Probe) ──────────────── │──► Host runner
     │      await accept_receipt ◄─────────────│─── Host runner
     │    else: ReAct loop (instrumental)       │
     │                                          │
     ├─ ReAct loop (goal-driven):               │  HipCortex Substrate
     │    observe → get_live_beliefs            │  (primary cognitive mind)
     │    reflect → POST /memory/reflect        │
     │    act → tools + file writes             │
     │    [env obs ONLY via accept_receipt]      │
     │    success_factors check                 │
     │                                          │
     │    LoopEngine.run_omega_loop() ──────────┤
     │    (on surprise/gap)                     │
     │                                          │
     └─ minimal LLM output ◄───────────────────┘

Background (SubstrateDaemon):
Observe → Reflect → Plan → GroundingGate → CriticVeto → Predict
         → Act (Probe intent or ReAct) → Update → ExitCheck
         (runs interval_secs; independent of LLM turns)
```

## Grounding Seam (v2.3.0) — THE env API

**HipCortex never observes the environment directly. It issues intents. The host executes and returns receipts.**

```
HipCortex                     Host Runner
    │                              │
    │── POST /intent/open ────────►│  ActionIntent{kind=Probe, target_entity, deadline_ms}
    │                              │  (host polls GET /intent/open?actor=<actor>)
    │                              │  host executes the probe (MCP call, filesystem, API)
    │◄── POST /intent/receipt ─────│  ActionReceipt{intent_id, ok, observation, sensor_path}
    │                              │
    │  AcceptReceipt atomically:   │
    │   • writes Temporal{receipt_observation}
    │   • updates WM entity_contacts (n_observations++)
    │   • marks intent Received
    │   • if n_obs ≥ 4 → GroundingGate exits
```

### GroundingGate rules

| Condition | Effect |
|-----------|--------|
| `coverage(Ê; goal predicates) < τ_c = 0.6` | Gate active → only Probe intents legal |
| `max_epistemic > τ_e = 0.5` (n < 4 observations) | Gate active → only Probe intents legal |
| Open/InFlight intents exist | Q10 = `probe_entity:<id>` / `ground_workspace` |
| Intent expires (host silence) | Intent marked Expired → Q10 = `escalate_to_user` |
| All goal entities have n_obs ≥ 4 AND coverage ≥ τ_c | Gate exits → instrumental planning allowed |

**`POST /memory/add` must NOT be used to record env observations.** It is only for LLM-authored beliefs, goals, and skills. Env observations must come through `accept_receipt`.

## Tool Discovery (start of task)

For any complex multi-step task, call `recommend_tools` to get the right MCP servers, skills, and setup commands for your use case:

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

Use `react_goal_template` as the starting `GoalPayload` for the ReAct loop. Install `mcp_servers` before starting work.

## Harness Installation

```bash
# Standard (conservative) — explicit invocation
hipcortex install

# Proactive (substrate-first) — strongly recommends substrate before each response
hipcortex install --mode proactive

# Per-actor (multi-agent) — scoped actor namespace
hipcortex install --mode proactive --actor my-agent
```

Sets `HIPCORTEX_HARNESS_SOFT` mode and installs the proactive SKILL template into `~/.claude/CLAUDE.md`.

## Action Space

All MCP tools and REST equivalents. Key primitives:

| Action | MCP Tool | REST |
|--------|----------|------|
| Observe substrate state | `get_live_beliefs` | `GET /memory/live_beliefs` |
| Probe env entity (grounding) | `open_intent` | `POST /intent/open` |
| Submit host observation | `accept_receipt` | `POST /intent/receipt` |
| List open intents (host polling) | — | `GET /intent/open?actor=<actor>` |
| Substrate chain-of-thought | `reflect` | `POST /memory/reflect` |
| Store LLM belief/goal/skill | `add_memory` | `POST /memory/add` |
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

> **Note:** `add_memory` / `POST /memory/add` is for LLM-authored records (Belief, Goal, Skill, Reflexion).
> **Never** use it to inject env observations — use `accept_receipt` instead.

## Cognitive Daemon — 9-Stage Loop

`SubstrateDaemon::subscribe_with_config(actor, cognitive, config)` spawns a background thread that runs until `max_iterations` is reached.

| # | Stage | What happens |
|---|-------|-------------|
| 0 | **Observe** | `purge_expired()` removes decayed records; snapshot taken |
| 1 | **Reflect** | Consolidation pressure from `SelfModel`; health → SynthesisMode; autonomous goal synthesis from `most_uncertain_entity` if no InProgress goal |
| 1b | **OOD Detect** | `get_entity_anomalies(most_uncertain_entity)` — if severity > threshold: `CreditAssign("ood_shift:entity_id")`. One CreditAssign per tick |
| 1c | **GroundingGate** | Builds `entity_contact_snap` from WM. If coverage < τ_c=0.6 or max_epistemic > τ_e=0.5: `grounding_active=true`, selects `probe_target` |
| 2 | **Critic** | `CriticGate::evaluate` against SelfModel; `CreditAssign` on failure; triggers `ClarifyEngine{ConsecutiveVeto}` after 3 consecutive vetoes |
| 3 | **Predict** | `WorldModelEnhanced::predict_next_state` — outputs labelled `PredictedOnly` |
| 4 | **VerifierGate** | Compares prediction vs observation; mismatch → `CreditAssign`; `ClarifyEngine{PreSuccess}` if all factors satisfied but verifier still fires |
| 5 | **Act** | If `grounding_active`: emit `OpenIntent(Probe, probe_target)`. Else if active goal: run `ReactEngine` (instrumental). Else: consolidate |
| 6 | **Update** | If `AcceptReceipt` arrived: ingest → Temporal + WM update. If Open intent expired: mark Expired + `CreditAssign("host_silence:intent=<id>")` |
| 7 | **ExitCheck** | Evaluate `CognitiveLoopConfig` exit conditions |

```bash
# Spawn daemon (config optional — defaults apply)
curl -X POST http://localhost:3030/v1/loop/subscribe \
  -d '{"actor": "my-agent", "config": {"interval_secs": 60, "pressure_threshold": 0.8}}'
# → {"ok": true, "handle_id": "<uuid>"}

# Stop daemon
curl -X POST http://localhost:3030/v1/loop/stop/<handle_id>

# Check status
curl http://localhost:3030/v1/loop/status/<handle_id>
# → {"id": "...", "actor": "...", "iterations": 12, "status": "Running", "stage_counts": [12,12,...]}
```

## Clarify Gate

**ClarifyEngine (`src/clarify_engine.rs`) runs inside the daemon loop — no host call required.**

Triggers (all daemon-owned, max 3 rounds per goal):
- `EmptyAC` — goal has no acceptance criteria at create time
- `ConsecutiveVeto` — CriticGate rejects ≥ 3 times in a row
- `PreSuccess` — all success factors satisfied but verifier still detects mismatch

`next_recommendation.recommended_op` = `POST /goal/:id/clarify` (added v1.8.0).

## Host Runner Protocol (v2.3.0)

The host runner is responsible for executing intents and returning receipts. **It does not write memories directly.**

```bash
# 1. Poll for open intents (or subscribe to push on SSE when available)
curl "http://localhost:3030/intent/open?actor=my-agent"
# → {"ok": true, "intents": [...], "count": 1}

# 2. Execute the probe (e.g. via Playwright, filesystem, API)
#    The host uses its own tools — HipCortex does not execute

# 3. Return receipt
curl -X POST http://localhost:3030/intent/receipt \
  -H "Content-Type: application/json" \
  -d '{
    "actor": "my-agent",
    "intent_id": "<uuid from step 1>",
    "ok": true,
    "observation": {"disk_free_gb": 42.5, "status": "healthy"},
    "sensor_path": "mcp:filesystem"
  }'
# → {"ok": true}
```

Or via MCP tools:

```python
# In Claude Code / MCP host
intent_resp = open_intent(actor="my-agent", target_entity="filesystem", deadline_ms=30000)
# ... host executes ...
accept_receipt(
    actor="my-agent",
    intent_id=intent_resp["intent_id"],
    ok=True,
    observation={"disk_free_gb": 42.5},
    sensor_path="mcp:filesystem",
)
```

## Memory-Centric ReAct Loop (with grounding + exit gate)

```
# ── Setup (once) ────────────────────────────────────────────────────
recommend_tools(task)
install MCP servers + skills
goal = POST /v1/cognitive/transact (GoalPayload)
POST /goal/:id/clarify → 400 if criteria empty; answer + retry
plan_validation(goal.success_factors) → test plan per factor

# ── Grounding Phase (before instrumental loop) ───────────────────────
while GroundingGate active:
    PROBE:   open_intent(actor, target_entity, deadline_ms=30000)
    WAIT:    host executes and calls accept_receipt
    CHECK:   GET /intent/open?actor= → empty means all received
    EXIT:    GroundingGate exits when n_obs ≥ 4 for all goal entities
    TIMEOUT: expired intent → Q10 = escalate_to_user → ask human

# ── Instrumental Loop (iterate until should_exit ≠ continue) ─────────
for iteration in 1..max_iterations:
    OBSERVE: get_live_beliefs GET /memory/live_beliefs
    REFLECT: POST /memory/reflect  # substrate CoT
    CRITIC:  CriticGate (internal to ReactEngine)
             → iter 0: always passes
             → iter N: score < 0.25 → Decision{rejected} written, skip
    ACT:     execute tool / write file / call API
    VERIFY:  VerifierGate: WM prediction vs actual target
             → mismatch → CreditAssign written, iteration skipped
    STORE:   LLM-authored observations → POST /memory/add (Temporal)
             Env observations → accept_receipt ONLY (never add_memory)
    GATE:    POST /worldmodel/can-execute before irreversible actions
    PROGRESS: check_progress(success_factors, observations, iteration, max)
              if check.uncertainty_detected:
                POST /memory/reflect
                POST /v1/loop/omega  # attribution + sparse mutation
    EXIT:    should_exit(iteration, max, check.progress_ratio, surprise)
             succeed → store Reflexion summary; break
             fail → store partial; report pending; break
             escalate → omega + ask user; break
             # else: continue
```

### Exit hard rules — never bypass

- `should_exit` called every iteration, no exceptions
- `max_iterations` is an absolute cap — no silent extension
- Zero progress at 25% budget → `fail` + `POST /goal/:id/clarify` to reframe
- High surprise (>0.8) + >50% budget + <50% progress → `escalate`, not `continue`

`src/agent_guidance.rs:should_exit` implements the 5-rule decision tree.

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
        {"name": "auth_system_tested",          "weight": 1.0, "satisfied": false},
        {"name": "news_feed_working",           "weight": 1.0, "satisfied": false},
        {"name": "user_profiles_working",       "weight": 1.0, "satisfied": false},
        {"name": "deployment_manifests_written","weight": 1.0, "satisfied": false},
    ],
    "max_react_iterations": 50,
}
```

`ReactEngine` terminates when all `success_factors` are `satisfied: true`, or when `max_react_iterations` is exhausted. Status: `Pending → InProgress → Succeeded | Failed`.

**Note:** `acceptance_criteria` must be non-empty or `POST /goal/:id/clarify` will block the loop.

## Cognitive Report — Q3/Q8/Q10 truth (v2.3.0)

| Q | What it reports |
|---|----------------|
| **Q3** valid assumptions | JTMS `In` **and** `contact_kind ≠ PredictedOnly`. Kalman fill-ins are NOT valid assumptions |
| **Q8** uncertain | Virgin/Stale entities + high Σ + never-probed actuators + **expired intents** (host silence = knowledge hole) |
| **Q9** authorized | ∩ actuator reachable ∩ **not grounding-blocked** for that op class |
| **Q10** next | `probe_entity:<id>` / `ground_workspace` → `escalate_to_user` on silence → `react_loop` only when grounded |

## Multi-Agent Actor Scoping

```bash
hipcortex install --actor frontend-agent   # Claude working on UI
hipcortex install --actor backend-agent    # Claude working on API
hipcortex install --actor orchestrator     # Top-level planner
```

Actor scoping in queries:

```
GET  /memory/live_beliefs?actor=frontend-agent   → only frontend context
GET  /memory/search?query=auth&actor=backend-agent
GET  /intent/open?actor=backend-agent            → intents for backend only
DELETE /memory/forget/frontend-agent             → GDPR wipe per agent
```

All agents share the global SymbolicStore and WorldModelEnhanced, but Temporal observations, Goals, and Intents are actor-scoped.

## Workspace Lease

`Workspace::open()` returns a workspace with `lease_until: Option<SystemTime>`. By default no lease is set.

```bash
curl -X POST http://localhost:3030/v1/workspace/<id>/renew \
  -d '{"secs": 3600}'
# → {"ok": true}
```

Expired workspaces are pruned in the daemon's Observe stage.

## WM Authorization Table

```bash
curl http://localhost:3030/v1/actions/authorized-wm
```

```json
[
  {"op": "world_model_rollout", "requires_wm": true, "max_depth": 10, "max_iterations": 200, "authorized": true},
  {"op": "counterfactual",      "requires_wm": true, "max_depth":  5, "max_iterations":  50, "authorized": true},
  {"op": "intervene",           "requires_wm": true, "max_depth":  3, "max_iterations":  10, "authorized": false}
]
```

`authorized: false` means `can_execute` would veto this op. Check before calling rollout/counterfactual endpoints.

## OLS Drift Isolation

`PredictionMonitor::feed_with_obs(error, x, y)` accumulates feature/target pairs. `fit_ols()` returns coordinate-wise OLS weights to identify input dimensions driving prediction error drift. Use `SelfModel::prediction_drift_weights()` to retrieve; high-magnitude entries indicate drifting sensors or stale beliefs.

## Motif Contraction

`mine_and_consolidate(store, archive, wm, log, min_frequency, actor)` consolidates recurring `derived_from` chains into `Skill` or `Belief` records.

- **Cycle guard**: motifs with cyclic member IDs are skipped (prevents corrupted provenance chains).
- **Causal validity**: if WM provided, motif's first action is validated against the WM transition model.
- **Archive before delete**: source records are appended to `ArchiveStore` before removal from the hot store.

## Lifecycle Self-Prompting

Self-prompting occurs at **every stage** — not just task start:

| Phase | Tool | When |
|-------|------|------|
| 1. Tool discovery | `recommend_tools(task)` | Before anything |
| 2. Goal clarification | `POST /goal/:id/clarify` | Before creating GoalPayload |
| 3. Grounding check | `GET /intent/open?actor=` | Before instrumental planning |
| 4. Validation planning | `plan_validation(success_factors)` | Before loop starts |
| 5. Per-iteration observe | `get_live_beliefs()` | Start every iteration |
| 6. Per-iteration reflect | `POST /memory/reflect` | When state is ambiguous |
| 7. Per-iteration progress | `check_progress(factors, obs, iter, max)` | End every iteration |
| 8. Per-iteration exit | `should_exit(iter, max, progress, surprise)` | End every iteration |

## Harness Compliance Targets

| Mode | Expected substrate calls/turn | LLM token reduction |
|------|-------------------------------|---------------------|
| Conservative | 1-2 (explicit only) | 30-50% |
| Proactive | 3-6 (get_live_beliefs + reflect + predict) | 70-99% |
| Proactive + omega | 6-10 (+ loop on surprise) | 80-99% |

## Configuration

| Env var | Default | Effect |
|---------|---------|--------|
| `HIPCORTEX_AGENT_DEFAULTS` | off | Wire PerceptionSession to all AgentMessage paths; enable auto-ingest |
| `HIPCORTEX_HARNESS_SOFT` | on | MCP warns on non-substrate-first calls; does not hard-block |
| `HIPCORTEX_ACTOR` | `mcp-session` | Default actor for MCP server session |
| `HIPCORTEX_URL` | `http://localhost:3030` | Server URL |

## Related

- `src/action_intent.rs` — `ActionIntent`, `ActionReceipt`, `ContactKind`, `GroundingStatus`, `ActuatorRegistry`
- `src/grounding_gate.rs` — `GroundingGate` (τ_c=0.6, τ_e=0.5), `expire_stale`
- `src/modules/loop_engine.rs` — `ReactEngine`, `LoopEngine.run_omega_loop()`
- `src/loop_gates.rs` — `CriticGate`, `VerifierGate`
- `src/substrate_daemon.rs` — `SubstrateDaemon`, `CognitiveLoopConfig` (9-stage loop)
- `src/workspace.rs` — `Workspace`, `WorkspaceRegistry::renew()`
- `src/action_registry.rs` — `WM_CONSTRAINTS`, `list_authorized_world_model()`
- `src/consolidation.rs` — `mine_and_consolidate` with cycle guard + archive-before-delete
- `src/modules/self_model/prediction_monitor.rs` — OLS drift
- `src/web_server.rs` — REST endpoints including `/intent/open`, `/intent/receipt`
- `sdk/python/hipcortex/install/SKILL.md` — proactive skill template
- `sdk/python/hipcortex/install/RUNNER_SKILL.md` — Claude Code runner skill
- `sdk/python/hipcortex/runner.py` — headless `IntentRunner` class
- `sdk/mcp/server.py` — MCP tools including `open_intent`, `accept_receipt`
- `docs/usage.md` — CLI and API reference
- `docs/integration.md` — LangChain, CrewAI, AutoGen integration

## Headless Runner (3-Month Autonomy)

When the IDE is closed, the SubstrateDaemon continues emitting Probe intents.
Without a runner, those intents expire and the daemon writes `host_silence`
CreditAssigns forever. The headless runner prevents this.

### Option A — Python CLI (no IDE required)

```bash
# Install SDK
pip install hipcortex

# Start runner (blocks, poll every 30 s)
hipcortex runner --actor hipcortex-runner --url http://localhost:3030

# Options
hipcortex runner --actor myagent --interval 60 --dry-run
```

The runner polls `GET /intent/open?actor=<actor>`, dispatches each probe, and
posts `POST /intent/receipt`. Dispatch table:

| `sensor_path`  | Action                                   |
|----------------|------------------------------------------|
| `filesystem`   | `os.stat(target_entity)`                 |
| `http`         | `requests.get(target_entity)`            |
| `shell:ping`   | `ping -c 1 <target>` (allowlisted)       |
| `default`      | Return `{"reachable": true}` immediately |

Expired intents (current time > `deadline_ms`) are skipped, not posted.

### Option B — Claude Code as IDE runner

When the IDE is open, Claude Code can act as runner by following
`sdk/python/hipcortex/install/RUNNER_SKILL.md`. After each tool call:

1. `GET /intent/open?actor=claude-code` — check for pending Probes.
2. For each intent: execute probe using Read/WebFetch/Bash tools.
3. `POST /intent/receipt` (or `mcp__hipcortex__accept_receipt`) — never `add_memory`.

### Deployment note

Run the headless runner as a background process or system service:

```bash
# systemd / launchd
hipcortex runner --actor hipcortex-runner &

# Docker / cron
hipcortex runner --actor hipcortex-runner --interval 60
```

The runner is stateless and crash-safe — it re-polls on restart with no
lost receipts (intents stay open until deadline or receipt arrives).

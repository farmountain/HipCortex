# HipCortex Memory Engine & Cognitive OS for VS Code & Antigravity IDE (`v2.6.0`)

[![Version](https://img.shields.io/badge/version-v2.6.0-blue.svg)](package.json)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
![Latency](https://img.shields.io/badge/write_p50-0.48ms__--__0.61ms-brightgreen.svg)
![Token Savings](https://img.shields.io/badge/token_savings-59%25__--__88%25-blueviolet.svg)

**Give your AI coding assistant persistent, cross-session causal memory with a full cognitive OS substrate — transactional belief revision, multi-agent workspaces, world-model rollout, DigitalTwin simulation, grounded probe planning, and topological graph tools.**

VSIX **2.6.0** (Closed Spine) · server/pip/npm **2.6.0**. 473 unit + 173 integration + 56 property + 9 AC-E1..E3/W1..W3/B1..B3 (v2.6.0) + 10 v2.5.0 + 5 v2.4.0 + 7 v2.3.0 + 6 v2.2.0 + 3 v2.1.0 + 5 v2.0.0 acceptance, **0 failures**. See [docs/channels.md](../docs/channels.md).

---

## What's new in v2.6.0 — Closed Spine

Wires the cognitive spine end-to-end: probe receipts feed back into the world model and reinforce supporting beliefs; every ReactEngine step is pre-flighted by an injectable `ExecutionGate`.

| Change | Details |
|--------|---------|
| **ExecutionGate in daemon** | `CognitiveLoopConfig` gains `#[serde(skip)] execution_gate` slot; Stage 5 evaluates gate before every `ReactEngine` step; rejection writes `Temporal{gate_veto}` and skips the step |
| **WM receipt feedback** | `accept_receipt_impl` calls `update_from_receipt(entity, ok, wm)` in a separate write lock; WM learns `entity → probe → entity_{ok\|failed}` Dirichlet-Multinomial transition rates |
| **Belief reinforcement** | `BeliefExecutive::reinforce(store, id, 0.05)` — positive-evidence path; called for every belief whose proposition contains the probed entity when `receipt.ok=true` |

---

## What's new in v2.5.0 — IG Probe Ranking + add_memory Adapter

| Change | Details |
|--------|---------|
| **IG probe ranking** | `ig_score = epistemic(n) × deficit(n) × probe_penalty(probe_count)`; grounded entities (n ≥ 4) score 0.0 and are never re-probed; `ig_probe_target()` returns `None` when all entities grounded — daemon exits probe loop |
| **add_memory adapter** | Three-layer enforcement: Rust `POST /memory/add` returns HTTP 400 + redirect when `intent_id` + Temporal; MCP `add_memory` routes to `handle_accept_receipt`; Python SDK routes to `POST /intent/receipt` |

---

## What's new in v2.4.0 — Published Runner

| Change | Details |
|--------|---------|
| **Headless IntentRunner** | `sdk/python/hipcortex/runner.py` — polls `GET /intent/open`, dispatches by `sensor_path` (filesystem / http / shell allowlist / default), posts `POST /intent/receipt`; `hipcortex runner` CLI subcommand; `RUNNER_SKILL.md` wires Claude Code as IDE runner |
| **Expiry guard** | `deadline_ms` check skips expired intents before dispatch — probe loop never stalls on silence |

---

## What's new in v2.3.0 — Grounding Obligation + Intent/Receipt Seam

| Change | Details |
|--------|---------|
| **GroundingGate** | Blocks `react_loop` when `coverage < τ_c=0.6` OR any goal-relevant entity has `epistemic > τ_e=0.5` (n < 4 observations). Stage 5 emits Probe intents instead |
| **Intent/Receipt seam** | `ActionIntent` (Probe\|Instrumental\|ClarifySense) + `ActionReceipt` are the only env API. `AcceptReceipt` atomically writes `Temporal{receipt_observation}` + updates `WorldModelEnhanced.entity_contacts` |
| **Q3 PredictedOnly filter** | Q3 now excludes beliefs with `contact_kind = Some(PredictedOnly)` — Kalman fill-ins no longer treated as facts |
| **Q10 probe-first** | Q10: `probe_entity` / `ground_workspace` while intents open → `escalate_to_user` on expired silence → `react_loop` only when grounded |

---

## What's new in v2.2.0 — Epistemic Filter Closure

| Change | Details |
|--------|---------|
| **Q2 JTMS filter** | `learned_beliefs` now requires `JtmsLabel::In AND confidence > 0.3`; Out beliefs excluded regardless of confidence |
| **Q8 Unknown beliefs** | `uncertain_beliefs` includes `JtmsLabel::Unknown` regardless of confidence |
| **Verifier Temporal** | `VerifierGate::check_and_record()` atomically writes `Temporal{verifier_mismatch_observed}` on mismatch |

---

## What's new in v1.7.0 — Epistemic Closure

| Change | Details |
|--------|---------|
| **ClarifyEngine** | Self-prompting loop (max 3 rounds) — triggered on empty `success_factors` or ≥3 consecutive vetoes. Writes `Reflexion{self_clarified}` on success, deduped `Belief{clarify_needed}` on escalation |
| **Dynamic CriticGate threshold** | SelfModel health drives threshold: low health → 0.50 (strict), high health → 0.15 (autonomous), balanced → 0.25 |
| **Veto as revision event** | CriticGate rejection fires `CognitiveDelta::CreditAssign(ExplicitFail)` — veto is a learning signal, not a skipped tick |
| **SelfModel steers loop** | `recommend_loop_config()` returns `{effective_veto_threshold, SynthesisMode}` per tick |
| **JTMS as report truth** | `cognitive_report` Q3 filters on `JtmsLabel::In`; `Unknown` fallback to confidence ≥ 0.5; `Out` excluded at any confidence |

---

## Zero-config onboarding (no Rust or Cargo required)

Install from Marketplace / Open VSX / GitHub release VSIX. Extension **starts a local Rust webserver** under `~/.hipcortex-vscode/bin/` (or uses `hipcortex.apiUrl`).

- **Zero external DB / Docker** for default petgraph path
- **Local-first** storage under `~/.hipcortex-vscode/storage`
- **Auto-recovery**: restarts server before queries when down
- **Executable bundled bins**: `chmod 0755` applied on macOS/Linux (fixes spawn `EACCES`)
- **Passive capture**: saves code edits and terminal output automatically when `hipcortex.passiveCapture` is `true`

```bash
code --install-extension hipcortex-memory-2.6.0.vsix
```

---

## What's new in v1.3.0 — Autonomous Agent Harness

| Capability | Details |
|-----------|---------|
| **Proactive harness mode** | `hipcortex install --mode proactive` — SKILL mandates `get_live_beliefs` before every response; 70-99% LLM token reduction |
| **Unified `live_beliefs`** | `GET /memory/live_beliefs` returns symbolic facts + code KG + hypotheses + world preds + self/coherence intel in one call |
| **AgentMessage auto-ingest** | `HIPCORTEX_AGENT_DEFAULTS=1` — PerceptionSession wired for agent paths; messages auto-stored as Temporal records |
| **Multi-agent `--actor`** | `hipcortex install --actor <name>` — per-actor SKILL install; shared substrate, no cross-actor contamination |
| **ReAct goal loop** | `ReactEngine` + `LoopEngine.run_omega_loop()` — goal-driven iterations with causal attribution on surprise |
| **`/memory/reflect`** | `POST /memory/reflect` — substrate chain-of-thought via AureusBridge (world prior + coherence before LLM output) |

---

## What's new in v1.2.0 — Causal SCM Continuous Substrate

| Capability | Details |
|-----------|---------|
| **Structural Equations** | `f_i(PA_i, U_i)` on every causal node via `StructuralEquation` trait |
| **Interventions** | `CognitiveDelta::Intervene` mutates shared graph, writes Reflexion audit |
| **Credit Assignment** | AAP triad (Abduction→Action→Prediction) isolates broken structural equation |
| **DigitalTwin clamping** | `step()` clamps RK4 output to pinned vars — causal impulses override ODE |
| **MCP tools** | `causal_intervene`, `causal_counterfactual`, `causal_credit_assign`, `causal_rewrite_equation` |

---

## What's new in v1.1.0 — Cognitive Loop Closure

| Capability | What it does |
|-----------|-------------|
| **GoalScheduler** | Ranks Pending/InProgress Goals by `urgency / estimated_cost`; returns highest-priority next goal |
| **EmergenceDetector** | Scans last 50 Temporal records every 10 writes; auto-synthesizes Beliefs from dense token patterns |
| **BeliefInvalidator** | Contradiction detection; decays confidence by `score × 0.3`; writes `belief_invalidated` marker at conf < 0.2 |
| **DecisionPayload** | New `MemoryType::Decision` per ReactEngine act-phase — captures `option_chosen`, `alternatives`, `rationale`, `confidence`, `outcome` |
| **CognitiveStateReport** | Single call answers all 10 cognitive questions: goals, beliefs, assumptions, decisions, failures, authorized actions, next recommendation |
| **WorldModelUpdater** | Closes feedback loop: ReactEngine feeds each observation into Dirichlet-Multinomial world model |
| **ActionRegistry** | `ALL_OPS` + `list_authorized(self_model)` — agent always knows what it's allowed to do |

New REST: `GET /v1/cognitive/report`, `GET /v1/goals`, `GET /v1/actions/authorized`, `GET /v1/memory/:id/provenance`

New MCP tools: `cognitive_report`, `list_authorized_actions`, `get_provenance`

---

## `@hipcortex` chat commands

Open Copilot / Antigravity chat and type `@hipcortex`:

- `@hipcortex health` — server status, calibration score, epistemic entropy
- `@hipcortex add <content>` — store decision / preference / constraint
- `@hipcortex query <query>` — semantic + topological retrieval
- `@hipcortex status` — Headroom vs Caveman mode and savings

---

## Language Model Tools (10)

Extension registers **10** tools with `vscode.lm` (requires host LM tool API):

| Tool | Purpose |
|------|---------|
| `hipcortex_search` | Semantic + live-belief-aware search |
| `hipcortex_health` | Health + calibration + capability gate |
| `hipcortex_predict` | WorldModel single-step `P(s'|s,a)` |
| `hipcortex_rollout` | Multi-step Kalman rollout with drift alarm |
| `hipcortex_graph_search` | PPR / related memories from seed UUID |
| `hipcortex_causal` | Causal attribution |
| `hipcortex_topo_ppr` | Topological Personalized PageRank |
| `hipcortex_deconstruct` | Hypothesis → candidate causal edges |
| `hipcortex_check_edge` | Contradiction / cycle check before link |
| `hipcortex_can_execute` | SelfModel ExecutionGate |

---

## VS Code Commands (15)

| Command | Action |
|---------|--------|
| `hipcortex.addMemory` | Add memory record |
| `hipcortex.queryMemory` | Query memory records |
| `hipcortex.healthCheck` | System health check |
| `hipcortex.predictState` | Predict next state |
| `hipcortex.systemHealth` | Calibrated health + ECE |
| `hipcortex.stateDiff` | Causal state diff (tx range) |
| `hipcortex.cognitiveHealth` | Cognitive health status |
| `hipcortex.cognitiveSnapshot` | Cognitive snapshot |
| `hipcortex.twinCreate` | Create DigitalTwin |
| `hipcortex.twinStep` | DigitalTwin: Step |
| `hipcortex.twinRollout` | DigitalTwin: Rollout |
| `hipcortex.twinGet` | DigitalTwin: Show State |
| `hipcortex.experienceTiers` | Show Experience Tier Stats |
| `hipcortex.restartServer` | Restart server |
| `hipcortex.testExtension` | Test extension |

---

## MCP Integration (45 tools, 7 resources)

MCP hosts (Claude Code, Cursor, Windsurf, …) use the Python MCP server via `hipcortex install`.  
45 tools + 7 auto-injected resources:

- `hipcortex://context/relevant` — top-k semantically relevant memories
- `hipcortex://beliefs/current` — active belief records
- `hipcortex://context/conversation` — recent temporal traces
- `hipcortex://experience/tiers` — ExperienceStore tier stats for current actor

Register in `.mcp.json`:
```json
{
  "mcpServers": {
    "hipcortex": {
      "type": "stdio",
      "command": "python",
      "args": ["/path/to/hipcortex/sdk/mcp/server.py"],
      "env": { "HIPCORTEX_URL": "http://localhost:3030" }
    }
  }
}
```

---

## Headroom & Caveman (token savings)

- **Headroom (Top-5)**: ~59–84% token reduction vs full history dump
- **Caveman (Top-3)**: ~70–88% in tight debug loops

---

## Configuration (`settings.json`)

```json
{
  "hipcortex.apiUrl": "http://127.0.0.1:3030",
  "hipcortex.apiKey": "",
  "hipcortex.autoStart": true,
  "hipcortex.optimizationMode": "headroom",
  "hipcortex.passiveCapture": true
}
```

---

## Local development & packaging

```bash
cd vscode-extension
npm install
npm run compile
npm test
npx @vscode/vsce package --no-dependencies
```

Produces `hipcortex-memory-2.6.0.vsix` (version from `package.json`).

---

## Related

- Channel honesty: [docs/channels.md](../docs/channels.md) · `hipcortex channels`
- Capability matrix: [docs/capabilities.md](../docs/capabilities.md)
- Host wizards: [docs/hosts/README.md](../docs/hosts/README.md)
- Architecture: [docs/architecture.md](../docs/architecture.md)

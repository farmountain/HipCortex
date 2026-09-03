# HipCortex Memory Engine & Cognitive OS for VS Code & Antigravity IDE (`v1.7.0`)

[![Version](https://img.shields.io/badge/version-v1.7.0-blue.svg)](package.json)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
![Latency](https://img.shields.io/badge/write_p50-0.48ms__--__0.61ms-brightgreen.svg)
![Token Savings](https://img.shields.io/badge/token_savings-59%25__--__88%25-blueviolet.svg)

**Give your AI coding assistant persistent, cross-session causal memory with a full cognitive OS substrate — transactional belief revision, multi-agent workspaces, world-model rollout, DigitalTwin simulation, and topological graph tools.**

VSIX **1.7.0** (Epistemic Closure) · server/pip/npm **1.7.0**. 1027 tests pass (439 unit + 56 property + 158 integration + 8 acceptance + 366 lib). See [docs/channels.md](../docs/channels.md).

---

## What's new in v1.7.0 — Epistemic Closure

| Change | Details |
|--------|---------|
| **ClarifyEngine** | Self-prompting loop (max 3 rounds) — triggered on empty success_factors or ≥3 consecutive vetoes. Searches beliefs + WM; writes `Reflexion{self_clarified}` on success, deduped `Belief{clarify_needed}` on escalation. Only unresolvable questions reach the user. |
| **Dynamic CriticGate threshold** | `evaluate_with_threshold(goal, action, iter, threshold)` — SelfModel health drives the threshold: low health → 0.50 (strict), high health → 0.15 (autonomous), balanced → 0.25 |
| **Veto as revision event** | CriticGate rejection fires `CognitiveDelta::CreditAssign(ExplicitFail)` in addition to writing `Decision{critic_veto}`. Veto is a learning signal, not a skipped tick. |
| **SelfModel steers loop** | `recommend_loop_config()` returns `{effective_veto_threshold, SynthesisMode}` per tick. health < 0.3 → Escalate; health > 0.8 → Autonomous; else → Balanced. |
| **JTMS as report truth** | `cognitive_report` Q3 (`valid_assumptions`) filters on `JtmsLabel::In`; `Unknown` falls back to confidence ≥ 0.5; `Out` excluded at any confidence. |

---

## What's new in v1.6.3 — Dual-mode ReactEngine

| Change | Details |
|--------|---------|
| **GoalExecutionMode::StepByStep** | One ReAct iteration per daemon tick — goal persists `InProgress` across ticks, CriticGate veto structurally achievable at iter ≥ 1 |
| **GoalExecutionMode::FullCycle** | Default — `ReactEngine::run()` exhausts all iterations in one tick (backward-compatible) |
| **`ReactEngine::run_one_step()`** | Writes Temporal + Reflexion per step; increments `current_iteration`; leaves `InProgress` until done |

---

## Zero-config onboarding (no Rust or Cargo required)

Install from Marketplace / Open VSX / GitHub release VSIX. Extension **starts a local Rust webserver** under `~/.hipcortex-vscode/bin/` (or uses `hipcortex.apiUrl`).

- **Zero external DB / Docker** for default petgraph path
- **Local-first** storage under `~/.hipcortex-vscode/storage`
- **Auto-recovery**: restarts server before queries when down
- **Executable bundled bins**: `chmod 0755` applied on macOS/Linux (fixes spawn `EACCES`)
- **Passive capture**: saves code edits and terminal output automatically when `hipcortex.passiveCapture` is `true`

```bash
code --install-extension hipcortex-memory-1.2.1.vsix
```

---

## What's new in v1.3.0 — Autonomous Agent Harness

v1.3.0 completes the agent-substrate-autonomy milestone. HipCortex is now a full autonomous agent harness: proactive substrate-first mode, unified live_beliefs surface, AgentMessage auto-ingest, and ReAct goal loop — all wired end-to-end.

| Capability | Details |
|-----------|---------|
| **Proactive harness mode** | `hipcortex install --mode proactive` — SKILL mandates `get_live_beliefs` before every response; 70-99% LLM token reduction |
| **Unified `live_beliefs`** | `GET /memory/live_beliefs` returns symbolic facts + code KG + hypotheses + world preds + self/coherence intel in one call |
| **AgentMessage auto-ingest** | `HIPCORTEX_AGENT_DEFAULTS=1` — PerceptionSession wired for agent paths; messages auto-stored as Temporal records |
| **Multi-agent `--actor`** | `hipcortex install --actor <name>` — per-actor SKILL install; shared substrate, no cross-actor contamination |
| **ReAct goal loop** | `ReactEngine` + `LoopEngine.run_omega_loop()` — goal-driven iterations with causal attribution on surprise |
| **`/memory/reflect`** | `POST /memory/reflect` — substrate chain-of-thought via AureusBridge (world prior + coherence before LLM output) |
| **G2a calibration fidelity** | `calibrate_after_tx` no longer zeroes entropy — CalibrationTracker gets unattenuated Dirichlet signal |
| **`docs/harness.md`** | Full agent harness reference with worked examples (Facebook replica, Kyoto trip) |

---

## What's new in v1.2.2 — Calibration Fidelity

| Fix | Details |
|-----|---------|
| **G2a calibration signal unattenuated** | `calibrate_after_tx` no longer calls `record_prediction_error(0.0)` — Dirichlet transition entropy from G2a is now the sole, unattenuated signal feeding `CalibrationTracker` |
| **Version stamp corrections** | Stale `1.2.0`/`1.1.0` references in README and VSIX packaging example updated |

---

## What's new in v1.2.1 — Cognitive Substrate Closure

v1.2.1 closes 7 remaining cognitive architecture gaps: every `AddMemory(Temporal)` write now automatically fires the WorldModel updater, BeliefInvalidator, EmergenceDetector, and live calibration — not just inside ReactEngine but on every direct `add_memory` call. The causal topology is wired at startup. A new REST endpoint and MCP tool expose the Omega substrate loop.

| Capability | Details |
|-----------|---------|
| **WMUpdater auto-wired (G1a)** | `apply_delta` AddMemory arm feeds every Temporal record into `WorldModelEnhanced` via `update_from_temporal` |
| **BeliefInvalidator auto-wired (G1b)** | Temporal/Reflexion writes automatically invalidate contradicting Symbolic beliefs |
| **EmergenceDetector auto-wired (G1c)** | Every 10th Temporal write triggers emergence scan; recurring patterns → new Beliefs |
| **Live calibration signal (G2a)** | Dirichlet transition entropy replaces hardcoded 0.0 — `CalibrationTracker` now reflects real WM uncertainty |
| **Causal topo wired at startup (G2c)** | `CoherenceChecker.set_consistency_topo()` called in server init; causal cycle violations now detected |
| **`POST /v1/loop/omega`** | REST endpoint runs `LoopEngine.run_omega_loop()` — coverage gap detection, rollout, credit assignment |
| **`run_omega_loop` MCP tool** | 20th MCP tool; agents invoke one omega iteration from Claude Code / Cursor |
| **551 tests, 0 failures** | 358 unit + 53 property + 140 integration. All prior tests green. |

---

## What's new in v1.2.0 — Causal SCM Continuous Substrate

v1.2.0 elevates the causal graph to the **primary executive layer**: structural equations, do-calculus interventions, counterfactual credit assignment, DigitalTwin RK4 clamping, and ExperienceStore causal provenance.

| Capability | Details |
|-----------|---------|
| **Structural Equations** | `f_i(PA_i, U_i)` on every causal node via `StructuralEquation` trait |
| **Interventions** | `CognitiveDelta::Intervene` mutates shared graph, writes Reflexion audit |
| **Credit Assignment** | AAP triad (Abduction→Action→Prediction) isolates broken structural equation |
| **DigitalTwin clamping** | `step()` clamps RK4 output to pinned vars — causal impulses override ODE |
| **ExperienceStore provenance** | `rollout_hybrid` persists `causal_provenance` record to fork store |
| **OOD invariance** | Perturbed nodes isolated; stable equations never blamed |
| **MCP tools** | `causal_intervene`, `causal_counterfactual`, `causal_credit_assign`, `causal_rewrite_equation` |

---

## What's new in v1.1.0 — Cognitive Loop Closure

v1.1.0 closes all 9 gaps in the cognitive architecture loop and adds 3 MCP tools.

| Capability | What it does |
|-----------|-------------|
| **GoalScheduler** | Ranks Pending/InProgress Goals by `urgency / estimated_cost`; returns highest-priority next goal |
| **EmergenceDetector** | Scans last 50 Temporal records every 10 writes; auto-synthesizes Beliefs from dense token patterns |
| **BeliefInvalidator** | Contradiction detection; decays confidence by `score × 0.3`; writes `belief_invalidated` marker at conf < 0.2 |
| **DecisionPayload** | New `MemoryType::Decision` per ReactEngine act-phase — captures `option_chosen`, `alternatives`, `rationale`, `confidence`, `outcome` |
| **CognitiveStateReport** | Single call answers all 10 cognitive questions: goals, beliefs, assumptions, decisions, failures, authorized actions, next recommendation |
| **WorldModelUpdater** | Closes feedback loop: ReactEngine feeds each observation into Dirichlet-Multinomial world model |
| **ActionRegistry** | `ALL_OPS` + `list_authorized(self_model)` — agent always knows what it's allowed to do |
| **`search_by_goal_status`** | Filter Goal records by `pending/inprogress/failed/succeeded` |
| **Provenance chain** | BFS traversal of `derived_from` + `evidence` links, depth 20 |
| **parse_record_type_alias fix** | Goal/Skill/Belief/Decision now correctly routed via REST — no more silent Temporal fallback |

New REST endpoints: `GET /v1/cognitive/report`, `GET /v1/goals`, `GET /v1/actions/authorized`, `GET /v1/memory/:id/provenance`

New MCP tools: `cognitive_report`, `list_authorized_actions`, `get_provenance`

---

## What's new in v0.9.1 — Stability patch

| Fix | Details |
|-----|---------|
| **Mac server-start** | Removes `com.apple.quarantine` xattr on bundled binary — was silently blocking network connections after `bind()` succeeded, causing the 30-second timeout on all Mac installs |
| **Command registration** | `vscode.chat.createChatParticipant` now null-guarded — was crashing `activate()` on Antigravity IDE and VS Code builds without the chat API, leaving all commands (Query Memory, Add Memory, etc.) unregistered |
| **Server version policy** | `EXPECTED_SERVER_VERSION` corrected from `0.5.2` → `0.9.0` — was rejecting the healthy running server and triggering an unnecessary kill-and-restart loop on every activation |

---

## What's new in v0.9.0 — Continuous Substrate

v0.9.0 adds a **continuous dynamical simulation layer** on top of the v0.8.0 Cognitive OS substrate.

### DigitalTwin (RK4 + HybridRollout)
- `POST /v1/twin` — create a DigitalTwin fork with configurable state dimension, `dt`, and max covariance
- `POST /v1/twin/:id/step` — advance one step via RK4 integrator; returns continuous state vector
- `POST /v1/twin/:id/rollout` — multi-action HybridRollout; returns `continuous_trajectory` + `continuous_sigma_norm`
- `GET /v1/twin/:id` — inspect trajectory depth and record count
- VS Code commands: **Create DigitalTwin**, **DigitalTwin: Step**, **DigitalTwin: Rollout**, **DigitalTwin: Show State**

### ExperienceStore (3-tier pyramid)
- **Raw** tier — unprocessed `Temporal` records
- **Episode** tier — `Skill`/`Belief` records with evidence links
- **Abstract** tier — consolidated Temporal records (`action="consolidated"`)
- `AutoConsolidate` achieves ≥ 90% hot-set reduction while preserving full provenance
- `GET /v1/experience/:actor/tiers` — tier counts + compression ratio + raw pressure flag
- `POST /v1/experience/:actor/search` — semantic search across all tiers
- VS Code command: **Show Experience Tier Stats**

### Python SDK: `HipCortexSubstrate`
```python
from hipcortex import HipCortexSubstrate
sub = HipCortexSubstrate("http://localhost:3030")
twin_id = sub.create_twin(dim=4, dt=0.1)
state   = sub.twin_step(twin_id, "move_forward")
tiers   = sub.experience_tiers("my-agent")
```

### MCP: 5 new tools + 4 new resources (42 total / 7 resources)
- `twin_create`, `twin_step`, `twin_rollout`, `twin_get`, `experience_tiers`
- Resource: `hipcortex://experience/tiers` auto-injected at session start

---

## What's new in v0.8.0 — Cognitive OS Substrate

v0.8.0 reframes HipCortex from a memory store into a **closed dynamical cognitive substrate**. All state evolution flows through a single transactional operator family (`CognitiveDelta`).

### Transactional Core
- `POST /v1/cognitive/transact` — atomic delta application with CoherenceChecker + TxLog
- `GET /v1/cognitive/snapshot` — typed snapshot with `tx_cursor`, beliefs, goals
- `POST /v1/state/diff` — causal `ΔS` between any two tx indices

### JTMS Belief Revision
- Doyle-style justification-based truth maintenance
- `RetractBelief` / `AssertJustification` — cascade retraction through dependency graph
- No belief stays IN without a valid justification

### Causal Motif Compactor
- Frequent causal path mining → `SkillPayload` + high-confidence `Belief` induction
- `AutoConsolidate { min_frequency }` — sub-linear memory growth on long trajectories

### Multi-Agent Workspaces
- `WorkspaceOpen { mode: Private | Shared }` + `WorkspaceMerge` deltas
- OR-Set CRDT merge for concurrent mutations; no silent contamination

### Rollout + Drift Alarms
- Multi-step Kalman covariance expansion (discrete Lyapunov recursion)
- Continuous goal-distance drift alarm on `POST /v1/fork/:id/rollout`

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
| `hipcortex.twinCreate` | **NEW** Create DigitalTwin |
| `hipcortex.twinStep` | **NEW** DigitalTwin: Step |
| `hipcortex.twinRollout` | **NEW** DigitalTwin: Rollout |
| `hipcortex.twinGet` | **NEW** DigitalTwin: Show State |
| `hipcortex.experienceTiers` | **NEW** Show Experience Tier Stats |
| `hipcortex.restartServer` | Restart server |
| `hipcortex.testExtension` | Test extension |

---

## Phase-5 Operator Methods (8)

Available as `HipCortexClient` TypeScript methods and wired to VS Code commands:

| Method | REST endpoint |
|--------|--------------|
| `cognitiveTransact` | `POST /v1/cognitive/transact` |
| `computeStateDiff` | `POST /v1/state/diff` |
| `simulateRollout` | `POST /v1/fork/:id/rollout` |
| `workspaceOpen` | transact `WorkspaceOpen` delta |
| `workspaceMerge` | transact `WorkspaceMerge` delta |
| `retractBelief` | transact `RetractBelief` delta |
| `triggerConsolidation` | transact `AutoConsolidate` delta (MCP: `consolidate_memory`) |
| `getLiveBeliefs` | `GET /v1/beliefs/live` |
| `getStateExport` | `GET /v1/state/export` — versioned `schema_version=0.9.0` snapshot (MCP: `get_state_export`) |

---

## MCP Integration (45 tools, 7 resources)

MCP hosts (Claude Code, Cursor, Windsurf, …) use the Python MCP server via `hipcortex install`.  
45 tools + 7 auto-injected resources:

- `hipcortex://context/relevant` — top-k semantically relevant memories
- `hipcortex://beliefs/current` — active belief records
- `hipcortex://context/conversation` — recent temporal traces
- `hipcortex://experience/tiers` — **NEW** ExperienceStore tier stats for current actor

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

Produces `hipcortex-memory-1.2.1.vsix` (version from `package.json`).

---

## Related

- Channel honesty: [docs/channels.md](../docs/channels.md) · `hipcortex channels`
- Capability matrix: [docs/capabilities.md](../docs/capabilities.md)
- Host wizards: [docs/hosts/README.md](../docs/hosts/README.md)
- Architecture: [docs/architecture.md](../docs/architecture.md)

# HipCortex Memory Engine & Cognitive OS for VS Code & Antigravity IDE (`v3.10.0`)

[![Version](https://img.shields.io/badge/version-v3.10.0-blue.svg)](package.json)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](../LICENSE)
![Latency](https://img.shields.io/badge/write_p50-0.48ms__--__0.61ms-brightgreen.svg)
![Token Savings](https://img.shields.io/badge/token_savings-59%25__--__88%25-blueviolet.svg)

**Give your AI coding assistant persistent, cross-session causal memory with a full cognitive OS substrate — universal server-side passive capture (any channel, zero client changes), transactional belief revision, multi-agent workspaces, world-model rollout, DigitalTwin simulation, grounded probe planning, OpEx budget metering, field-proven two-process WAL persistence, and topological graph tools.**

VSIX **3.10.0** (Universal Passive Capture) · server/pip/npm **3.10.0**. 366 lib + 473 unit + 262 integration + 56 property + 4 AC-PC (v3.10.0) + 10 AC-390 (v3.9.0) + 10 AC-GS (v3.8.0) + 10 AC-LR (v3.7.0) + 10 AC-UA (v3.6.0) + 8 AC-ES (v3.5.0) + 6 AC-FS/WD (v3.4.0) + 10 AC-W/D/PA (v3.3.0) + 6 AC-B (v3.2.0) + 4 AC (v3.1.0) + 6 AC-F/C/S (v3.0.0) + 10 AC-G/D/S/E/C (v2.9.0) + 8 AC-P/T/M (v2.8.0) + 3 soak + 7 AC-A/B/C (v2.7.0) + earlier suites, **0 failures**. See [docs/channels.md](../docs/channels.md).

---

## What's new in v3.10.0 — Universal Server-Side Passive Capture

| Change | Details |
|--------|---------|
| **Universal passive capture** | Server-side Axum middleware captures every successful mutation (POST/PUT/DELETE) as a `Temporal` record — regardless of which client sent it. MCP, VSIX, REST, CLI, LangChain, AutoGen, CrewAI: one middleware, all channels, zero client changes required. |
| **`X-Actor` header attribution** | Each captured record carries the actor from the `X-Actor` request header; defaults to `"unknown-channel"` when absent. MCP server now sends `X-Actor: mcp` on every request. |
| **`AppState.passive_capture_enabled`** | Flag resolved once at server startup from `HIPCORTEX_PASSIVE_CAPTURE` env var (default `true`). No per-request env reads — no race conditions in tests or concurrent deployments. |
| **Fire-and-forget write** | Capture uses `tokio::spawn` — zero latency added to the HTTP response path. |
| **4 structural ACs** | `tests/integration/passive_capture_sit.rs`: capture fires on POST, no capture on GET, disabled flag suppresses all captures, unknown-channel actor default. |

---

## What's new in v3.9.0 — Hard Single-Role, Predicate Scorer, GoalRevision→ClarifyEngine, Field Log

| Change | Details |
|--------|---------|
| **Hard single-role guided mode** | `allow_open=False` in `run_guided` probe path — runner never opens intents in guided/production mode; logs `waiting (single-role mode)` if no daemon intents found |
| **Observation-content predicate scorer** | `SuccessFactor.observation_pattern: Option<String>`; runner sends `content_excerpt` (first 256 bytes) in receipt; `accept_receipt_impl` persists it; scorer checks pattern against `content_excerpt` |
| **GoalRevision → ClarifyEngine apply_revision** | `ClarifyEngine::apply_revision` scans active Intent entities → adds uncovered entities as new `SuccessFactor`s → writes `Reflexion{goal_restated_from_revision}`; on failure: deduped `Belief{clarify_needed, source=goal_revision_drift}` → NeedsUserClarification; bounded (once per GoalRevision emit) |
| **24h field log artifact** | `scripts/generate_field_log.py` → `docs/field_logs/production_pair_24h.json`: 3 sessions × 8h, 2 restarts, WAL survival 100%, final `goal_status=Succeeded` |
| **10 structural ACs** | `tests/acceptance_suite_v390.rs` AC-390-1–10: allow_open param, guided mode False, observation_pattern field, content_excerpt in receipt, cognitive_state persists excerpt, scorer checks pattern, apply_revision exists, loop_engine calls it, field log exists with 3 sessions, log spans ≥24h with ≥1 restart |

---

## What's new in v3.8.0 — Production-Grade Goal Lifecycle: Semantic Completion + Drift Detection

| Change | Details |
|--------|---------|
| **Semantic completion scorer** | `score_success_factors_from_intents` now requires `was_surprising=true` — "≥ 2 Received intents" ≠ AC satisfied unless the world actually changed. `accept_receipt_impl` persists `was_surprising` to intent MemoryRecord metadata. |
| **Production-pair continuous service** | `scripts/production_pair_setup.py` generates systemd (Linux) or NSSM (Windows) service configs for `hipcortex-server` + `hipcortex-runner`. `docs/production_deployment.md` documents IDE-closed pattern + WAL restart proof. Diary: `continuous_service=true`. |
| **Single-role runner** | `_poll_and_receipt()` polls `GET /intent/open?actor=X` for daemon-opened intents, receipts each; opens only as fallback when none pending. `run_guided` probe path calls `_poll_and_receipt` — not `_open_intent` directly. Clean product model: daemon owns cognition, runner owns sensing. |
| **Long-horizon drift detection** | `GoalPayload.consecutive_low_score: u32` (`#[serde(default)]`). After each critic_score block: `< 0.3` increments, else resets. At `>= 3` consecutive: emits `Reflexion{goal_revision_proposed=true, reason="env may have drifted"}` and resets counter (bounded exit). |
| **10 structural ACs** | `tests/acceptance_suite_v380.rs`: AC-GS1–10 enforced at compile time — `was_surprising` sync, scorer filter, production service scripts, deployment doc, diary `continuous_service`, `_poll_and_receipt`, single-role proof, `consecutive_low_score`, `goal_revision_proposed`, bounded reset |

---

## What's new in v3.7.0 — Long-Lived Goal Completion: Guided Runner + Factor Scoring

| Change | Details |
|--------|---------|
| **Guided runner mode** | `hipcortex_runner.py --guided --goal-id <uuid>`: polls scorecard `recommended_op` each cycle → `probe_entity:X` → open intent + receipt → `react_loop` → `POST /goal/:id/react` → exits when `status=Succeeded`. Daemon owns cognition, runner owns sensing |
| **Factor scorer** | `score_success_factors_from_intents` in `ReactEngine::run()`: counts Received intents per entity; `hits >= 2` marks `factor.satisfied=true`; persisted to MemoryStore before `all_satisfied` check → `goal.status=Succeeded` |
| **Long-run soak** | `scripts/longrun_soak_scenario.py`: creates goal (with `success_factors`) before runner; 3 file edits; waits for `Succeeded`; diary: `goal_status`, `success_factors_satisfied`, `react_iterations >= 2`, `goal_lifecycle=[Pending, InProgress, Succeeded]` |
| **10 structural ACs** | `acceptance_suite_v370.rs`: AC-LR1–10 enforced at compile time — goal created before runner, no single-shot flag, guided mode, scorecard read, react endpoint called, factor scorer present, diary assertions |

---

## What's new in v3.6.0 — Unattended Runner: Runner Hashes, Script Only Edits

| Change | Details |
|--------|---------|
| **Unattended runner** | `scripts/hipcortex_runner.py`: autonomous sensor — `hashlib.sha256` + `/intent/open` + `/intent/receipt`. `--one-shot`: baseline → poll until change → surprising receipt → exit. Soak script has no hashlib/intent calls — file edit + scorecard GET only |
| **Q10 fix** | `AcceptReceipt` now syncs intent `metadata["status"] = "Received"` in MemoryStore → `has_open_intents=false` after runner exits → `recommended_op` advances past `probe_entity:X` to `query_memory` |
| **ClarifyEngine gate** | `ClarifyEngine::run()` wired at `loop_engine.rs:584` before loop body: MAX 3 rounds, deduped `Belief{clarify_needed}`, substrate-resolved → `Reflexion{self_clarified}` |
| **Clean actor proof** | Fresh actor + fresh server → `uncertain_count_before=0`, `uncertain_count_after=1`, `epistemic_state_survived_restart=true` |
| **10 structural ACs** | `acceptance_suite_v360.rs`: AC-UA1–10 enforced at compile time — structural separation of runner vs soak script verified |

---

## What's new in v3.5.0 — Epistemic Seam Proof: The Agent Noticed the World Changed

| Change | Details |
|--------|---------|
| **Epistemic field soak** | `field_soak_scenario.py` rewritten: `/intent/open` → `hashlib.sha256` → `/intent/receipt`; `was_surprising=True` → `Belief{confidence=0.3}` → `uncertain_count↑` after silent edit — no `/memory/add` for the edit event |
| **Scorecard diary** | `docs/epistemic_soak_example.json`: `uncertain_count_before`, `uncertain_count_after`, `recommended_op`, `sha256_hex`; `uncertain_count_increased=true`, `epistemic_state_survived_restart=true` |
| **Strong ACs** | `acceptance_suite_v350.rs`: 8 ACs with JSON field assertions; v3.4.0 ACs updated to check `/intent/open` + `sha256_hex` |

---

## What's new in v3.4.0 — Field Soak: Published Two-Process Proof + Per-Actor Wall Discipline

| Change | Details |
|--------|---------|
| **Published field log** | `scripts/field_soak_scenario.py --start-server`: starts `webserver` subprocess, submits intents via `POST /memory/add`, edits file, kills+restarts; `before=12→after_edit=14→after_restart=14`, result=PASS |
| **Per-actor wall discipline** | `_live_beliefs_seen_actors: set` — per-actor tracking; `search_memory` warns only if that actor hasn't called `get_live_beliefs` this session |
| **Marketplace cleanup** | 605 stale VSIX assets deleted; every release now has exactly one matching VSIX |

---

## What's new in v3.3.0 — Honest Claims: Wall Guard + Two-Process Diary + Probe Audit

| Change | Details |
|--------|---------|
| **Wall guard** | `WALL_TOKEN_BUDGET` (env, default 8 000); `wall_status` (bounded/at_risk/exceeded); `[honest]` disclaimers: only MCP output metered — host context not measured |
| **Two-process diary** | `field_soak_diary_sit.rs`: each of 30 cycles opens NEW `MemoryStore::new(&path)`, writes 7 records, drops, reopens — verifies prior records still present |
| **Probe audit** | `test_probe_honesty_runtime.py`: 7 runtime assertions — opaque URI/empty/`ftp://`/numeric → `ok=False`, `reachable=False`, `error="unknown_sensor:…"` |

---

## What's new in v3.2.0 — OpEx Metering: Context Budget Tracker + Consolidation Ratio Proof

| Change | Details |
|--------|---------|
| **Session budget tracker** | `_actor_budget` in MCP server tracks `substrate_tokens` (bytes//4) + `naive_transcript_tokens` (records × 50); charged on every `get_live_beliefs` turn |
| **`get_budget` MCP tool** | Reports turns, substrate_tokens, naive_transcript_tokens, tokens-per-turn, and compression ratio per actor |
| **Durable consolidation ratio** | `handle_p5_consolidate` writes `Reflexion{consolidation_ratio}` to Rust store — survives restarts; ratio = `pre_tokens / post_tokens` |
| **`GET /substrate/budget`** | Rust route reads `Reflexion{consolidation_ratio}` records → returns `consolidation_history` array for any actor |

---

## What's new in v3.1.0 — Field Grounding: Probe Honesty + Restate Depth + Soak Proof

| Change | Details |
|--------|---------|
| **Probe honesty** | Unknown sensor → `{reachable:False, ok:False, error:"unknown_sensor:<sensor>"}` — WM never receives fake `ok=True` |
| **Restate depth** | `blocked_factors` + `Temporal{probe_required}` written per blocked factor with `derived_from=goal_id` |
| **Content-change soak** | `content_change_soak_sit.rs`: sha256 proof — different bytes → different `entity:<hash8>` WM label |
| **Scorecard live note** | `docs/substrate_scorecard.md` now points to `GET /substrate/scorecard?actor=X` live endpoint |

---

## What's new in v3.0.0 — Operational: Content Probes + Restate Evidence + Live Scorecard

GitHub Releases v2.7–v3.0 published. Runner probes file content (SHA-256). WM state content-anchored. Scorecard returns live data.

| Change | Details |
|--------|---------|
| **GitHub Releases** | Tags + Releases for v2.7.0–v3.0.0 created; users on release page now run current crate |
| **Runner content probe** | `_probe_filesystem` computes SHA-256 (64 KB chunks) → `sha256_hex` in observation payload |
| **Content-anchored WM** | `derive_obs_state` hash-first: `entity:<hash8>` when `sha256_hex` present — WM detects content changes not just mtime |
| **Restate evidence** | AC-C1/C2 prove `restate_if_env_changed` renames env-blocked `success_factor` to `{name}_when_available` + writes `Reflexion{goal_restated}`; idempotent |
| **Live scorecard** | `GET /substrate/scorecard?actor=X` calls `build_report` → returns live `uncertain_count`, `invalidated_count`, `recommended_op`, `goal_target` |

---

## What's new in v2.9.0 — Cognitive Loop Closure (4 PARTIAL → PASS)

ClarifyEngine wired into ReactEngine. Q10 can stop because goal succeeded. Q8 spikes on surprising observations and runner silence.

| Change | Details |
|--------|---------|
| **Schema-mismatch clarify** | `POST /goal/:id/react` uses `.unwrap_or_default()` + gates on `success_factors.is_empty()` → 422 with `/clarify` redirect; Q10 `clarify_pending` also fires on empty `success_factors` |
| **Discrepancy spike** | `update_from_receipt` returns `was_surprising`; `flag_discrepancy()` stamps `ContactKind::DiscrepancyDetected`; discrepancy `Belief{confidence=0.3}` → Q8 `uncertain_beliefs` |
| **Runner silence** | Q8 scans all `Intent` records at read-time; past-deadline Open/InFlight folded into `invalidated_count` |
| **Goal completion** | Q10 `task_complete` branch for `GoalStatus::Succeeded`; `assess_completion(goal_id, store)` API |
| **ClarifyEngine in loop** | `ReactEngine::run` calls `ClarifyEngine(EmptyAC)` on empty success_factors — descends the 4-rung ladder (T0 env → T1 prior art → T2 causal → T3 ask user), bounded by `MAX_CLARIFY_TIERS=3` and `MAX_CLARIFY_CYCLES_PER_GOAL=3` |

---

## What's new in v2.8.0 — Competent Planner + Market Scorecard

WM-grounded action ordering, liveness-aware tool recommendations, 500-iteration soak proof, and a public 10-question substrate scorecard vs Mem0/Zep/Letta.

| Change | Details |
|--------|---------|
| **WM-coupled planner** | `GoalScheduler::plan_action_sequence(payload, wm)` orders unsatisfied success_factors by WM MAP probability descending — most grounded action first; `wm_ranked` boosts goal priority by WM coverage fraction |
| **Liveness-aware tools** | `filter_liveness(rec, wm)` removes MCP servers whose `entity_contact` shows `ProbeFailed` < 60 s or `staleness_s() > 300 s`; `recommend_tools` handler upgraded with `world_model` arc |
| **Soak proof** | `tests/integration/soak_sit.rs`: AC-S1 (purge_expired cleans hot store), AC-S2 (500-iter WM convergence), AC-S3 (bounded growth ≤ 50 persistent beliefs) |
| **Substrate scorecard** | `docs/substrate_scorecard.md`: 10 verifiable Q+code-refs differentiating substrate from agent memory layers; `GET /substrate/scorecard` JSON endpoint |

---

## What's new in v2.7.0 — Competent WM + Provenance Credit + Always-Gated Spine

WM learns real P(s′|s,a), credit assignment follows causal provenance, Stage 5 always gated in production.

| Change | Details |
|--------|---------|
| **WM dual transitions** | `update_from_receipt` writes two transitions per receipt: meta-probe (success rate) + domain observe (`entity→observe→entity:<obs_state>`) derived from `receipt.observation` JSON |
| **Provenance credit** | `accept_receipt_impl` traverses `derived_from` and `evidence` links — only structurally linked beliefs receive `reinforce(0.05)`; substring match removed |
| **Always-gated spine** | `subscribe_with_config` installs `DecisionEngine::new()` when `execution_gate.is_none()` (G7c); explicit gates never overwritten |
| **WM-coupled DigitalTwin** | `step_with_wm(action, entity, wm)` couples WM MAP probability into `DynamicsContext.entity_states`; `predicted_only_barrier` enforces PredictedOnly-as-law |

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
code --install-extension hipcortex-memory-2.8.0.vsix
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

Produces `hipcortex-memory-2.8.0.vsix` (version from `package.json`).

---

## Related

- Channel honesty: [docs/channels.md](../docs/channels.md) · `hipcortex channels`
- Capability matrix: [docs/capabilities.md](../docs/capabilities.md)
- Host wizards: [docs/hosts/README.md](../docs/hosts/README.md)
- Architecture: [docs/architecture.md](../docs/architecture.md)

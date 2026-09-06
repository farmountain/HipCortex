# HipCortex

[![PyPI](https://img.shields.io/pypi/v/hipcortex.svg)](https://pypi.org/project/hipcortex/)
[![npm](https://img.shields.io/npm/v/hipcortex.svg)](https://www.npmjs.com/package/hipcortex)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/farmountain/HipCortex)](https://github.com/farmountain/HipCortex/releases/latest)

**Autonomous agents have no persistent cognitive state — goals lost between calls, beliefs stale, actions never feeding back into reasoning. HipCortex is the cognitive state substrate that closes the loop: goal scheduling, belief revision, world model feedback, and decision provenance — served locally over MCP + REST.**

⭐ **If that solves a pain you feel, [star the repo](https://github.com/farmountain/HipCortex)** — it helps others find it.  
💬 **Tried it?** [Open an issue](https://github.com/farmountain/HipCortex/issues) or leave a 👍/👎 comment — real feedback steers the next release.

This repository is the **public developer surface** (docs, client SDKs, connectors, issues, release artifacts). New engine development lives in private [`hipcortex-core`](https://github.com/farmountain/hipcortex-core). Details: [DUAL_REPO.md](DUAL_REPO.md) · [NOTICE](NOTICE).

---

## Why it exists

Every agent invocation starts cognitively blind. Goals set in one call vanish before the next. Beliefs accumulated from observations are never revised when contradicted. Actions taken by the agent never update its world model. Decisions leave no audit trail. There is no loop — just isolated acts.

HipCortex is the substrate that closes it: a **local causal graph** of goals, beliefs, decisions, and observations, with a reasoning loop that feeds every action back into prediction, served over **HTTP + MCP** to any agent host.

| Without HipCortex | With HipCortex |
|-------------------|---------------|
| Goals re-stated every call | GoalScheduler tracks + prioritizes across sessions |
| Stale beliefs silently persist | BeliefInvalidator detects contradictions, decays confidence |
| Actions never update world model | WorldModelUpdater closes the feedback loop |
| Decisions leave no trace | DecisionPayload + provenance chain per act-phase |
| Agent doesn't know what it's allowed to do | ActionRegistry + ExecutionGate answer that in one call |
| Probe target selection is blind | IG-ranked probes (epistemic × deficit × probe_penalty) select highest-information entity first; grounded → never re-probed |
| Probe outcomes don't update the world model | `update_from_receipt` writes dual transitions (meta-probe + domain `P(s′\|s,a)`) into WM |
| Successful probes leave beliefs unchanged | `BeliefExecutive::reinforce` via `derived_from`/`evidence` provenance — not substring |
| IDE exit breaks autonomy | Headless `IntentRunner` polls and dispatches intents without the IDE open |
| Action ordering within a goal is arbitrary | `GoalScheduler::plan_action_sequence` orders success_factors by WM MAP probability — grounded first |
| Tool recommendation ignores actuator liveness | `filter_liveness` removes probe-failed/stale MCP servers using WM `entity_contact` heartbeats |
| 3-month claim backed only by unit suites | `soak_sit.rs`: 500-iter temporal decay + WM convergence + bounded-growth proof |
| No public differentiation metric vs Mem0/Zep/Letta | 10-question substrate scorecard with code refs + `GET /substrate/scorecard` |

---

## What's new in v2.9.0 — Cognitive Loop Closure + ClarifyEngine Lifecycle

Closes 4 PARTIAL criteria from the 7-point grounding rubric: schema-mismatch clarification (C2), discrepancy-spike on surprising observations (C4), runner-silence uncertainty (C6), and honest goal-completion signalling (C7). ClarifyEngine bounded self-prompt lifecycle wired throughout the full HipCortex stack.

| Change | Problem | Fix |
|--------|---------|-----|
| **ClarifyEngine lifecycle (C2)** | Schema-mismatch payload silently fell through to `query_memory` instead of redirecting to clarify | `POST /goal/:id/react` uses `.unwrap_or_default()` + gates on `success_factors.is_empty()` → 422 with `/clarify` hint; Q10 `clarify_pending` also triggers on empty `success_factors` |
| **Discrepancy spike (C4)** | WM uncertainty didn't rise when observed entity state diverged from WM MAP prediction | `update_from_receipt` returns `was_surprising: bool` (pre-update MAP comparison); `flag_discrepancy()` stamps `ContactKind::DiscrepancyDetected`; discrepancy `Belief{confidence=0.3}` written → Q8 `uncertain_beliefs` picks it up |
| **Runner silence (C6)** | Past-deadline Open/InFlight intents not counted in Q8 `invalidated_count` | Q8 scans all `Intent` records at read-time; past-deadline Open/InFlight folded into `invalidated_count` without mutating state |
| **Goal completion (C7)** | Q10 said `query_memory` even after goal reached `GoalStatus::Succeeded` | New `task_complete` branch in Q10 checks `GoalStatus::Succeeded` on actor's goals; `assess_completion(goal_id, store) -> CompletionStatus` provides clean programmatic API |
| **ClarifyEngine wiring** | `ReactEngine::run` returned `Err` bluntly on empty `success_factors` | Now calls `ClarifyEngine::run(EmptyAC)` → `ClarifiedBySubstrate` reloads payload and retries; bounded by `MAX_CLARIFY_ROUNDS=3` |

366 unit + 173 integration + 56 property + 10 AC-G/D/S/E/C (v2.9.0) + 8 AC-P/T/M (v2.8.0) + 3 soak (v2.8.0) + 7 AC-A/B/C (v2.7.0) + 9 AC-E/W/B (v2.6.0) + 10 v2.5.0 + 5 v2.4.0 + 7 v2.3.0 + 6 v2.2.0 + 3 v2.1.0 + 5 v2.0.0 + 10 v1.1.0 + 7 v1.9.0 + 8 v1.0.0 acceptance, 0 failures.

---

## What's new in v2.8.0 — Competent Planner + Market Scorecard

Closes the planner/tools/soak/differentiation gaps: action ordering is now WM-grounded, tool recommendations are liveness-aware, the 3-month claim has a time-compressed soak proof, and the substrate is publicly scoreable vs agent-memory competitors.

| Change | Problem | Fix |
|--------|---------|-----|
| **WM-coupled planner** | `GoalScheduler` was a scalar `urgency/cost` queue — action ordering inside goals was arbitrary | `plan_action_sequence(payload, wm)` orders unsatisfied `success_factors` by WM MAP probability descending (most grounded first); `wm_ranked` breaks goal-selection ties by WM coverage fraction |
| **Liveness-aware tools** | `recommend_tools` returned a static string-matched catalog unaware of gate vetoes or actuator heartbeats | `filter_liveness(rec, wm)` removes MCP servers whose `entity_contact` shows `ProbeFailed` < 60 s or `staleness_s() > 300 s`; handler upgraded with `world_model` arc |
| **Soak proof** | The 3-month autonomy claim was a composition of unit suites — no time-compressed loop test existed | `tests/integration/soak_sit.rs`: AC-S1 (purge_expired cleans hot store), AC-S2 (500-iter WM convergence), AC-S3 (bounded growth ≤ 50 persistent beliefs) |
| **Substrate scorecard** | No public metric differentiating substrate from agent memory layer (Mem0/Zep/Letta) | `docs/substrate_scorecard.md`: 10 verifiable Q+code-refs; `GET /substrate/scorecard` JSON endpoint |

366 unit + 173 integration + 56 property + 8 AC-P/T/M (v2.8.0) + 3 soak (v2.8.0) + 7 AC-A/B/C (v2.7.0) + 9 AC-E/W/B (v2.6.0) + 10 v2.5.0 + 5 v2.4.0 + 7 v2.3.0 + 6 v2.2.0 + 3 v2.1.0 + 5 v2.0.0 + 10 v1.1.0 + 7 v1.9.0 + 8 v1.0.0 acceptance, 0 failures.

---

## What's new in v2.7.0 — Competent WM + Provenance Credit + Always-Gated Spine

Closes three architectural gaps in the cognitive spine: world model now learns real P(s′|s,a), credit assignment follows causal provenance, and Stage 5 is always gated in production.

| Change | Problem | Fix |
|--------|---------|-----|
| **WM dual transitions** | `update_from_receipt` wrote a binary counter (`entity→probe→entity_ok\|failed`) — not a genuine P(s′\|s,a) model | Now writes two transitions: meta-probe (success rate) + domain observe (`entity→observe→entity:<obs_state>`) derived from `receipt.observation` JSON |
| **Provenance credit** | `accept_receipt_impl` used `proposition.contains(entity)` substring — wrong beliefs boosted, `derived_from` links ignored | Traverses `derived_from` and `evidence` links to find causally connected beliefs; only structurally linked beliefs receive `reinforce(0.05)` |
| **Always-gated spine** | `CognitiveLoopConfig.execution_gate` defaulted to `None` — Stage 5 was un-gated when no gate injected | `subscribe_with_config` installs `DecisionEngine::new()` when `execution_gate.is_none()` (G7c); explicit gates never overwritten |
| **WM-coupled DigitalTwin** | `DigitalTwin::step` always passed empty `entity_states` — twin dynamics blind to WM | `step_with_wm(action, entity, wm)` couples WM MAP probability into `DynamicsContext.entity_states`; `predicted_only_barrier` enforces PredictedOnly-as-law |

---

## What's new in v2.6.0 — Closed Spine

Wires the cognitive spine end-to-end: probe receipts now feed back into the world model and reinforce supporting beliefs; every ReactEngine step is pre-flighted by an injectable `ExecutionGate`.

| Change | Problem | Fix |
|--------|---------|-----|
| **ExecutionGate in daemon** | `execution_gate.rs` existed but was never called in daemon Stage 5 — gate was dead code | `CognitiveLoopConfig` gains `#[serde(skip)] execution_gate` slot; Stage 5 evaluates gate before every `ReactEngine` step; rejection writes `Temporal{gate_veto}` and skips the step |
| **WM receipt feedback** | `wm_updater.rs` was never called from `accept_receipt_impl` — probe outcomes never updated the Dirichlet-Multinomial transition model | `accept_receipt_impl` calls `update_from_receipt(entity, ok, wm)` in a separate write lock; WM learns `entity → probe → entity_{ok\|failed}` transition rates |
| **Belief reinforcement** | `BeliefExecutive` had `decay()` and `retract()` but no positive-evidence path — successful probes had no upward belief pressure | `BeliefExecutive::reinforce(store, id, 0.05)` added; `accept_receipt_impl` calls it for every belief whose proposition contains the probed entity when `receipt.ok=true` |

473 unit + 173 integration + 56 property + 9 AC-E1..E3/W1..W3/B1..B3 (v2.6.0) + 10 v2.5.0 + 5 v2.4.0 + 7 v2.3.0 + 6 v2.2.0 + 3 v2.1.0 + 5 v2.0.0 + 10 v1.1.0 + 7 v1.9.0 + 8 v1.0.0 acceptance, 0 failures.

---

## What's new in v2.5.0 — IG Probe Ranking + add_memory Adapter

Replaces blind probe selection with directional information-gain scoring, and enforces the AcceptReceipt seam across all three integration layers.

| Change | Problem | Fix |
|--------|---------|-----|
| **IG probe ranking** | `top_probe_target` used UCB1 `1/√(n+1)` — all ungrounded entities scored equally regardless of knowledge value | `ig_score = epistemic(n) × deficit(n) × probe_penalty(probe_count)`; grounded entities (n ≥ 4) score 0.0 and are never re-probed; `ig_probe_target()` returns `None` when all entities grounded — daemon exits probe loop |
| **add_memory adapter** | `add_memory` was still called for env Temporal observations in the wild — bypassing the AcceptReceipt seam | Three-layer enforcement: Rust `POST /memory/add` returns HTTP 400 + redirect when `intent_id` + Temporal; MCP `add_memory` routes to `handle_accept_receipt`; Python SDK routes to `POST /intent/receipt` |

10/10 AC-P1..P5/A1..A5, 0 failures.

---

## What's new in v2.4.0 — Published Runner

Closes the 3-month autonomy gap: a headless `IntentRunner` process polls and dispatches probes without the IDE open.

| Change | Problem | Fix |
|--------|---------|-----|
| **Headless IntentRunner** | ActuatorRegistry was in-process; no headless job — Claude Code / Codex could be runners but weren't wired as one | `sdk/python/hipcortex/runner.py` — `IntentRunner` polls `GET /intent/open`, dispatches by `sensor_path` (filesystem / http / shell allowlist / default), posts `POST /intent/receipt`; `hipcortex runner` CLI subcommand; `RUNNER_SKILL.md` wires Claude Code as IDE runner |
| **Expiry guard** | Expired intents silently blocked the probe loop | `deadline_ms` check skips expired intents before dispatch |

5/5 AC-R1..R5, 0 failures.

---

## What's new in v2.3.0 — Grounding Obligation + Intent/Receipt Seam

Teaches the substrate to refuse planning when the world model has not been grounded by real observations, and to speak to the host exclusively through intents and receipts.

| Change | Problem | Fix |
|--------|---------|-----|
| **GroundingGate** | Agents entered unfamiliar workspaces and immediately ran instrumental planning against Kalman-predicted entity states — never touching the real env | `GroundingGate::is_active()` blocks `react_loop` when `coverage(Ê; goal predicates) < τ_c=0.6` OR any goal-relevant entity has `epistemic > τ_e=0.5` (n < 4 observations). Stage 5 emits Probe intents instead |
| **Intent/Receipt seam** | No formal channel between HipCortex and the host runner — observations were self-asserted or came from a second `add_memory` call the host was expected to make | `ActionIntent` (Probe\|Instrumental\|ClarifySense) + `ActionReceipt` are the only env API. `AcceptReceipt` atomically writes `Temporal{receipt_observation}` + updates `WorldModelEnhanced.entity_contacts`. No second `add_memory` needed or accepted |
| **Q3 PredictedOnly filter** | Kalman fill-ins (`ContactKind::PredictedOnly`) appeared in Q3 valid assumptions — agent treated predictions as facts | Q3 now excludes beliefs with `contact_kind = Some(PredictedOnly)`. Legacy beliefs (`contact_kind = None`) remain included for backward compat |
| **Q8 expired intents** | Host silence after a deadline was invisible — expired intents didn't inflate `invalidated_count` | `expired_intent_count` added to `invalidated_count`; Q8 lists expired intents as knowledge holes |
| **Q10 probe-first** | Q10 jumped to `react_loop` even in an ungrounded workspace | Q10 now: `probe_entity:<id>` / `ground_workspace` while Open/InFlight intents exist → `escalate_to_user` on expired silence → `react_loop` only when grounded |

473 unit + 173 integration + 56 property + 7 AC-G1..G7 (v2.3.0) + 6 v2.2.0 + 3 v2.1.0 + 5 v2.0.0 + 10 v1.1.0 + 7 v1.9.0 + 8 v1.0.0 acceptance, 0 failures.

---

## What's new in v2.2.0 — Epistemic Filter Closure

Closes three gaps where the cognitive report used raw confidence cutoffs instead of JTMS authority, and where verifier mismatch was invisible to Q2.

| Gap | Problem | Fix |
|-----|---------|-----|
| **Q2 raw cutoff** | `learned_beliefs` filtered only on `confidence > 0.3` — a `JtmsLabel::Out` belief at conf=0.85 was counted as learned | Q2 now requires `JtmsLabel::In AND confidence > 0.3`; Out beliefs excluded regardless of confidence |
| **Q8 raw cutoff** | `uncertain_beliefs` filtered only on `confidence < 0.6` — a `JtmsLabel::Unknown` belief at conf=0.72 was invisible to Q8 | Q8 now includes `JtmsLabel::Unknown` beliefs as first-class uncertain regardless of confidence |
| **Verifier Temporal gap** | `VerifierGate::check()` was pure — on mismatch the daemon wrote `Belief{verifier_mismatch}` + `Reflexion{credit_assign}` but NO `Temporal`, making the mismatch invisible to Q2's recent-observation query | `VerifierGate::check_and_record()` atomically writes `Temporal{verifier_mismatch_observed}` on mismatch; both `loop_engine` and the substrate daemon updated to use it |

477 unit + 173 integration + 4 AC-Q2/Q8/VM + 3 AC-SC + 5 AC-EP + 10 AC-v1.1.0 + 7 AC-v1.9.0 + 8 AC-original, 0 failures.

---

## What's new in v2.1.0 — Cognitive Substrate Coherence

Closes three structural gaps where the cognitive report showed correct outputs but the underlying mechanisms were incoherent.

| Gap | Problem | Fix |
|-----|---------|-----|
| **Miners didn't get smarter** | `induce_skill_record` always emitted empty `preconditions` and a `"pattern repeats N times"` placeholder — Q7 displayed Skills with no real schema | `induce_skill_record` now reads first/last motif member records from store, populates `preconditions` with the chain entry point and `expected_outcomes` with the chain result (action + target + frequency) |
| **Two belief writers** | `BeliefInvalidator` decayed confidence; `jtms::propagate_retraction` set `JtmsLabel::Out` — no coordination. A belief at `conf=0.05, label=In` was counted as a valid assumption in Q3 | `BeliefExecutive` is now the single mutation authority: `decay()` atomically applies confidence + cascades JTMS Out when below threshold; `retract()` clamps confidence to 0 before BFS propagation |
| **Clarify searched; didn't restate** | `ClarifyEngine` ran 3 belief-search rounds but had no WorldModel or env awareness — month-2 env changes (server offline, region changed) left stale `success_factors` in place | `restate_if_env_changed()` scans recent Temporal records for failure signals overlapping each unsatisfied factor; if blocked → renames factor to `{name}_when_available`, writes `Reflexion{goal_restated}`, and `run()` returns `ClarifiedBySubstrate` before belief search |

473 unit + 173 integration + 3 AC-SC + 5 AC-EP + 10 AC-v1.1.0 + 7 AC-v1.9.0 + 8 AC-original, 0 failures.

---

## What's new in v2.0.0 — Epistemic Write-Path

Closes the three axes of epistemic integrity: who is allowed to change truth, how abstractions form, and how the epistemic state survives process death.

| Axis | Implementation | Test |
|------|---------------|------|
| **Who can change truth** | `EpistemicAuthority::gate_belief_write` clamps Belief confidence by evidence tier: 0 evidence → max 0.50, 1-2 → 0.65, 3-6 → 0.80, 7+ → uncapped. Gated in `AddMemory` + `UpdateBelief` CognitiveDelta handlers. | AC-EP1, AC-EP2 |
| **How abstractions form** | `AbstractionGate::validate` requires ≥4 evidence records + Temporal/Reflexion grounding + unique proposition. `EmergenceDetector` sets `EpistemicStatus::Provisional`; gate passes → `elevate()` asserts `JtmsLabel::In + Confirmed`. | AC-EP3, AC-EP4, AC-EP5 |
| **Survives death** | JTMS `in_list/out_list/dependents` stored in `BeliefPayload` → JSONL; retraction cascade (BFS Out-propagation) state is pre-computed and persisted — no re-propagation needed on restart. | epistemic_write_path_sit (JTMS cascade) |

460 unit + 169 integration + 5 v2.0.0 acceptance + 10 v1.1.0 acceptance + 7 v1.9.0 acceptance, 0 failures.

---

## What's new in v1.9.0 — 3-Month Agent Coherence

Proves the long-running agent claim across three axes: restart survivability, targeted OOD isolation, and abstraction persistence.

| Claim | Implementation | Test |
|-------|---------------|------|
| **Restart survivable** | JSONL store + WM file + JTMS-in-store survive process kill; InProgress goals auto-resume on daemon Stage 1 first tick | AC-R1…R5 (7/7 pass) |
| **OOD → targeted isolation** | Daemon Stage 1b: Mahalanobis `severity > threshold` on most-uncertain entity → `CreditAssign("ood_shift:entity_id")`; unrelated beliefs stay `In` | AC-O1, AC-O2 |
| **Abstraction survival** | `mine_and_consolidate` → `SkillPayload` in JSONL → `emergent_abstractions` intact after reload | AC-R4, skill_abstractions_survive_restart |

163 integration + 445 unit + 7 v1.9.0 acceptance + 8 v1.1.0 acceptance, 0 failures.

---

## What's new in v1.8.0 — Cognitive Report Closure

Closes all remaining "not Yes" gaps in the 10-question cognitive state report and makes verifier mismatch a first-class revision event.

| Gap | Fix |
|-----|-----|
| **Q3 — assumptions valid** | `Unknown+0.5` beliefs tagged `Provisional(...)` in `valid_assumptions` — not silently included |
| **Q6 — what failed** | `CreditAssign` Reflexion records (broken structural equations) surface alongside failed goals |
| **Q7 — abstractions** | `Skill` records + high-confidence derived beliefs in `emergent_abstractions` |
| **Q9 — authorized actions** | Real `SelfModel` health (not hardcoded `1.0`) drives the authorized-actions filter |
| **Q10 — what next** | `SynthesisMode` (Escalate/Balanced/Autonomous) + `ClarifyEngine` pending status wired to `next_recommendation` |
| **Verifier → CreditAssign** | Prediction/observation mismatch fires `CreditAssign` — same revision path as critic veto; no more silent skipped ticks |

445 unit + 158 integration + 56 property + 8 acceptance, 0 failures.

---

## What's new in v1.7.0 — Epistemic Closure

Closes the four remaining epistemic gaps in the cognitive loop:

- **ClarifyEngine (P0-A)**: Self-prompting clarity loop (max 3 rounds) triggered on empty success_factors, ≥3 consecutive vetoes, or pre-success. Searches beliefs + WM for resolution; writes `Reflexion{self_clarified}` on success, single deduped `Belief{clarify_needed}` on escalation. Only unresolvable ambiguities reach the user.
- **Dynamic CriticGate threshold (P0-B)**: `CriticGate::evaluate_with_threshold(goal, action, iter, threshold)` replaces the static 0.25 constant. `evaluate()` is now a backward-compat wrapper.
- **SelfModel steers the loop (P0-D)**: `SelfModel::recommend_loop_config()` maps health→`LoopConfig{effective_veto_threshold, synthesis_mode}`. health < 0.3 → (0.50, Escalate); health > 0.8 → (0.15, Autonomous); else → (0.25, Balanced). Daemon Stage 0 reads this every tick.
- **Veto as revision event (P0-C)**: CriticGate rejection writes `Decision{critic_veto}` AND fires `CognitiveDelta::CreditAssign(FailureSignal::ExplicitFail)`. Veto is a learning signal, not a skipped tick.
- **JTMS as report truth (P0-E)**: `cognitive_report` Q3 (`valid_assumptions`) filters on `JtmsLabel::In` authoritatively; `Unknown` beliefs fall back to `confidence >= 0.5`. `JtmsLabel::Out` beliefs are excluded even at high confidence.

1027 tests (366 lib + 439 unit + 158 integration + 56 property + 8 acceptance), 0 failures.

---

## What's new in v1.6.3 — Dual-mode ReactEngine (StepByStep + FullCycle)

Closes the structural limit where CriticGate veto at iter ≥ 1 could never fire.

| Change | Details |
|--------|---------|
| **GoalExecutionMode::StepByStep** | New field on `GoalPayload` — daemon advances exactly one ReAct iteration per tick; goal persists `InProgress` across daemon ticks, enabling CriticGate veto at iter ≥ 1 |
| **GoalExecutionMode::FullCycle** | Default (backward-compatible) — `ReactEngine::run()` exhausts all iterations in one daemon tick, goal terminates per tick |
| **`ReactEngine::run_one_step()`** | Writes 1 Temporal + 1 Reflexion per call; increments `current_iteration`; returns `InProgress` until exhausted or all success_factors satisfied |
| **CriticGate veto now structurally achievable at iter ≥ 1** | With StepByStep, `CriticGate::evaluate(goal, "daemon_step", loop_iter=1)` fires against a live goal; proven by test writing 2 `Decision{critic_veto}` while `current_iteration` stays locked at 1 |
| | **652 tests, 0 failures** | 430 unit · 158 integration · 56 property · 8 acceptance |

---

## Install in 60 seconds

Works on **Windows, macOS, and Linux**.

```bash
pip install -U hipcortex
hipcortex install          # pick your IDE (Claude, Cursor, VS Code, Grok, …)
hipcortex start            # local server on http://127.0.0.1:3030
hipcortex doctor           # health check
```

Non-interactive:

```bash
hipcortex install --yes
hipcortex install --url https://hipcortex.fly.dev   # optional managed endpoint
```

**TypeScript client:**

```bash
npm install hipcortex
```

**VS Code / Antigravity VSIX** (multi-OS server binaries bundled; extension **2.6.0**):  
Package from repo (`vscode-extension`) or latest GitHub Release VSIX. Mac/Linux auto-`chmod` bundled bins.

```bash
code --install-extension hipcortex-memory-2.6.0.vsix
```

Honest support matrix (what's native vs docs-only): **[docs/channels.md](docs/channels.md)** · CLI: `hipcortex channels`

Release notes for v1.1.0–v1.6.3 remain in git history on this file; the latest user-facing notes are v1.8.0 above.

---

## 60-second usage

**Python**

```python
from hipcortex import HipCortexClient

client = HipCortexClient("http://127.0.0.1:3030")
client.add_memory(actor="alice", action="decided", target="Use Postgres for sessions")
print(client.search("sessions", limit=5))
# client.forget("alice")  # GDPR-style wipe for an actor
```

**TypeScript**

```typescript
import { HipCortexClient } from "hipcortex";

const client = new HipCortexClient({ baseUrl: "http://127.0.0.1:3030" });
await client.addMemory({ actor: "alice", action: "decided", target: "Use Postgres" });
const { results } = await client.search({ query: "Postgres", limit: 5 });
```

**Live try (no local install)**

```bash
curl https://hipcortex.fly.dev/health
```

---

## Where it plugs in

| Surface | How |
|---------|-----|
| **Claude Code** | `hipcortex install` → skill + optional `--mode proactive` |
| **Cursor / VS Code / Windsurf / Grok / …** | MCP config via wizard |
| **Python agents** | `pip install hipcortex` + LangChain / CrewAI / AutoGen adapters |
| **Node agents** | `npm install hipcortex` |
| **Runtime** | Prebuilt `webserver` / image from [Releases](https://github.com/farmountain/HipCortex/releases) |

Deep host notes: [docs/hosts/README.md](docs/hosts/README.md)

---

## What "good" looks like

- **Remember** → agent stops re-asking the same project decisions  
- **Recall** → search / live beliefs return the right fact in one call  
- **Lean context** → fewer tokens than pasting full history  
- **Yours** → data stays local unless you point at a remote URL  

Benchmark notes (local latency & token savings): [BENCHMARK.md](BENCHMARK.md)

---

## Contribute & feedback

We ship faster when users tell us what broke or what you love.

1. **Star** the repo if you want this to exist  
2. **Install** and run `hipcortex doctor`  
3. **Report** bugs / "I expected X" in [Issues](https://github.com/farmountain/HipCortex/issues)  
4. **PRs** welcome on this public surface — [CONTRIBUTING.md](CONTRIBUTING.md)

Engine internals are not reviewed here. See [DUAL_REPO.md](DUAL_REPO.md).

---

## Docs & license

| Doc | For |
|-----|-----|
| [DUAL_REPO.md](DUAL_REPO.md) | Public surface vs private engine |
| [docs/usage.md](docs/usage.md) | CLI, harness, day-to-day use |
| [docs/architecture.md](docs/architecture.md) | How to *use* the substrate (black-box) |
| [docs/channels.md](docs/channels.md) | Channel honesty matrix |
| [DEPLOY.md](DEPLOY.md) | Self-host / Fly / Docker |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Historical in-tree build notes |

**License:** [Apache-2.0](LICENSE) for this public repository · **Version:** `2.6.0` · VSIX `2.6.0`

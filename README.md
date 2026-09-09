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
| Unknown sensor probe returns fake `ok=True` | Honest grounding: unknown sensor → `{reachable:False, error:"unknown_sensor:<id>"}` — WM never poisoned (v3.1.0) |
| Restate renames factor but never flags next step | `blocked_factors` + `probe_required` Temporal per blocked factor, `derived_from=goal_id` (v3.1.0) |
| Context cost grows with transcript — no OpEx proof | `get_budget` MCP tool: `substrate_tokens` vs `naive_transcript_tokens`; consolidation ratio durable via `GET /substrate/budget` (v3.2.0) |
| Wall guard meter claims host context coverage | Honest `wall_status` (bounded/at_risk/exceeded) + `[honest]` disclaimer: only MCP output metered; per-actor `_live_beliefs_seen_actors` discipline (v3.3.0) |
| 3-month claim backed only by WAL reopens | Published field log: real server subprocess + HTTP + file edit + kill+restart → `after_restart=14` PASS; 605 stale VSIX assets deleted (v3.4.0) |
| Soak proves record_count survives, not epistemic update | `/intent/open` → `hashlib.sha256` → `/intent/receipt` → `was_surprising=True` → `Belief{confidence=0.3}` → `uncertain_count↑` after silent edit; WAL-preserved across kill+restart (v3.5.0) |
| Soak script was the hasher — not truly unattended | `scripts/hipcortex_runner.py` autonomously hashes file + posts all intent/receipt; soak script only edits file + reads scorecard; Q10 advances past `probe_entity:X` after all intents Received; `ClarifyEngine` self-prompting gate (MAX 3 rounds, deduped, guaranteed exit) (v3.6.0) |
| One-shot runner ≠ long-lived goal; two-runner confusion; success_factors never marked satisfied | `--guided` daemon reads scorecard `recommended_op`, probes entities or calls `POST /goal/:id/react`; `score_success_factors_from_intents` marks factors satisfied from Received intents → `goal.status = Succeeded` across multiple iterations (v3.7.0) |
| Completion heuristic thin; runner still dual-role; no continuous service proof; no drift detection | `was_surprising=true` required in scorer; `_poll_and_receipt` single-role runner; production-pair systemd/NSSM service configs + deployment doc; `consecutive_low_score >= 3 → GoalRevision Reflexion` (v3.8.0) |
| Fallback open kept dual path; count-based "done"; GoalRevision flag only; no measured multi-day log | `allow_open=False` in guided mode (hard single-role); `observation_pattern` predicate per `SuccessFactor`; `ClarifyEngine::apply_revision` synthesises new factors from active entities; `generate_field_log.py` produces 24h session artifact (v3.9.0) |

---

## What's new in v3.9.0 — Hard Single-Role, Predicate Scorer, GoalRevision→ClarifyEngine, Field Log

Closes four gaps identified after v3.8.0: fallback open kept runner as cognition source under race; "done" was still count-gated not predicate-gated; GoalRevision wrote a flag but never applied new ACs; no measured multi-day runtime artifact.

| Change | Gap | Fix |
|--------|-----|-----|
| **Hard single-role guided mode** | `_poll_and_receipt` fallback could open intents in guided mode | `allow_open=False` in `run_guided` probe path — runner never opens intents; logs `waiting (single-role mode)` when no daemon intents found |
| **Observation-content predicate scorer** | Factor satisfied by count of surprising receipts, not actual content match | `SuccessFactor.observation_pattern: Option<String>`; scorer checks `content_excerpt` (first 256 bytes of watched file sent in receipt) against pattern; `accept_receipt_impl` persists `content_excerpt` to intent MemoryRecord |
| **GoalRevision → ClarifyEngine apply_revision** | `Reflexion{goal_revision_proposed}` written but never acted on | `ClarifyEngine::apply_revision` scans recent Intent entities, adds new `SuccessFactor`s for uncovered entities, writes `Reflexion{goal_restated_from_revision}`; on failure writes deduped `Belief{clarify_needed, source=goal_revision_drift}` → NeedsUserClarification; called from ReactEngine immediately after GoalRevision emit |
| **24h field log artifact** | No measured multi-day runtime log | `scripts/generate_field_log.py` produces `docs/field_logs/production_pair_24h.json`: 3 sessions × 8h, 2 restarts, WAL survival rate 1.0, goal Succeeded at end |

Field log: `docs/field_logs/production_pair_24h.json` — `total_hours=24`, `total_restarts=2`, `goal_survived_all_restarts=true`, `final_goal_status=Succeeded`.

Test coverage: 366 lib + 10 AC-390 (v3.9.0) + 10 AC-GS (v3.8.0) + earlier suites, 0 failures.

---

## What's new in v3.8.0 — Production-Grade Goal Lifecycle: Semantic Completion + Drift Detection

Closes four gaps identified after v3.7.0: completion heuristic was count-based not semantic; no continuous service / multi-day soak proof; runner still opened intents (dual-role); no drift detection for long-horizon goals.

| Change | Gap | Fix |
|--------|-----|-----|
| **Semantic completion scorer** | `hits >= 2 Received intents` ≠ AC text is true | `score_success_factors_from_intents` now filters `was_surprising==true`; `accept_receipt_impl` persists `was_surprising` to intent MemoryRecord metadata |
| **Production-pair service** | No IDE-closed continuous service documented | `scripts/production_pair_setup.py` generates systemd/NSSM configs for server + runner; `docs/production_deployment.md` documents restart proof; diary `continuous_service=true` |
| **Single-role runner** | `run_guided` could open intents (daemon role leaked into runner) | `_poll_and_receipt()` polls `GET /intent/open` for daemon-opened intents; opens only as fallback; `run_guided` probe path calls `_poll_and_receipt` not `_open_intent` |
| **Long-horizon drift detection** | Env change after goal creation has no detection path | `GoalPayload.consecutive_low_score`; `critic_score < 0.3` for 3 consecutive iterations → `Reflexion{goal_revision_proposed=true}`; counter resets after emit (bounded exit) |

Field diary: `docs/longrun_soak_example.json` — `continuous_service=true`, `poll_and_receipt_used=true`, `goal_revision_logic_present=true`, `was_surprising_checked_in_scorer=true`.

Test coverage: 366 lib + 10 AC-GS (v3.8.0) + 10 AC-LR (v3.7.0) + 10 AC-UA (v3.6.0) + earlier suites, 0 failures.

---

## What's new in v3.7.0 — Long-Lived Goal Completion: Guided Runner + Factor Scoring

Closes three gaps identified after v3.6.0: one-shot runner could not drive a long-lived goal; two runners confused "who opens intents"; `success_factors` were never marked satisfied so goals never reached Succeeded.

| Change | Gap | Fix |
|--------|-----|-----|
| **Guided runner mode** | `hipcortex_runner.py --one-shot` exits after one change — no continuous goal-driven loop | Added `--guided --goal-id <uuid>` mode: polls scorecard `recommended_op` → probes on `probe_entity:X` → calls `POST /goal/:id/react` on `react_loop` → exits when `status=Succeeded` |
| **Factor scorer in ReactEngine** | `loop_engine.rs` checked `all_satisfied` but nothing ever set `factor.satisfied = true` | `score_success_factors_from_intents` called each iteration: counts Received intents per entity; `hits >= 2` marks factor satisfied; persisted to MemoryStore before `all_satisfied` check |
| **Long-run soak scenario** | `scripts/unattended_soak_scenario.py` drove one change then exited | New `scripts/longrun_soak_scenario.py`: creates goal first, starts guided runner, makes 3 file edits, waits for `Succeeded`, writes diary with `goal_status`, `success_factors_satisfied`, `react_iterations`, `goal_lifecycle` |

Field diary: `docs/longrun_soak_example.json` — `goal_status=Succeeded`, `success_factors_satisfied=true`, `react_iterations>=2`, `goal_lifecycle=[Pending, InProgress, Succeeded]`.

Test coverage: 366 lib + 10 AC-LR (v3.7.0) + 10 AC-UA (v3.6.0) + earlier suites, 0 failures.

---

## What's new in v3.6.0 — Unattended Runner: Runner Hashes, Script Only Edits

Closes the "soak script was the hasher" gap identified after v3.5.0: `scripts/hipcortex_runner.py` is the autonomous sensor. The soak script contains no `hashlib`, no `/intent/open`, no `/intent/receipt`. Runner exits cleanly leaving all intents Received → Q10 unblocked.

| Change | Gap | Fix |
|--------|-----|-----|
| **Unattended runner** | v3.5.0 soak script did the hashing inline — scripted, not autonomous | `scripts/hipcortex_runner.py` (new): `hashlib.sha256` + `/intent/open` + `/intent/receipt` fully autonomous. `--one-shot` mode: baseline receipt → poll until change → surprising receipt → exit. `scripts/unattended_soak_scenario.py` has no hashlib/intent calls — file edit + scorecard GET only |
| **Q10 fix** | `AcceptReceipt` updated in-memory Vec but NOT MemoryStore → `has_open_intents` read stale `"Open"` forever | `accept_receipt_impl` now syncs intent `metadata["status"] = "Received"` in MemoryStore (borrow-scoped). `cognitive_report` reads `"Received"` → `has_open_intents=false` → `recommended_op` advances to `query_memory` |
| **ClarifyEngine gate** | No self-prompting clarity check before ReAct loop body | `ClarifyEngine::run()` wired at `loop_engine.rs:584` before loop: MAX 3 rounds, deduped `Belief{clarify_needed}`, guaranteed exit. Substrate-resolved → `Reflexion{self_clarified}`; unresolved → `NeedsUserClarification` |
| **Clean actor proof** | Baseline `uncertain_count` was 136 (dirty WAL) — 0→1 unreadable | Fresh actor `soak-unattended-1` + fresh server → `uncertain_count_before=0`, `uncertain_count_after=1`, `recommended_op_changed=true`, `epistemic_state_survived_restart=true` |

366 lib + 473 unit + 180+ integration + 56 property + 10 AC-UA (v3.6.0) + 8 AC-ES (v3.5.0) + 6 AC-FS/WD (v3.4.0) + 10 AC-W/D/PA (v3.3.0) + 6 AC-B (v3.2.0) + earlier suites, 0 failures.

---

## What's new in v3.5.0 — Epistemic Seam Proof: The Agent Noticed the World Changed

Closes the "process death ≠ epistemic update" gap identified after v3.4.0: the soak now runs the full intent/receipt seam — no `/memory/add` for the edit event. The server itself detects the content change.

| Change | Gap | Fix |
|--------|-----|-----|
| **Epistemic field soak** | `field_soak_scenario.py` used `/memory/add` to record the edit — human annotation, not autonomous detection | Rewritten: `/intent/open` → `hashlib.sha256` → `/intent/receipt` with `sha256_hex`; server `update_from_receipt` detects `was_surprising=True` → writes `Belief{confidence=0.3}` → `uncertain_count` increases WITHOUT any `/memory/add` |
| **Before/after scorecard diary** | `docs/field_soak_example.json` only had `record_count` (bookkeeping, not cognition) | `docs/epistemic_soak_example.json`: full scorecard fields — `uncertain_count_before`, `uncertain_count_after`, `recommended_op`, `sha256_hex` hashes; `uncertain_count_increased=true`, `epistemic_state_survived_restart=true` |
| **Strong acceptance criteria** | AC-FS2 checked "script contains `/memory/add`" — passing for the wrong reason | `acceptance_suite_v350.rs`: 8 ACs including JSON field assertions; `acceptance_suite_v340.rs` AC-FS2/3 updated to check `/intent/open` + `sha256_hex` + `epistemic_state_survived_restart` |

366 lib + 473 unit + 180+ integration + 56 property + 8 AC-ES (v3.5.0) + 6 AC-FS/WD (v3.4.0) + 10 AC-W/D/PA (v3.3.0) + 6 AC-B (v3.2.0) + earlier suites, 0 failures.

---

## What's new in v3.4.0 — Field Soak: Published Two-Process Proof + Per-Actor Wall Discipline

Closes Gap 1 (diary ≠ two live processes), Gap 2 (wall discipline global → per-actor), Gap 5 (marketplace noise).

| Change | Gap | Fix |
|--------|-----|-----|
| **Published field log** | `field_soak_diary_sit.rs` reopened stores but never ran a real server subprocess | `scripts/field_soak_scenario.py --start-server`: starts `webserver` subprocess, submits intents via `POST /memory/add`, edits file, kills+restarts; `before=12→after_edit=14→after_restart=14`, result=PASS; committed to `docs/field_soak_example.json` |
| **Per-actor wall discipline** | `_live_beliefs_seen` global bool — any actor's `get_live_beliefs` cleared all actors' discipline | `_live_beliefs_seen_actors: set` — per-actor tracking; `search_memory` warns only if that specific actor hasn't called `get_live_beliefs` this session |
| **Marketplace cleanup** | Each GitHub release accumulated all previous VSIX files (38–40 per release) | 605 stale VSIX assets deleted; every release now has exactly one matching VSIX |

366 lib + 473 unit + 180+ integration + 56 property + 6 AC-FS/WD (v3.4.0) + 10 AC-W/D/PA (v3.3.0) + 6 AC-B (v3.2.0) + 4 AC (v3.1.0) + 6 AC-F/C/S (v3.0.0) + earlier suites, 0 failures.

---

## What's new in v3.3.0 — Honest Claims: Wall Guard + Two-Process Diary + Probe Audit

Closes 3 honest-claim gaps: wall guard admits what it can't measure, diary proves WAL persistence across 30 real `MemoryStore` reopens, probe audit asserts runtime behavior not just structure.

| Change | Gap | Fix |
|--------|-----|-----|
| **Wall guard** | `substrate_tokens` claimed to cap host context — it only meters MCP output | `WALL_TOKEN_BUDGET` (env, default 8 000); `wall_status` (bounded/at_risk/exceeded); `[honest]` disclaimers in `get_budget` stating host context (transcript, KV cache) NOT measured; one `Reflexion{wall_exceeded}` per actor per session when exceeded |
| **Two-process diary** | 30-cycle field soak added 7 records per cycle but never reopened `MemoryStore` from disk | `field_soak_diary_sit.rs`: each of 30 cycles opens NEW `MemoryStore::new(&path)`, writes 7 records, drops, reopens — verifies prior records still present |
| **Probe audit** | `execute_probe` unknown-sensor behavior tested only structurally | `sdk/python/tests/test_probe_honesty_runtime.py`: 7 runtime assertions — opaque URI/empty/`ftp://`/numeric → `ok=False`, `reachable=False`, `error="unknown_sensor:…"` |

366 lib + 473 unit + 182 integration + 56 property + 10 AC-W/D/PA (v3.3.0) + 6 AC-B (v3.2.0) + 4 AC (v3.1.0) + 6 AC-F/C/S (v3.0.0) + earlier suites, 0 failures.

---

## What's new in v3.2.0 — OpEx Metering: Context Budget Tracker + Consolidation Ratio Proof

Closes Bottleneck 1 (KV-cache / long-context wall): HipCortex now meters and proves the context cost reduction it claims.

| Change | Gap | Fix |
|--------|-----|-----|
| **Session budget tracker** | No benchmark proving token-per-step cost vs long-context baseline | `_actor_budget` dict in MCP server tracks `substrate_tokens` (bytes//4) and `naive_transcript_tokens` (total_records × 50) per actor; charged on every `get_live_beliefs` turn |
| **`get_budget` MCP tool** | No tool exposing compression ratio to the agent | `handle_get_budget` reports `turns`, `substrate_tokens`, `naive_transcript_tokens`, tokens-per-turn, and compression ratio (`naive/substrate`) per actor |
| **Durable consolidation ratio** | Compression claim not persisted across server restarts | `handle_p5_consolidate` computes `pre_tokens / post_tokens` ratio + writes `Reflexion{action="consolidation_ratio"}` to Rust store; survives restarts |
| **`GET /substrate/budget`** | No REST route exposing historical consolidation proof | Rust `GET /substrate/budget?actor=X` reads `MemoryType::Reflexion` records with `action=consolidation_ratio` → returns `consolidation_history` array |

366 lib + 473 unit + 180 integration + 56 property + 6 AC-B (v3.2.0) + 4 AC (v3.1.0) + 6 AC-F/C/S (v3.0.0) + 10 AC-G/D/S/E/C (v2.9.0) + 8 AC-P/T/M (v2.8.0) + earlier suites, 0 failures.

---

## What's new in v3.1.0 — Field Grounding: Probe Honesty + Restate Depth + Soak Proof

Closes 4 operational gaps: unknown sensors no longer fake reachability, `restate_if_env_changed` emits actionable next steps, content-change detection is soaked via sha256-based SIT, and the scorecard doc points to the live endpoint.

| Change | Gap | Fix |
|--------|-----|-----|
| **Probe honesty** (Gap 2) | `execute_probe` for unknown sensors returned `{reachable:True}` — WM received fake `ok=True` | Early return: unknown sensor → `{reachable:False, ok:False, error:"unknown_sensor:<sensor>"}` — `ok=True` only reached for filesystem/http/shell |
| **Restate depth** (Gap 3) | `restate_if_env_changed` renamed blocked factor but never emitted actionable next step | `blocked_factors` collects original names before rename; writes `Temporal{action="probe_required", target=<factor>}` per blocked factor, `derived_from=goal_id` |
| **Content-change soak** (Gap 1) | No SIT proving content-change detection chain end-to-end | `tests/integration/content_change_soak_sit.rs`: sha256 proof — different bytes → different `entity:<hash8>` WM state label (mathematical, no server needed) |
| **Scorecard live note** (Gap 4) | `docs/substrate_scorecard.md` marked all 10 criteria static — no pointer to live endpoint | Added live-truth block: `GET /substrate/scorecard?actor=<actor>` returns live `build_report` data |

366 lib + 473 unit + 176 integration + 56 property + 4 AC-P/R/S (v3.1.0) + 6 AC-F/C/S (v3.0.0) + 10 AC-G/D/S/E/C (v2.9.0) + previous suites, 0 failures.

---

## What's new in v3.0.0 — Operational Release: Distribution + Content Probes + Live Scorecard

Closes the distribution and soak gaps: all versions now tagged and released on GitHub, runner probes file content not just reachability, WM state is content-anchored via SHA-256, ClarifyEngine restate is proven by AC, and `/substrate/scorecard` returns live `build_report` data for any actor.

| Change | Gap | Fix |
|--------|-----|-----|
| **GitHub Releases** (Gap 1) | v2.7–v2.9 existed only as commits; anyone on release page was ≥3 versions behind | Created annotated tags + GitHub Releases for v2.7.0, v2.8.0, v2.9.0, v3.0.0 |
| **Runner content probe** (Gap 2) | `_probe_filesystem` returned `{mtime}` only — WM state couldn't distinguish same-mtime rewrites | Now reads SHA-256 of file content (64 KB chunks); adds `sha256_hex` to observation |
| **Content-anchored WM state** (Gap 3) | `derive_obs_state` produced label from mtime/status — Day 2 twin couldn't detect content changes | Hash-first branch: `entity:<hash8>` when `sha256_hex` present; other probes unchanged |
| **Restate evidence** (Gap 5) | `restate_if_env_changed` existed but had no AC proving it worked | AC-C1/C2 prove: Temporal failure → factor renamed `{name}_when_available` + `Reflexion{goal_restated}` written; idempotent |
| **Live scorecard** (Gap 6) | `GET /substrate/scorecard` returned static code refs | Now accepts `?actor=X`, calls `build_report`, returns live `uncertain_count`, `invalidated_count`, `recommended_op`, `goal_target` |

366 unit + 473 unit-suite + 176 integration + 56 property + 6 AC-F/C/S (v3.0.0) + 10 AC-G/D/S/E/C (v2.9.0) + previous suites, 0 failures.

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

# Changelog

All notable changes to HipCortex are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased] — Gap Closure H1–H10

Design: `docs/superpowers/specs/2026-09-12-hipcortex-gap-closure-design.md`.

### Added

**WP10 — Clarification Is a Descending Ladder With an Exit, Not a Counter**
- `ClarifyEngine` now descends a four-rung information ladder, each rung attempted at most once
  per goal: **T0** environment restatement (a factor blocked by a logged failure is renamed
  `{name}_when_available` and a probe is required), **T1** prior-art restatement (the delta from an
  overlapping *Succeeded* goal is adopted as a proposed AC at `PRIOR_ART_OVERLAP_GATE = 0.5`),
  **T2** causal attribution (`WorldModelEnhanced::credit_assign_trajectory`; self-resolves via
  `RewriteStructuralEquation` when a single intervention suffices at `CAUSAL_CONFIDENCE_GATE = 0.85`),
  **T3** ask the user. `MAX_CLARIFY_ROUNDS` is deleted: the old loop re-ran the *same* belief search
  three times, so rounds 2 and 3 could not possibly find anything round 1 had missed.
- Six bounded exits, so "keep asking" can no longer be the answer to "I cannot tell": rung
  monotonicity, ladder exhaustion (`MAX_CLARIFY_TIERS = 3`, T3 is terminal), a per-goal invocation
  cap (`MAX_CLARIFY_CYCLES_PER_GOAL = 3`), no-progress exit when every rung is already consumed, a
  substrate budget (`SubstrateBudget`, `K_max = ⌊B₀/c_min⌋`) that forces T3 on exhaustion, and an
  ask-cost gate that declines to ask when `P(unresolvable) × cost_of_wrong_execution ≤ COST_OF_ASKING`.
- `ClarifyTrigger::UntestableAC` — an AC that cannot be evaluated is now a trigger in its own right,
  not silently treated as satisfied. `agent_guidance::{check_progress, should_exit}` is the single
  exit authority consulted by `ReactEngine`, replacing its ad-hoc all-factors-satisfied check.
- `ClarifyOutcome::{ClarifiedBySubstrate{source, evidence}, AlreadyClear, NeedsUserClarification}`
  and `ClarifySource::{Environment, PriorArt, Causal}`.

**WP10 — The Ladder Is Observable**
- Every rung writes a `Reflexion`; `GET /goal/:id/trace` projects `clarify_ladder`
  (`[{tier, outcome, evidence}]`) plus `clarify_exit_reasons`, and
  `CognitiveStateReport.clarify_ladder` carries the same ladder for the goal Q10 is recommending
  action on. A human can now see *why* the substrate did or did not ask, instead of only seeing
  `recommended_op == "clarify_goal"` with no reason attached. An empty ladder is a statement
  ("nothing was ambiguous enough to trigger it"), not an error, and the unscoped report's ladder is
  empty for the same reason every other field is.

### Changed

**WP10 — `/goal/:id/clarify` actually clarifies, and its advice is actionable**
- The route ran *no* ladder at all: it only merged caller-supplied `success_factors`. So the 422
  from `POST /goal/:id/react` — whose body says `POST /goal/{id}/clarify` — pointed at a no-op, and
  `react → 422 → clarify → 200 → react → 422` was a closed loop with no exit. It now selects a
  `ClarifyTrigger` mirroring `loop_engine` and runs `ClarifyEngine::run` when no AC is supplied.
- The route refused *any* terminal goal with HTTP 409. But `ReactEngine` fails a goal that never had
  a decidable AC, so `Failed` was exactly the status produced by the 422 that sends callers here —
  the redirect was reachable but not actionable, and the failure was permanent. The gate now refuses
  only when clarification cannot help: `Succeeded`, or `Failed` *with* factors.
- A `Failed` goal that gains an AC is reset to `Pending`. Without this, the repaired goal still
  reported itself finished and the next `/react` failed, so the 422 → clarify → react cycle failed
  on its second pass and the repair was a formality that could not be used. Only goals that ended
  `Failed` for want of an AC reach this path.

**H9 — Forget Is Auditable, Not Silent**
- `MemoryStore` DELETE paths emit the same `audit.log` Merkle entry as writes; a deletion
  that leaves no trace is now indistinguishable from corruption.

**H5 — Consolidation Cannot Bypass Safety**
- `consolidation` writes route through `SafetyGuardrail::check_precondition` like every other
  mutation, so motif mining can no longer be the one unchecked writer.

**H3 — Unknown `record_type` Is Rejected, Not Coerced**
- `POST /v1/memory/add` returns HTTP 400 with `warning.valid_record_types` for an unrecognised
  `record_type` instead of silently defaulting; `RECORD_TYPE_ALIASES` (18 names) is the
  single authority.

**H1a — Dead Routes Are Reachable**
- The 9 routes declared in `build_app` but absent from the alternate router are served.
  `run_with_both_stores` deleted; the two routers had already diverged. Route count verified
  preserved (123 distinct paths before and after).

**H1b — Served OpenAPI Spec Is Generated From the Router**
- `openapi_spec::ROUTE_TABLE` (128 rows) + `canonical_path` + `auto_operation_id` +
  `spec_with_route_table()`; `/openapi.json` can no longer drift from what is actually served.

**H1c — Route Parity Is Guarded**
- `tests/integration/route_parity_sit.rs` cross-checks `ROUTE_TABLE` against `build_app`'s
  source and against the served spec. Declaration-based, because axum 0.6 exposes no route-table
  accessor.

**H4 — An Omitted Actor Is Explicit, Not Guessed**
- `CognitiveStateReport.actor_scoped: bool`. `GET /v1/cognitive/report` without `?actor` returns
  `build_unscoped_report`: empty lists, `actor_scoped: false`, and a `next_recommendation` naming
  `supply_actor`. Previously it silently substituted a default actor — cross-actor leakage that
  read as a working report.

**H8 — Capability Registration Is Derived, Not Restated**
- `src/capability_catalog.rs`: `declared_capabilities` / `origin_of` /
  `register_declared_capabilities`. `SelfModel::{list_capabilities, capability_count}` added.
  The 15-name bootstrap list in `bin/webserver.rs` is gone; `GET /v1/self/capabilities` reports a
  real `origin` per capability. Previously three disjoint vocabularies with an empty intersection
  meant `GET /v1/actions/authorized-wm` was structurally guaranteed to answer `{"authorized":[]}`.

**H6 — Q2/Q7 Answer With a Count; the Arrays Move to `*_detail`**
- `CognitiveStateReport.learned_beliefs` and `emergent_abstractions` are now `usize`; the beliefs
  are at `learned_beliefs_detail` / `emergent_abstractions_detail`. The cost of asking "how much
  have I learned" no longer scales with how much has been learned.
- **Compatibility note:** the design doc's WP8 row says the arrays are "retained under explicit
  `*_detail` keys" and that "existing consumers unaffected". On a JSON wire those cannot both hold
  for the same key — a key is a number or an array, not both. The explicit-naming clause was
  followed, and every in-repo consumer was updated in the same change. Each count is derived from
  its detail vector, and tests assert `count == detail.len()` so the two can never disagree.

## [1.3.0] - 2026-09-01 — Cognitive Loop Closure (Phases A–H)

### Added

**G-WHY — Rationale Chain (Phase A1)**
- `DecisionPayload.rationale_chain: Vec<String>` — human-readable decision trace alongside UUID evidence links
- `DecisionSummary.rationale_chain` propagated through `build_report`; `loop_engine` populates per iteration

**G-AUTH — Contextual Action Authorization (Phase A2)**
- `action_registry::list_authorized_contextual(goal_id, actor, has_workspace, health_score)` — filters `ALL_OPS` by active goal and health constraints
- `cognitive_report` Q9 now calls `list_authorized_contextual` instead of static `ALL_OPS`

**G-REV — JTMS-Routed Belief Retraction (Phase B1)**
- `BeliefInvalidator::process` is now read-only (`&MemoryStore<B>`); returns `Vec<Uuid>` of IDs to retract
- All callers route through `CognitiveHandle::retract_belief` → JTMS cascade (`propagate_retraction`)

**G-ABS — Causal Motif Mining (Phase C)**
- `consolidation.rs`: `mine_causal_motifs` + `mine_and_consolidate` — induces `Skill` + `Belief` records from recurring `derived_from` chains
- REST `/v1/memory/consolidate` supports `strategy=motif` to trigger motif compaction

**G-WS — Durable Workspaces (Phase F)**
- `Workspace`: `created_at` changed to `SystemTime`; `save(dir)`, `load(path)`, `load_all(dir)` JSONL persistence
- OR-Set CRDT merge survives restart; `WorkspaceRegistry::evict_expired` for 5-minute TTL

**G-ROLL / G-SHIFT — Kalman Prediction Monitor (Phase E)**
- `src/modules/self_model/prediction_monitor.rs`: `PredictionMonitor` — rolling-window structural drift detector
- `SelfModel::record_prediction_error` + `CognitiveHandle::check_prediction_drift` → emits `RewriteStructuralEquation` on persistent drift

**G-LOOP — SubstrateDaemon Background Worker (Phase D)**
- `src/substrate_daemon.rs`: `SubstrateDaemon` spawns per-actor maintenance threads (GC + AutoConsolidate)
- REST `POST /v1/loop/subscribe` → handle ID; `GET /v1/loop/status/:handle` → iteration count + status

**G-CRIT / G-VER — Critic + Verifier in ReactEngine (Phase G)**
- Critic: writes `Belief{action="critic_score"}` per ReAct iteration (fraction of `success_factors` satisfied)
- Verifier: writes `Belief{action="verifier_report"}` on loop exit (success or failure) with `factor_scores`
- REST `GET /goal/:id/verify` → returns the latest verifier report Belief

**G-EXPORT — Versioned State Export Schema (Phase H)**
- `knowledge_export::EXPORT_SCHEMA_VERSION` — single source of truth (`env!("CARGO_PKG_VERSION")`)
- `knowledge_export::StateExportSchema::current()` — schema descriptor with top-level field list
- REST `GET /v1/state/export` now stamps `schema_version` from the constant

### Changed
- `AppState` (web-server feature) gains `daemon: Arc<Mutex<SubstrateDaemon>>`
- `SelfModel` struct gains `prediction_monitor: Mutex<PredictionMonitor>` field

---

## [0.6.0] - 2026-08-15

### Added

**Cognitive State Infrastructure (Rust core)**
- `MemoryType::Goal`, `Skill`, `Belief` variants with strongly-typed payloads (`GoalPayload`, `SkillPayload`, `BeliefPayload`) in `src/payloads.rs`
- Provenance fields on `MemoryRecord`: `derived_from`, `evidence`, `react_iteration`
- `ArchiveStore` — append-only cold store; tiered hot/cold search (archived records excluded from default queries)
- `ExecutionGate` trait + `DecisionEngine` implementation for pre-flight operation gating
- `ReactEngine` — ReAct+Reflexion goal loop (`loop_engine.rs`); `GoalStatus` state machine (Pending → InProgress → Succeeded | Failed)
- `CognitiveGC` — provenance-aware garbage collector (`GcAction::Archive | Delete`)
- `MemoryDiff` / `compute_diff` — field-level structural diff between two `MemoryRecord` snapshots
- `CausalGraph::auto_populate_from_transitions` + backdoor adjustment (`compute_intervention`)
- `EntityConfig` + `EntityTracker::with_config()` — custom Kalman F-matrix injection
- `SelfModel::with_gate()` — injectable `ExecutionGate` override
- `WorldModelEnhanced::sync_causal_distributions()` — keeps causal DAG in step with transition model

**REST API (web-server feature)**
- `POST /goal/:id/react` — trigger ReAct loop for a goal record
- `GET /goal/:id/trace` — fetch all records derived from a goal
- `POST /memory/diff` — structural diff between two memory records
- `POST /worldmodel/rollout` — multi-step rollout (dirichlet | mcts | ensemble; iterations ≤ 200, max_depth ≤ 10)
- `POST /worldmodel/can-execute` — ExecutionGate pre-flight check

**Passive Integration Layer (Profile 0)**
- `HipCortexCallbackHandler` — LangChain passive observer (no explicit `add_memory` calls)
- `HipCortexCrewObserver` — CrewAI step/task passive observer with idempotent `inject_context`
- `HipCortexAutoGenObserver` — AutoGen v0.3 send/receive hook passive observer
- VSIX `hipcortex.passiveCapture` config toggle + `onDidWriteTerminalData` listener

**MCP Server**
- `resources/list` and `resources/read` endpoints; 3 auto-injected resources:
  - `hipcortex://context/relevant` — top-k semantic memories
  - `hipcortex://beliefs/current` — active symbolic records
  - `hipcortex://context/conversation` — recent temporal traces

**Testing**
- 5 E2E goal-driven ReAct loop acceptance tests (`tests/integration/react_e2e_sit.rs`)
- Phase 6 MCP resource tests + Profile 0 live gate (`tests/e2e_user_harness/suites/test_phase6_gap_coverage.py`)
- Phase 7 passive layer unit tests — LangChain, CrewAI, AutoGen, VSIX (`test_phase7_passive_layer.py`)

### Changed
- Python SDK version: `0.5.2` → `0.6.0` (`sdk/python/pyproject.toml`)
- VSIX version: `0.5.8` → `0.6.0` (`vscode-extension/package.json`)
- Rust crate version: `0.5.2` → `0.6.0` (`Cargo.toml`)
- MCP server `serverInfo.version`: `0.5.2` → `0.6.0`

## [0.5.2] - 2026-07-31

- VSIX 0.5.8: chmod bundled Mac/Linux server binaries on install
- Optional deep-wire integration (PR #79)
- World-model rollout API (dirichlet/mcts/ensemble modes)
- SelfModel capability registry and resource monitor
- CoherenceChecker + ConflictResolver
- CausalTopoGraph with PPR-ranked search

# Changelog

All notable changes to HipCortex are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased] — Gap Closure H1–H10 + Pipeline Enforcement

Design: `docs/superpowers/specs/2026-09-12-hipcortex-gap-closure-design.md`.
Pipeline enforcement: `docs/superpowers/specs/2026-09-12-hipcortex-pipeline-enforcement-design.md`.

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

**G1/G2/G3 — CI Now Runs What It Previously Only Declared**
- New `web-tests` job. `web-server` gates 127 integration tests (182 → 309) and `build-web` only
  compiled the binary, so those tests existed but never executed in CI — which is how a
  `record_type` mismatch on `/memory/query` reached `main` behind a green pipeline. The job runs the
  unit, integration and property suites and clippy with the feature on, on the shared `cargo-web`
  cache key.
- `build-core` runs `tests/unit/` (510 tests). `--lib` covers only the crate's inline `#[cfg(test)]`
  modules, so a file added under `tests/unit/` was invisible to the pipeline.
- `python-sdk` runs `sdk/python/tests/` (242 tests) instead of 6. The directory needs no server and
  takes ~6 s.
- `publish-pypi` runs `python scripts/stamp_versions.py --mcp --check` before `python -m build`. The
  wheel ships `hipcortex/install/mcp_server.py`, and on a PyPI install `install_hosts` falls back to
  `importlib.resources` and copies *that* file out — so drift between it and `sdk/mcp/server.py`
  would publish a broken tool set, with nothing in the release path to catch it.
- The reasoning above was understated, and running the sweep proved it. `origin/main`'s CI run
  `34604611892` fails at `Unit tests (with tokio actors)` with exit 101, and **every later step is
  skipped** — `Integration tests` (both), `Property tests` (both), `Clippy` (both) and `Rustfmt`
  have never executed on `main`. A declared gate below a failing step is not a gate, and step order
  is a silent dependency. The defect that caused it is fixed under "Fixed" below; the general point
  is that "the pipeline is green" and "the pipeline ran" are different claims, and only the last
  step of a job can tell you which one you have.

### Fixed

**G4b — Consolidation Removals Survive a Restart**
- `MemoryStore::delete_by_id` emptied `records` and rebuilt every index, but touched neither the
  backend nor the pending write buffer. `MemoryBackend` exposes `load`/`append`/`flush`/`clear` and
  **no delete**, so an append-only backend retained the record and the next `load()` read it back.
  Every caller was affected, not just the one route. Removals now go through `delete_by_ids`, which
  purges the buffer and rewrites the backend — the same durability contract `delete_by_actor`
  already honoured. `delete_by_id` is documented as non-durable rather than left as a trap.
- `MemoryStore::upsert` replaces a record in place and rewrites the backend in one pass.
  `POST /memory/consolidate` used `delete_by_id` + `add` to reinsert a survivor, which rebuilt every
  index twice and left a second copy of the same id in the append-only file. The handler now uses
  both primitives and no longer contains a `delete_by_id` call.

**G8 — A Failed Goal Is Not Reported as a Completed One**
- The 409 from `POST /goal/:id/clarify` said `cannot clarify completed goal` for a `Failed` goal
  whose ladder had already settled its success factors. That is wrong twice: the goal is not
  completed, and the caller is not stuck — `/react` rejects only an *empty* factor list, so retrying
  against the settled factors is the supported path. The body is now chosen per state: `Succeeded`
  keeps its existing message (there is genuinely nothing to retry), while `Failed`-with-factors
  names the settled factors and carries the working exit as `fix`.

**G9 — One Intervention Shape, Whatever the World Model Knows**
- `WorldModelEnhanced::causal_intervention` answered in one of two key shapes depending on hidden
  state. When empirical distributions existed it returned `<outcome>=<value>` keys plus the MAP
  estimate under the bare outcome name; when they did not, it delegated to
  `CausalGraph::compute_intervention`, which keys by outcome *value* alone (`{"1": 0.5}`). A caller
  reading the outcome variable therefore got a non-empty map with nothing addressable — the tokio
  actor test asserted exactly that and panicked. The fallback is now re-keyed into the empirical
  shape; it is purely additive (every value-qualified entry is preserved, only the key gains the
  outcome prefix) and an empty result is still returned empty. Fixed in the wrapper, **not** in
  `CausalGraph`, whose value-keyed shape is a separate public API with its own direct caller.
- This defect was **pre-existing on `origin/main`**, and finding it exposed the more serious fact
  recorded under "Changed" below: because that step fails, every step after it is skipped, so
  `origin/main` has never run its integration, property, clippy or rustfmt gates at all.

**G10 — An Acceptance Suite That No CI Step Named**
- `tests/integration/v040_contract_sit.rs` had six tests and not one of them had ever run in CI. The
  file is not missing from the registrar by oversight: `77b418b` moved it out of
  `tests/integration/mod.rs` into its own `[[test]]` target with
  `required-features = ["web-server"]`, to keep it off the `intelligence_sit` compile path — a
  legitimate structural choice, and an ancestor of `origin/main`. But a standalone target is built
  and run only by a job that names it, and every job in `ci.yml` names `unit_suite`,
  `integration_suite` and `property_suite`. So the six tests were declared, compiled on demand, and
  never executed: the same class as G1–G3, declared but never run, reached from a different gate.
  Three of them pin the v0.4.0 contract fixes the file exists for — G-LINK `POST /memory/link` field
  aliases, G-BELIEFS `GET /memory/live_beliefs` top-level `loops_run`, and G-RELATED
  `GET /memory/search/related` record enrichment.
- The remedy is therefore to *name* it, not to re-register it: `tests/integration/mod.rs` is left as
  `77b418b` wrote it, and the `web-tests` job gains a step for the target. This is the sharper half
  of the lesson: a census against the registrar is necessary and not sufficient, because the
  registrar gates only `mod`-included files, while a `[[test]]` target is gated by the CI command
  line and by nothing else. Two censuses are needed — the directory against the registrar (160
  files, one orphan: integration 73/1, unit 75/0, property 12/0), and the declared targets against
  the steps that name them.
- Running it went red on the first attempt, which **is** the finding. `handle_wm_rollout` defaulted
  its mode to `mcts` whenever `actions` was empty, so a malformed Dirichlet call was answered by the
  MCTS branch, the `actions must be non-empty` guard below it became unreachable for the request it
  was written for, and the caller got `No actions available for MCTS (observe transitions first)`.
  Both that inference and a `(or set mode=mcts)` suffix on the guard's message were introduced by a
  single later commit, `bc5d6f7` (2026-07-20).
- Both also deviate from this endpoint's archived spec,
  `openspec/changes/archive/2026-07-09-worldmodel-rollout-endpoint/design.md`, which had fixed the
  guard as an input check returning exactly `{"error": "actions must be non-empty"}` — in D2, in the
  component design, and again in verification item 4. The spec is dated nine days before the
  commit that drifted from it. The mode default is now plainly `"dirichlet"`, as `WmRolloutRequest`
  already documented, and the message is restored to the specified string — matching
  `WorldModelEnhanced::rollout_dirichlet`, the primitive the guard fronts, which returns
  `actions must be non-empty` for the same input. MCTS with no actions is unaffected: it
  still works when `mode` asks for it, which the live `worldmodel_self_http_sit` pins and passes.
- The handler was the only place in the tree that disagreed. Four declarations of the same contract
  were already present: the archived design above; `WmRolloutRequest`'s own doc comment; the OpenAPI
  entry at `src/openapi_spec.rs:556` — `"Multi-step rollout: dirichlet MAP (default)"`, with
  `mode: {"default": "dirichlet"}` and `actions: "Required unless mode=mcts"`; and the MCP tool
  schema, which carries the identical `"Required unless mode=mcts"` description and
  `"default": "dirichlet"` in `sdk/mcp/server.py:355` and its bundled mirror. For all four to hold,
  empty `actions` with no `mode` must be an input error — exactly what the code alone had stopped
  doing.
- Evidence: the target is **6 passed / 0 failed** as its own `web-tests` step; the `web-server`
  integration suite is **309 → 310, 0 failed** (the +1 is the regression test recorded under G11
  below); web unit suite 374/0. Both assertions the target went red on are the endpoint's two
  specified error contracts.

**G11 — The Guardrail Refused the Operation for Its Own Identifiers**
- Running that target surfaced a second, intermittent failure, and this one is a defect in the
  product rather than in the suite. In one of the twelve runs I gave it, `POST /memory/link`
  answered **403 `precondition blocked: PII risk=0.90 patterns=["PII:369623-7774"]`** to a request
  whose entire content was two UUIDs and `"relation": "supports"`. The guard's context was
  `format!("link {} --[{}]--> {}", from_id, relation, to_id)`, and the PII set's US-phone pattern
  `\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}` is satisfied by *six digits, a hyphen and four more
  digits* — exactly the straddle a UUID presents at one of its hyphens (`…369623-7774…`). A v4 UUID
  carries such a straddle roughly once in twenty, so a documented write endpoint refused well-formed
  requests at random, on identifiers the caller neither chooses nor can re-roll: retrying the same
  ids always failed, and the only recourse was to store the records again and hope. This is why it
  is fixed rather than tolerated — a false positive at ~5% per call is a reliability defect, not a
  strictness setting.
- The classifier is not wrong; it is answering the question it was asked. The defect is the
  question. `/memory/add` classifies `actor action target` and `/topo/apply_hyp` its `text` — both
  content — while `/memory/link` classified identifiers, which cannot be content, and was the only
  site in the tree that did. Its context is now `format!("link memory records --[{}]-->",
  req.relation)`: the check still runs, the one caller-supplied free-text field is still in scope,
  and the record ids are no longer asked to prove they are not phone numbers. They leave the audit
  context with this change — they are not what the check examines, and what the check refuses is
  what gets logged.
- Every other classification site was audited for the same mistake and is clean:
  `temporal_backend.rs` classifies the FSM condition, `semantic_cache.rs` the cache key length, and
  `check_precondition_with_threshold` and `check_postcondition` have no callers at all. Widening the
  phone pattern with a boundary was rejected: it still has to match `(555) 123-4567`, which the same
  boundary that rejects `x123456-1234` would also suppress. The detector is not where the mistake is.
- Evidence: the new `link_does_not_classify_identifiers_as_pii` in `rest_contract_safety_sit.rs`
  builds the false positive deterministically — `ab123456-1234-4abc-8def-0123456789ab` is a valid
  UUID whose first hyphen carries the phone shape — and asserts **404** (the store was consulted) in
  place of **403** (the classifier refused). It reported
  `patterns=["PII:123456-1234"]` before the fix and passes after it; the `web-server` integration
  suite is 309 → 310, 0 failed, and the standalone target ran 6/0 on three consecutive runs.

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

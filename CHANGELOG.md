# Changelog

All notable changes to HipCortex are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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

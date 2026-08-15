# Changelog

All notable changes to HipCortex are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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

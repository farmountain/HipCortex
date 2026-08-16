# HipCortex v0.8.0 — Phase 0: CognitiveState Foundation Design

> **For agentic workers:** This spec is Phase 0 of 5 sub-project specs for v0.8.0.
> Use `superpowers:writing-plans` then `superpowers:executing-plans` to implement.

**Goal:** Introduce `CognitiveHandle<B>` (internal substrate) + `CognitiveSnapshot` (external/wire type) as the unified cognitive state object, wired into `AppState<B>` without breaking any existing code.

**Version target:** v0.8.0-phase0 (foundations for subsequent phases)

**Backward compat:** Additive only. Zero existing tests may break.

---

## 1. Context & Motivation

### What already exists (v0.7.0 baseline)

- `MemoryStore<B>` — hot store, WAL, SHA-256 integrity, Merkle audit log
- `ArchiveStore` — cold store for superseded records
- `WorldModelEnhanced` — Dirichlet-Multinomial transitions, Kalman entity tracking, causal do-calculus, MCTS rollouts
- `SelfModel` + `CalibrationTracker` — EWMA prediction error, epistemic entropy, health aggregator
- `CoherenceChecker` + `SystemInvariants` — consistency checks, acyclicity, decay monotonicity
- `CognitiveGC` — provenance-aware GC
- `TxLog` + `TxStateDiff` — transaction cursor and causal diff
- `BeliefPayload` / `GoalPayload` / `SkillPayload` — typed payloads in `payloads.rs`
- `MemoryType::{Belief, Goal, Skill, Temporal, Symbolic, Procedural, Reflexion}`

### The gap

No single object composes all of the above into a coherent, queryable, transactionally-safe unit. Agent code (MCP tools, REST handlers, ReAct loop) accesses each store independently via raw `Arc` clones. Mutations bypass the coherence gate. There is no standard way to materialize a complete cognitive context.

### What Phase 0 delivers

- `CognitiveHandle<B>` — internal substrate wrapping all Arc store references
- `CognitiveSnapshot` — serializable, immutable projection of current state
- `CognitiveDelta` — typed mutation enum; 4 variants implemented, 3 stubbed for Phase 4
- `transact()` — single mutation code path: safety → coherence → apply → TxLog → calibration
- `snapshot()` — materializes all views in ~6ms at 10k records
- `SimulationFork` — stub type for Phase 2 (DigitalTwin); all methods return `NotImplemented`
- `AppState<B>` gains `cognitive: Arc<CognitiveHandle<B>>` — additive field
- `GET /v1/cognitive/snapshot` — single REST endpoint for testability
- 12 unit tests + 3 inline entropy tests

---

## 2. Type Definitions

**File:** `src/cognitive_state.rs` (new)  
**Feature gate:** none — default `petgraph_backend` build

### 2.1 View types (inside CognitiveSnapshot)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalView {
    pub hot_count: usize,
    pub recent: Vec<MemoryRecord>,      // last N temporal records, sorted created_at DESC
    pub decay_pressure: f32,            // from CalibrationTracker.consolidation_pressure()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateView {
    pub entity_count: usize,
    pub causal_edge_count: usize,
    pub transition_count: usize,
    pub dag_verified: bool,             // via SystemInvariants::check_dag_acyclicity
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfStateView {
    pub calibration_score: f32,
    pub prediction_error_ewma: f32,
    pub epistemic_entropy: f32,
    pub consolidation_pressure: f32,
    pub calibration_healthy: bool,
    pub overall_health: f32,            // from SelfModel::get_health().overall
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefEntry {
    pub id: Uuid,
    pub payload: BeliefPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefDistribution {
    pub active: Vec<BeliefEntry>,       // filtered by min_belief_confidence
    pub calibration_ece: f32,           // Phase 0: proxy = prediction_error_ewma; real ECE in Phase 4
    pub entropy: f32,                   // H(B) = -∑(c_i/∑c) · log₂(c_i/∑c)
    pub count: usize,                   // len(active)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSnapshot {
    pub id: Uuid,
    pub status: GoalStatus,
    pub payload: GoalPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSnapshot {
    pub id: Uuid,
    pub payload: SkillPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceSummary {
    pub merkle_root: String,            // hex SHA-256; "" if audit log not configured
    pub tx_cursor: u64,                 // TxLog cursor at snapshot time; 0 if no TxLog
    pub record_count: usize,            // total hot records in MemoryStore (all actors)
    pub evidence_edge_count: usize,     // ∑ record.evidence.len() across all records
}
```

### 2.2 CognitiveSnapshot — external/wire type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSnapshot {
    pub id: Uuid,                        // new Uuid per call — snapshot identity, not actor identity
    pub tx_cursor: u64,                  // read anchor (tx_before optimistic check)
    pub actor: String,
    pub temporal: TemporalView,
    pub world: WorldStateView,
    pub self_model: SelfStateView,
    pub goals: Vec<GoalSnapshot>,
    pub skills: Vec<SkillSnapshot>,
    pub beliefs: BeliefDistribution,
    pub provenance: ProvenanceSummary,
}

pub struct SnapshotOpts {
    pub actor: String,
    pub max_recent_temporal: usize,             // default: 20
    pub min_belief_confidence: f32,             // default: 0.0
    pub goal_status_filter: Option<Vec<GoalStatus>>,  // None → [Pending, InProgress]
}

impl Default for SnapshotOpts {
    fn default() -> Self {
        Self {
            actor: String::new(),
            max_recent_temporal: 20,
            min_belief_confidence: 0.0,
            goal_status_filter: None,
        }
    }
}
```

### 2.3 CognitiveDelta — mutation enum

```rust
#[derive(Debug, Clone)]
pub enum CognitiveDelta {
    // ── Phase 0 — fully implemented ─────────────────────────────────────
    AddMemory(MemoryRecord),
    UpdateBelief(BeliefPayload),
    AdvanceGoal { id: Uuid, status: GoalStatus },
    RegisterSkill(SkillPayload),

    // ── Phase 4 — stubs: return Err(NotImplemented) until Phase 4 ───────
    Consolidate { source_ids: Vec<Uuid>, summary: MemoryRecord },
    ForgetActor(String),
    ArchiveRecord(Uuid),
}

impl CognitiveDelta {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AddMemory(_)       => "cognitive.add_memory",
            Self::UpdateBelief(_)    => "cognitive.update_belief",
            Self::AdvanceGoal { .. } => "cognitive.advance_goal",
            Self::RegisterSkill(_)   => "cognitive.register_skill",
            Self::Consolidate { .. } => "cognitive.consolidate",
            Self::ForgetActor(_)     => "cognitive.forget_actor",
            Self::ArchiveRecord(_)   => "cognitive.archive_record",
        }
    }
}
```

### 2.4 TransactionResult + CognitiveError

```rust
pub struct TransactionResult {
    pub tx_cursor: u64,
    pub affected_ids: Vec<Uuid>,
    pub coherence_warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CognitiveError {
    #[error("memory: {0}")]          Memory(String),
    #[error("coherence: {0}")]       Coherence(String),
    #[error("safety: {0}")]          Safety(String),
    #[error("txlog: {0}")]           TxLog(String),
    #[error("not found: {0}")]       NotFound(Uuid),
    #[error("not implemented: {0}")] NotImplemented(String),
}
```

### 2.5 CognitiveHandle — internal substrate

```rust
pub struct CognitiveHandle<B: MemoryBackend + Clone + Send + Sync + 'static> {
    memory:      Arc<MemoryStore<B>>,
    archive:     Arc<ArchiveStore>,
    world:       Arc<WorldModelEnhanced>,
    self_model:  Arc<SelfModel>,
    tx_log:      Option<Arc<TxLog>>,
    coherence:   Arc<RwLock<CoherenceChecker>>,
    calibration: Arc<CalibrationTracker>,
    gc:          Arc<CognitiveGC>,
}

impl<B: MemoryBackend + Clone + Send + Sync + 'static> CognitiveHandle<B> {
    pub fn new(
        memory:      Arc<MemoryStore<B>>,
        archive:     Arc<ArchiveStore>,
        world:       Arc<WorldModelEnhanced>,
        self_model:  Arc<SelfModel>,
        tx_log:      Option<Arc<TxLog>>,
        coherence:   Arc<RwLock<CoherenceChecker>>,
        calibration: Arc<CalibrationTracker>,
        gc:          Arc<CognitiveGC>,
    ) -> Self

    pub fn snapshot(&self, opts: SnapshotOpts)
        -> Result<CognitiveSnapshot, CognitiveError>

    pub fn transact(&self, delta: CognitiveDelta)
        -> Result<TransactionResult, CognitiveError>

    // Phase 0 stub — constructs SimulationFork; all SimulationFork methods return NotImplemented
    pub fn fork(&self) -> SimulationFork<B>
}
```

### 2.6 SimulationFork — Phase 0 stub only

```rust
pub struct SimulationFork<B: MemoryBackend + Clone + Send + Sync + 'static> {
    inner: CognitiveHandle<B>,
    origin_cursor: u64,
}

impl<B: MemoryBackend + Clone + Send + Sync + 'static> SimulationFork<B> {
    pub fn step(&self, _action: &str) -> Result<CognitiveSnapshot, CognitiveError> {
        Err(CognitiveError::NotImplemented("SimulationFork::step — Phase 2".into()))
    }

    pub fn trajectory(
        &self,
        _actions: &[String],
        _horizon: usize,
    ) -> Result<Vec<CognitiveSnapshot>, CognitiveError> {
        Err(CognitiveError::NotImplemented(
            "SimulationFork::trajectory — Phase 2".into(),
        ))
    }

    pub fn diff_from_origin(&self) -> TxStateDiff {
        TxStateDiff::empty(self.origin_cursor)   // empty diff until Phase 2
    }
}
```

`TxStateDiff::empty(cursor)` must exist or be added to `src/state_diff.rs`. If absent, add it — returns a `TxStateDiff` with `from_tx == to_tx == cursor` and all delta fields empty.

---

## 3. `transact()` Pipeline

Order is non-negotiable. Every step must complete before the next begins.

```
CognitiveDelta
    │
    ▼ Step 1 — Safety
    │   SafetyGuardrail::check_precondition(delta.label())
    │   → Err(Safety)  blocks: unsafe ops, blacklisted actors
    │   → nothing written
    │
    ▼ Step 2 — Coherence pre-check
    │   self.coherence.read()?.check_delta(&delta)
    │   → Ok(Vec<String>)   non-blocking warnings, collected for TransactionResult
    │   → Err(String)       blocking violation → Err(Coherence), nothing written
    │
    ▼ Step 3 — Apply delta
    │   self.apply_delta(&delta)
    │   AddMemory(r)           → memory.add(r)
    │   UpdateBelief(b)        → memory.update(belief_record_from(b))
    │   AdvanceGoal{id,status} → memory.get(id) → patch GoalPayload.status → memory.update
    │   RegisterSkill(s)       → memory.add(skill_record_from(s))
    │   Consolidate / ForgetActor / ArchiveRecord
    │                          → Err(NotImplemented("Phase 4"))  ← pipeline ends here
    │   on any Err → Err(Memory/NotFound/NotImplemented), TxLog NOT written
    │
    ▼ Step 4 — TxLog append (if configured)
    │   tx_log.append_delta(&delta, &affected_ids) → new cursor
    │   on failure:
    │     attempt compensating reverse op (best-effort)
    │     if reverse also fails: tracing::error!(CRITICAL, "TxLog/store diverged: ...")
    │     return Err(TxLog)
    │   if no TxLog: tx_cursor = 0, continue silently
    │
    ▼ Step 5 — Calibration
        calibration.record_mutation(delta.label())
        → updates EWMA prediction error tracker

→ Ok(TransactionResult { tx_cursor, affected_ids, coherence_warnings })
```

### Rollback policy (documented limitation)

`MemoryStore` writes via WAL — crash-safe but not transactionally coupled to `TxLog`. Full 2-phase commit is out of scope for Phase 0. Gap: if Step 3 succeeds and Step 4 fails, store is one record ahead of log. WAL replay reconstructs the store; `TxLog` misses one entry. Acceptable for Phase 0. Full coupling is a future hardening task.

### `CoherenceChecker::check_delta()` — new method (structural checks only)

This is a fast synchronous pre-mutation validator. Does NOT run full graph consistency — that stays asynchronous. Added to the existing `CoherenceChecker` type in `src/modules/coherence/mod.rs`.

| Delta variant | Check |
|---|---|
| `AddMemory` | `record.actor` non-empty; `MemoryType` is a valid enum variant |
| `UpdateBelief` | `confidence ∈ [0.0, 1.0]`; `id` is non-nil UUID |
| `AdvanceGoal` | status transition must be legal (see table below) |
| `RegisterSkill` | `name` non-empty; `executor` non-empty |
| Phase 4 stubs | always `Ok(vec![])` — coherence never blocks stubs (Step 3 already rejects them) |

**Legal `GoalStatus` transitions:**

| From | To | Result |
|------|----|--------|
| `Pending` | `InProgress` | Ok |
| `InProgress` | `Succeeded` | Ok |
| `InProgress` | `Failed` | Ok |
| `Succeeded` | `Succeeded` | Ok (idempotent) |
| `Failed` | `Failed` | Ok (idempotent) |
| any other | any | `Err("illegal status transition: {from} → {to}")` |

Returns `Ok(Vec<String>)` for warnings; `Err(String)` for blocking violations.

---

## 4. `snapshot()` Materialization

### Read order + concurrency

```
tx_before = self.tx_cursor()          ← anchor BEFORE any reads

materialize_temporal(opts)
materialize_world()
materialize_self()
materialize_goals(opts)
materialize_skills()
materialize_beliefs(opts)
materialize_provenance(tx_before)

tx_after = self.tx_cursor()
if tx_after != tx_before:
    tracing::warn!("CognitiveSnapshot stale: cursor {} → {}", tx_before, tx_after)
    // warn only — not an error

return CognitiveSnapshot { id: Uuid::new_v4(), tx_cursor: tx_before, ... }
```

Optimistic snapshot — no global lock. Concurrent mutation logs a warning; snapshot is still returned. Documented limitation; acceptable for Phase 0.

### Individual materializers

**`materialize_temporal(opts)`**
- `memory.search_by_type(MemoryType::Temporal, &opts.actor, opts.max_recent_temporal)`
- `TemporalView { hot_count: records.len(), recent: records, decay_pressure: calibration.consolidation_pressure() }`

**`materialize_world()`**
- `world.list_entities()` → entity_count
- `world.get_causal_edges()` → causal_edge_count
- `world.transition_count()` → transition_count
- `SystemInvariants::check_dag_acyclicity(&self.world)` → dag_verified: bool

**`materialize_self()`**
- All 6 fields from `calibration.*()` + `self_model.get_health().overall`
- All O(1) reads — no store scan

**`materialize_goals(opts)`**
- `memory.search_by_type(MemoryType::Goal, &opts.actor, 100)`
- Deserialize each `record.metadata` as `GoalPayload`
- Filter by `opts.goal_status_filter` (default: `[Pending, InProgress]`)
- Map to `GoalSnapshot { id: record.id, status: payload.status.clone(), payload }`

**`materialize_skills()`**
- Query all `MemoryType::Skill` records regardless of actor — skills are global
- If `search_by_type` requires an actor arg, pass `"*"` or use `search_all_by_type`; if neither exists, add `MemoryStore::all_by_type(MemoryType) -> Result<Vec<MemoryRecord>>` (implementation plan confirms exact method name)
- Limit: 100 records; deserialize each as `SkillPayload` → `SkillSnapshot`

**`materialize_beliefs(opts)`**
- `memory.search_by_type(MemoryType::Belief, &opts.actor, 200)`
- Filter: `payload.confidence >= opts.min_belief_confidence`
- Collect `Vec<BeliefEntry>`
- Compute `entropy = compute_belief_entropy(&entries)`:
  ```
  total = ∑ entry.payload.confidence
  if total == 0 or entries empty: entropy = 0.0
  else: H = -∑ (c_i / total) · log₂(c_i / total)
  ```
- `calibration_ece = calibration.prediction_error_ewma()` — proxy for Phase 0; real ECE (binned, outcome-based) implemented in Phase 4

**`materialize_provenance(tx_cursor)`**
- `memory.record_count()` — total hot records
- `memory.evidence_edge_count()` — ∑ `record.evidence.len()` across all records
- `memory.merkle_root_hex()` — cached Merkle root; returns `""` if audit log not configured

### Required new methods on existing types

| Type | New method | Notes |
|------|-----------|-------|
| `MemoryStore<B>` | `record_count() -> Result<usize>` | count of all hot records |
| `MemoryStore<B>` | `evidence_edge_count() -> Result<usize>` | ∑ `record.evidence.len()` |
| `MemoryStore<B>` | `merkle_root_hex() -> Option<String>` | cached; `None` if no audit log |
| `CoherenceChecker` | `check_delta(delta: &CognitiveDelta) -> Result<Vec<String>, String>` | see §3 |
| `TxStateDiff` | `empty(cursor: u64) -> TxStateDiff` | for `SimulationFork::diff_from_origin` |
| `SystemInvariants` | `check_dag_acyclicity(world: &WorldModelEnhanced) -> bool` | may already exist; confirm |

**Implementation note:** if `search_by_type` does not accept an `actor` parameter today, add an overload or filter post-fetch. Do not change the existing `search_by_type` signature if it would break callers.

### Performance budget

| View | Source | Cost at 10k records |
|------|--------|---------------------|
| Temporal | scan + early exit | ~1ms |
| World | entity list + edge scan | ~0.2ms |
| Self | O(1) Arc reads | ~0μs |
| Goals | scan | ~1ms |
| Skills | scan | ~1ms |
| Beliefs | scan + entropy compute | ~2ms |
| Provenance | O(N) count + O(1) merkle | ~1ms |
| **Total** | | **~6ms** |

Acceptance Criterion 7 (sub-ms) applies to individual hot-path store reads, not to full snapshot materialization. `snapshot()` is called at context-fetch time, not per-action.

---

## 5. `AppState<B>` Integration

### New field — additive only

```rust
// src/web_server.rs
pub struct AppState<B: MemoryBackend> {
    // ALL existing fields UNCHANGED — no removals, no renames
    pub memory_store:  Arc<MemoryStore<B>>,
    pub archive_store: Arc<ArchiveStore>,
    pub world_model:   Arc<WorldModelEnhanced>,
    pub self_model:    Arc<SelfModel>,
    pub calibration:   Arc<CalibrationTracker>,
    pub tx_log:        Option<Arc<TxLog>>,
    // ... all other existing fields ...

    // NEW — Phase 0:
    pub cognitive: Arc<CognitiveHandle<B>>,
}
```

### Construction in `run_with_state()`

Appended after all existing Arc construction. Clones Arcs that already exist — no duplication of state:

```rust
// If CognitiveGC not yet in AppState, construct it:
let gc = Arc::new(CognitiveGC::new());

let cognitive = Arc::new(CognitiveHandle::new(
    memory_store.clone(),
    archive_store.clone(),
    world_model.clone(),
    self_model.clone(),
    tx_log_arc.clone(),
    coherence.clone(),          // Arc<RwLock<CoherenceChecker>>
    calibration.clone(),
    gc.clone(),
));
// Then add cognitive to AppState struct literal.
```

If `CognitiveGC` is not already constructed in `run_with_state()`, add `let gc = Arc::new(CognitiveGC::new());` before `cognitive`. `CognitiveGC::new()` has no external dependencies.

### Minimal REST endpoint (Phase 0 only)

Registered inside `run_with_state()` alongside existing v1 routes:

```
GET /v1/cognitive/snapshot
Query params:
  actor          string   required
  max_temporal   u32      optional, default 20
  min_belief_conf f32     optional, default 0.0

Response: CognitiveSnapshot (JSON)
Error:    { "error": "<message>" }  HTTP 500
```

Phase 5 adds the full Cognitive API surface (POST /v1/cognitive/transact, MCP tools, Python/TS SDK). This single endpoint is Phase 0 only for E2E testability.

---

## 6. Module Wiring

### `src/lib.rs`

```rust
// After: pub mod cognitive_gc;
pub mod cognitive_state;

// Re-exports at crate root:
pub use cognitive_state::{
    CognitiveHandle, CognitiveSnapshot, CognitiveDelta, CognitiveError,
    TransactionResult, SnapshotOpts, SimulationFork,
    TemporalView, WorldStateView, SelfStateView,
    BeliefDistribution, BeliefEntry, ProvenanceSummary,
    GoalSnapshot, SkillSnapshot,
};
```

### Test registration

```rust
// tests/unit/mod.rs — add:
mod cognitive_state_tests;
```

---

## 7. Test Strategy

**File:** `tests/unit/cognitive_state_tests.rs` (new)  
**Feature flag:** `--no-default-features --features petgraph_backend`  
**Server required:** no

| # | Test | Assertion |
|---|------|-----------|
| 1 | `handle_constructs_without_panic` | `CognitiveHandle::new(...)` completes, no panic |
| 2 | `transact_add_memory_record_visible_in_store` | `store.search(actor)` finds record after `transact(AddMemory)` |
| 3 | `transact_update_belief_patches_not_duplicates` | belief count = 1 before and after `UpdateBelief` |
| 4 | `transact_advance_goal_valid_transition` | `Pending→InProgress` returns `Ok(TransactionResult)` |
| 5 | `transact_advance_goal_invalid_transition` | `Succeeded→Pending` returns `Err(Coherence(...))` |
| 6 | `transact_coherence_blocks_invalid_confidence` | `UpdateBelief(confidence=1.5)` → `Err(Coherence(...))`, nothing written |
| 7 | `transact_txlog_cursor_increments_per_mutation` | cursor = n before; cursor = n+k after k successful mutations |
| 8 | `transact_stubs_return_not_implemented` | `Consolidate`, `ForgetActor`, `ArchiveRecord` all return `Err(NotImplemented(...))` |
| 9 | `snapshot_all_views_populated_after_mutations` | after `AddMemory` + `UpdateBelief`: `temporal.hot_count > 0`, `beliefs.count > 0` |
| 10 | `snapshot_tx_cursor_matches_txlog_at_call_time` | `snapshot.tx_cursor == tx_log.current_cursor()` when no concurrent mutation |
| 11 | `snapshot_repeated_calls_same_cursor_no_mutation` | two snapshots, no mutation between → `snap1.tx_cursor == snap2.tx_cursor` |
| 12 | `fork_stub_methods_return_not_implemented` | `fork()` constructs; `step()` and `trajectory()` → `Err(NotImplemented(...))` |

### Inline entropy tests (`src/cognitive_state.rs` — `#[cfg(test)]`)

```rust
#[test]
fn uniform_beliefs_max_entropy() {
    // 4 beliefs, each confidence=0.25 → H = log₂(4) = 2.0 bits
    let entries = vec![entry(0.25); 4];
    assert!((compute_belief_entropy(&entries) - 2.0).abs() < 1e-3);
}

#[test]
fn single_belief_zero_entropy() {
    assert_eq!(compute_belief_entropy(&[entry(1.0)]), 0.0);
}

#[test]
fn empty_beliefs_zero_entropy() {
    assert_eq!(compute_belief_entropy(&[]), 0.0);
}
```

---

## 8. Acceptance Criteria — Phase 0 Gates

All must pass before Phase 1 begins.

| Gate | Criterion | Measurement |
|------|-----------|-------------|
| G0-1 | All 12 unit tests pass | `cargo test cognitive_state_tests` = 0 failures |
| G0-2 | All 3 entropy tests pass | inline `#[cfg(test)]` = 0 failures |
| G0-3 | All pre-existing tests still pass | `cargo test --lib` = same pass count as v0.7.0 |
| G0-4 | `cargo clippy` clean | 0 errors, 0 warnings with `-D warnings` |
| G0-5 | `GET /v1/cognitive/snapshot` returns 200 with all fields present | manual curl or E2E test |
| G0-6 | `transact(Consolidate)` returns `NotImplemented`, not panic | test #8 |
| G0-7 | `snapshot()` returns in < 10ms for ≤1k records | unit test timing assertion |
| G0-8 | `fork()` constructs without panic; `step()` returns `NotImplemented` | test #12 |

---

## 9. Explicit Out-of-Scope for Phase 0

Do not implement these during Phase 0. They have their own specs.

| Item | Phase |
|------|-------|
| `SimulationFork::step()` real impl (copy-on-write stores) | 2 — DigitalTwin |
| `POST /v1/cognitive/transact` REST endpoint | 5 — Agent Surfaces |
| MCP tools: `get_cognitive_state`, `project_view` | 5 — Agent Surfaces |
| Python SDK `CognitiveState` class | 5 — Agent Surfaces |
| TypeScript SDK `CognitiveState` class | 5 — Agent Surfaces |
| `Consolidate` / `ForgetActor` / `ArchiveRecord` arms | 4 — ExperienceStore |
| Continuous dynamics on `SimulationFork` | 3 — Hybrid Dynamics |
| Real ECE (outcome-based, binned calibration) | 4 — ExperienceStore |
| `DigitalTwin` struct | 2 — DigitalTwin |

---

## 10. File Changeset

```
NEW   src/cognitive_state.rs                    ~380 lines
MOD   src/lib.rs                                +1 pub mod + re-exports
MOD   src/web_server.rs                         +1 AppState field, +1 route, CognitiveHandle construction
MOD   src/modules/coherence/mod.rs              +check_delta() method on CoherenceChecker
MOD   src/memory_store.rs                       +record_count(), evidence_edge_count(), merkle_root_hex()
MOD   src/state_diff.rs                         +TxStateDiff::empty(cursor)
NEW   tests/unit/cognitive_state_tests.rs       12 tests
MOD   tests/unit/mod.rs                         +1 mod line
```

If `SystemInvariants::check_dag_acyclicity` does not exist in `src/modules/coherence/`, add it there. If it exists under a different name, use that name — do not duplicate.

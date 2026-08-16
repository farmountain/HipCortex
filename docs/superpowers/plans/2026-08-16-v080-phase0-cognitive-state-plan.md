# v0.8.0 Phase 0 — CognitiveState Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `CognitiveHandle<B>` + `CognitiveSnapshot` + `transact()` + `snapshot()` to HipCortex, exposed via `GET /v1/cognitive/snapshot`.

**Architecture:** Thin coordination layer over existing stores. `CognitiveHandle<B>` wraps `MemoryStore<B>`, `WorldModelEnhanced`, `SelfModel`, `CoherenceChecker`, `TxLog`, and `CalibrationTracker`. Enforces the coherence gate on all mutations. Does NOT replace or break existing APIs.

**Tech Stack:** Rust (edition 2021, MSRV ~1.70), Axum 0.6, serde/serde_json, sha2 (already in Cargo.toml), uuid, chrono

**Spec:** `docs/superpowers/specs/2026-08-16-v080-phase0-cognitive-state-design.md`

---

## Corrections vs Spec

These spec errors were found by reading actual source files. This plan uses the corrected versions throughout — do NOT use the spec versions for these:

| Spec wrote | Correct (use this) |
|---|---|
| `Arc<MemoryStore<B>>` | `Arc<Mutex<MemoryStore<B>>>` (add/update need `&mut self`) |
| `Arc<WorldModelEnhanced>` | `Arc<RwLock<WorldModelEnhanced>>` |
| `Arc<RwLock<CoherenceChecker>>` | `Arc<CoherenceChecker>` (already thread-safe internally) |
| `tx_log.current_cursor()` | `tx_log.current_tx()` |
| `tx_log.append_delta()` | `tx_log.append(kind, ids, actor)` returns `u64` |
| `record.memory_type` | `record.record_type` |
| `calibration.calibration_score()` | `calibration.snapshot().calibration_score` (field on CalibrationState) |
| `calibration.record_mutation()` | `calibration.record_prediction_error(0.0)` |
| Safety gate via `SafetyGuardrail` | `coherence.gate_write(delta.label())` — takes `&self`, returns `Result<(), WriteRejection>` |
| `search_by_type()` | `all()` + filter by `r.record_type == rt` |
| `UpdateBelief(BeliefPayload)` | `UpdateBelief { id: Uuid, payload: BeliefPayload }` — `BeliefPayload` has no `id` field |

---

## File Map

**Create:**
- `src/cognitive_state.rs` — all types, `CognitiveDelta`, `CognitiveSnapshot`, `CognitiveHandle<B>` impl
- `src/simulation_fork.rs` — `SimulationFork<B>` Phase-2 stub

**Modify:**
- `src/memory_store.rs:282` — add 4 helper methods after `pub fn all`
- `src/state_diff.rs` — add `TxStateDiff::empty(cursor)`
- `src/modules/coherence/mod.rs` — add `CoherenceChecker::check_delta()`
- `src/modules/world_model_enhanced/mod.rs` — add `causal_node_count()`, `causal_edge_count()`
- `src/lib.rs` — `pub mod cognitive_state;` + `pub mod simulation_fork;`
- `src/web_server.rs:145-179` — add `cognitive` field to `AppState`; `src/web_server.rs:505-518` — construct it in `run_with_memory`; `src/web_server.rs:895` — register route
- `tests/unit/cognitive_state_tests.rs` — 12 unit tests + 3 entropy tests (new file)
- `tests/unit/mod.rs` — `pub mod cognitive_state_tests;`

---

## Acceptance Gates

| Gate | Criterion | Covered by |
|---|---|---|
| G0-1 | 12 unit tests pass | Task 3–7 tests |
| G0-2 | 3 entropy tests pass | Task 6 tests |
| G0-3 | Pre-existing `--lib` tests unchanged | Task 9 |
| G0-4 | `cargo clippy` 0 errors/warnings with `-D warnings` | Task 9 |
| G0-5 | `GET /v1/cognitive/snapshot` returns 200 with all fields | Task 8 + Task 9 |
| G0-6 | `transact(Consolidate)` returns `NotImplemented`, not panic | Task 5 test #4 |
| G0-7 | `snapshot()` < 10ms for ≤1k records | Task 6 test #4 |
| G0-8 | `fork()` no panic; `step()` returns `NotImplemented` | Task 7 tests |

---

## Task 1: MemoryStore helpers + TxStateDiff::empty

**Files:**
- Modify: `src/memory_store.rs` (after line 282, `pub fn all`)
- Modify: `src/state_diff.rs`

- [ ] **Step 1.1: Write failing tests**

  Add to `#[cfg(test)] mod tests` block at line 878 of `src/memory_store.rs`:

  ```rust
  #[test]
  fn test_record_count_empty() {
      let store = MemoryStore::new_in_memory();
      assert_eq!(store.record_count(), 0);
  }

  #[test]
  fn test_record_count_after_add() {
      let mut store = MemoryStore::new_in_memory();
      let r = MemoryRecord::new(MemoryType::Temporal, "a".into(), "did".into(), "t".into(), serde_json::Value::Null);
      store.add(r).unwrap();
      assert_eq!(store.record_count(), 1);
  }

  #[test]
  fn test_all_by_type_filters() {
      let mut store = MemoryStore::new_in_memory();
      let t = MemoryRecord::new(MemoryType::Temporal, "a".into(), "did".into(), "t".into(), serde_json::Value::Null);
      let b = MemoryRecord::new(MemoryType::Belief, "a".into(), "assert".into(), "prop".into(), serde_json::Value::Null);
      store.add(t).unwrap();
      store.add(b).unwrap();
      assert_eq!(store.all_by_type(MemoryType::Temporal).len(), 1);
      assert_eq!(store.all_by_type(MemoryType::Belief).len(), 1);
      assert_eq!(store.all_by_type(MemoryType::Goal).len(), 0);
  }

  #[test]
  fn test_evidence_edge_count() {
      let mut store = MemoryStore::new_in_memory();
      let id1 = uuid::Uuid::new_v4();
      let id2 = uuid::Uuid::new_v4();
      let mut r = MemoryRecord::new(MemoryType::Temporal, "a".into(), "did".into(), "t".into(), serde_json::Value::Null);
      r.evidence = vec![id1, id2];
      store.add(r).unwrap();
      assert_eq!(store.evidence_edge_count(), 2);
  }

  #[test]
  fn test_merkle_root_hex_non_empty() {
      let mut store = MemoryStore::new_in_memory();
      let r = MemoryRecord::new(MemoryType::Temporal, "a".into(), "did".into(), "t".into(), serde_json::Value::Null);
      store.add(r).unwrap();
      let root = store.merkle_root_hex();
      assert_eq!(root.len(), 64, "SHA-256 hex = 64 chars");
  }
  ```

- [ ] **Step 1.2: Run to verify failure**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --lib test_record_count_empty 2>&1 | tail -5
  ```
  Expected: `error[E0599]: no method named 'record_count' found`

- [ ] **Step 1.3: Implement helpers in src/memory_store.rs**

  Add after `pub fn all(&self) -> &[MemoryRecord]` (line 282):

  ```rust
  pub fn record_count(&self) -> usize {
      self.records.iter().filter(|r| r.status == "active").count()
  }

  pub fn all_by_type(&self, rt: crate::memory_record::MemoryType) -> Vec<&MemoryRecord> {
      self.records
          .iter()
          .filter(|r| r.record_type == rt && r.status == "active")
          .collect()
  }

  pub fn evidence_edge_count(&self) -> usize {
      self.records.iter().map(|r| r.evidence.len()).sum()
  }

  pub fn merkle_root_hex(&self) -> String {
      use sha2::{Digest, Sha256};
      let mut hasher = Sha256::new();
      for r in &self.records {
          let h = r.integrity.as_deref().unwrap_or("");
          hasher.update(h.as_bytes());
      }
      format!("{:x}", hasher.finalize())
  }
  ```

  `sha2` is already in `Cargo.toml = "0.10"`. `MemoryType` already derives `PartialEq` — no changes needed there.

- [ ] **Step 1.4: Write failing test for TxStateDiff::empty**

  Locate the `#[cfg(test)]` block in `src/state_diff.rs`. If none exists, add one at the end. Add:

  ```rust
  #[test]
  fn test_tx_state_diff_empty_cursor() {
      let diff = TxStateDiff::empty(42);
      assert_eq!(diff.from_tx, 42);
      assert_eq!(diff.to_tx, 42);
      assert_eq!(diff.tx_count, 0);
      assert!(diff.causal_attributions.is_empty());
  }
  ```

- [ ] **Step 1.5: Check MemoryDelta + WorldModelDelta derive Default**

  In `src/state_diff.rs`, find `pub struct MemoryDelta` and `pub struct WorldModelDelta`. Add `Default` to their `#[derive(...)]` if not already present.

- [ ] **Step 1.6: Implement TxStateDiff::empty in src/state_diff.rs**

  Add to `impl TxStateDiff`:

  ```rust
  pub fn empty(cursor: u64) -> Self {
      Self {
          from_tx: cursor,
          to_tx: cursor,
          timestamp_range: (0, 0),
          tx_count: 0,
          memory_delta: MemoryDelta::default(),
          world_model_delta: WorldModelDelta::default(),
          causal_attributions: Vec::new(),
      }
  }
  ```

- [ ] **Step 1.7: Run tests to verify pass**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --lib test_record_count 2>&1 | tail -5
  cargo test --no-default-features --features "petgraph_backend" --lib test_all_by_type 2>&1 | tail -5
  cargo test --no-default-features --features "petgraph_backend" --lib test_evidence_edge_count 2>&1 | tail -5
  cargo test --no-default-features --features "petgraph_backend" --lib test_merkle_root_hex 2>&1 | tail -5
  cargo test --no-default-features --features "petgraph_backend" --lib test_tx_state_diff_empty 2>&1 | tail -5
  ```
  Expected: all 5 pass, 0 failures.

- [ ] **Step 1.8: Commit**

  ```sh
  git add src/memory_store.rs src/state_diff.rs
  git commit -m "feat(cognitive): add MemoryStore helpers + TxStateDiff::empty for Phase 0"
  ```

---

## Task 2: WorldModelEnhanced — causal count helpers

`CausalGraph` has `pub fn node_count()` and `pub fn edge_count()`, but the field `causal_graph` is private on `WorldModelEnhanced`. Add two forwarding methods.

**Files:** `src/modules/world_model_enhanced/mod.rs`

- [ ] **Step 2.1: Write failing tests**

  Add to the inline `#[cfg(test)]` block in `src/modules/world_model_enhanced/mod.rs`:

  ```rust
  #[test]
  fn test_wm_causal_counts_empty() {
      let wm = WorldModelEnhanced::new();
      assert_eq!(wm.causal_node_count(), 0);
      assert_eq!(wm.causal_edge_count(), 0);
  }

  #[test]
  fn test_wm_causal_counts_after_edge() {
      let wm = WorldModelEnhanced::new();
      wm.add_causal_edge("A".into(), "B".into()).unwrap();
      assert_eq!(wm.causal_node_count(), 2);
      assert_eq!(wm.causal_edge_count(), 1);
  }
  ```

- [ ] **Step 2.2: Run to verify failure**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --lib test_wm_causal_counts 2>&1 | tail -5
  ```
  Expected: `error[E0599]: no method named 'causal_node_count'`

- [ ] **Step 2.3: Implement**

  Add to `impl WorldModelEnhanced` in `src/modules/world_model_enhanced/mod.rs`:

  ```rust
  pub fn causal_node_count(&self) -> usize {
      self.causal_graph.read().map(|g| g.node_count()).unwrap_or(0)
  }

  pub fn causal_edge_count(&self) -> usize {
      self.causal_graph.read().map(|g| g.edge_count()).unwrap_or(0)
  }
  ```

- [ ] **Step 2.4: Run tests to verify pass**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --lib test_wm_causal_counts 2>&1 | tail -5
  ```
  Expected: 2 tests pass.

- [ ] **Step 2.5: Commit**

  ```sh
  git add src/modules/world_model_enhanced/mod.rs
  git commit -m "feat(cognitive): expose causal_node_count/edge_count on WorldModelEnhanced"
  ```

---

## Task 3: Scaffold src/cognitive_state.rs — types only

All types, no impl bodies beyond `label()`. This unblocks Tasks 4–7.

**Files:**
- Create: `src/cognitive_state.rs`
- Modify: `src/lib.rs` — add `pub mod cognitive_state;`
- Create: `tests/unit/cognitive_state_tests.rs`
- Modify: `tests/unit/mod.rs` — add `pub mod cognitive_state_tests;`

- [ ] **Step 3.1: Write failing compile test**

  Create `tests/unit/cognitive_state_tests.rs`:

  ```rust
  use hipcortex::cognitive_state::{CognitiveDelta, CognitiveError, CognitiveSnapshot};
  use hipcortex::memory_record::{MemoryRecord, MemoryType};
  use hipcortex::payloads::{BeliefPayload, EpistemicStatus, GoalStatus, SkillPayload};
  use uuid::Uuid;

  #[test]
  fn test_cognitive_delta_label_add_memory() {
      let r = MemoryRecord::new(MemoryType::Temporal, "a".into(), "did".into(), "t".into(), serde_json::json!({}));
      let delta = CognitiveDelta::AddMemory(r);
      assert_eq!(delta.label(), "AddMemory");
  }

  #[test]
  fn test_cognitive_delta_label_update_belief() {
      let payload = BeliefPayload {
          proposition: "sky is blue".into(),
          justification: "".into(),
          contradicts: vec![],
          confidence: 0.9,
          epistemic_status: EpistemicStatus::Observed,
          causal_source_ids: vec![],
          half_life_ms: 0,
          tx_origin: None,
      };
      let delta = CognitiveDelta::UpdateBelief { id: Uuid::new_v4(), payload };
      assert_eq!(delta.label(), "UpdateBelief");
  }

  #[test]
  fn test_cognitive_error_not_implemented_display() {
      let e = CognitiveError::NotImplemented("Consolidate".into());
      let msg = format!("{}", e);
      assert!(msg.contains("not implemented"), "got: {msg}");
  }
  ```

  Add to `tests/unit/mod.rs`:
  ```rust
  pub mod cognitive_state_tests;
  ```

- [ ] **Step 3.2: Run to verify failure**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite cognitive_state_tests 2>&1 | tail -10
  ```
  Expected: `error[E0432]: unresolved import 'hipcortex::cognitive_state'`

- [ ] **Step 3.3: Create src/cognitive_state.rs**

  ```rust
  //! CognitiveState — unified transactional interface over all HipCortex stores.
  //!
  //! Chain-of-thought: Agent code currently accesses MemoryStore, WorldModel, and
  //! SelfModel independently via raw Arc clones, bypassing the coherence gate.
  //! CognitiveHandle<B> is the single composition point: all mutations go through
  //! transact(), all reads go through snapshot().

  use std::fmt;
  use std::sync::{Arc, Mutex};
  use uuid::Uuid;

  use crate::cognitive_gc::CognitiveGC;
  use crate::memory_record::{MemoryRecord, MemoryType};
  use crate::memory_store::MemoryStore;
  use crate::modules::coherence::CoherenceChecker;
  use crate::modules::self_model::calibration::CalibrationTracker;
  use crate::modules::self_model::SelfModel;
  use crate::modules::world_model_enhanced::WorldModelEnhanced;
  use crate::payloads::{BeliefPayload, EpistemicStatus, GoalPayload, GoalStatus, SkillPayload};
  use crate::persistence::MemoryBackend;
  use crate::tx_log::{TxKind, TxLog};

  // ─── Error ───────────────────────────────────────────────────────────────────

  #[derive(Debug, Clone)]
  pub enum CognitiveError {
      CoherenceRejection(String),
      DeltaInvalid(String),
      StoreError(String),
      NotImplemented(String),
      LockError,
  }

  impl fmt::Display for CognitiveError {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
          match self {
              Self::CoherenceRejection(r) => write!(f, "coherence rejection: {r}"),
              Self::DeltaInvalid(r) => write!(f, "delta invalid: {r}"),
              Self::StoreError(r) => write!(f, "store error: {r}"),
              Self::NotImplemented(op) => write!(f, "{op} not implemented in Phase 0"),
              Self::LockError => write!(f, "lock poisoned"),
          }
      }
  }

  impl std::error::Error for CognitiveError {}

  // ─── CognitiveDelta ──────────────────────────────────────────────────────────

  /// All mutations go through this enum.
  /// Phase-4 variants (Consolidate, ForgetActor, ArchiveRecord) compile but
  /// return CognitiveError::NotImplemented at runtime until Phase 4.
  #[derive(Debug, Clone)]
  pub enum CognitiveDelta {
      // Phase 0 — implemented
      AddMemory(MemoryRecord),
      /// `id` = the MemoryRecord.id of the existing Belief record to update.
      /// BeliefPayload has no id field of its own.
      UpdateBelief { id: Uuid, payload: BeliefPayload },
      AdvanceGoal { id: Uuid, status: GoalStatus },
      RegisterSkill(SkillPayload),
      // Phase 4 stubs — return CognitiveError::NotImplemented
      Consolidate { source_ids: Vec<Uuid>, summary: MemoryRecord },
      ForgetActor(String),
      ArchiveRecord(Uuid),
  }

  impl CognitiveDelta {
      pub fn label(&self) -> &'static str {
          match self {
              Self::AddMemory(_) => "AddMemory",
              Self::UpdateBelief { .. } => "UpdateBelief",
              Self::AdvanceGoal { .. } => "AdvanceGoal",
              Self::RegisterSkill(_) => "RegisterSkill",
              Self::Consolidate { .. } => "Consolidate",
              Self::ForgetActor(_) => "ForgetActor",
              Self::ArchiveRecord(_) => "ArchiveRecord",
          }
      }
  }

  // ─── Snapshot sub-types ──────────────────────────────────────────────────────

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct TemporalView {
      pub record_count: usize,
      pub recent_actions: Vec<String>,
      pub temporal_span_ms: u64,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct WorldStateView {
      pub node_count: usize,
      pub edge_count: usize,
      pub dag_verified: bool,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct SelfStateView {
      pub calibration_score: f32,
      pub prediction_error_ewma: f32,
      pub consolidation_pressure: f32,
      pub epistemic_entropy: f32,
      pub healthy: bool,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct GoalSnapshot {
      pub id: Uuid,
      pub target_state: String,
      pub status: GoalStatus,
      pub iteration: u32,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct SkillSnapshot {
      pub id: Uuid,
      pub procedure: String,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct BeliefSummary {
      pub id: Uuid,
      pub proposition: String,
      pub confidence: f32,
      pub epistemic_status: EpistemicStatus,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct BeliefDistribution {
      pub count: usize,
      pub mean_confidence: f32,
      pub epistemic_entropy: f32,
      pub beliefs: Vec<BeliefSummary>,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct ProvenanceSummary {
      pub merkle_root_hex: String,
      pub record_count: usize,
      pub evidence_edge_count: usize,
  }

  #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
  pub struct CognitiveSnapshot {
      pub id: Uuid,
      pub tx_cursor: u64,
      pub actor: String,
      pub temporal: TemporalView,
      pub world: WorldStateView,
      pub self_model: SelfStateView,
      pub goals: Vec<GoalSnapshot>,
      pub skills: Vec<SkillSnapshot>,
      pub beliefs: BeliefDistribution,
      pub provenance: ProvenanceSummary,
  }

  // ─── CognitiveHandle ─────────────────────────────────────────────────────────

  pub struct CognitiveHandle<B: MemoryBackend + Clone + Send + Sync + 'static> {
      pub(crate) memory: Arc<Mutex<MemoryStore<B>>>,
      pub(crate) world: Arc<std::sync::RwLock<WorldModelEnhanced>>,
      pub(crate) self_model: Arc<SelfModel>,
      pub(crate) tx_log: Option<Arc<TxLog>>,
      pub(crate) coherence: Arc<CoherenceChecker>,
      pub(crate) calibration: Arc<CalibrationTracker>,
      pub(crate) gc: Arc<CognitiveGC>,
  }
  ```

- [ ] **Step 3.4: Register in src/lib.rs**

  Add near top of module declarations:
  ```rust
  pub mod cognitive_state;
  ```

- [ ] **Step 3.5: Run tests to verify pass**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite cognitive_state_tests 2>&1 | tail -10
  ```
  Expected: 3 tests pass, 0 failures.

- [ ] **Step 3.6: Commit**

  ```sh
  git add src/cognitive_state.rs src/lib.rs tests/unit/cognitive_state_tests.rs tests/unit/mod.rs
  git commit -m "feat(cognitive): scaffold CognitiveDelta, CognitiveError, CognitiveSnapshot, CognitiveHandle types"
  ```

---

## Task 4: CoherenceChecker::check_delta

**Files:** `src/modules/coherence/mod.rs`

- [ ] **Step 4.1: Write failing tests**

  Add to `tests/unit/cognitive_state_tests.rs`:

  ```rust
  use hipcortex::modules::coherence::CoherenceChecker;
  use std::sync::Arc;

  #[test]
  fn test_check_delta_add_memory_ok() {
      let checker = CoherenceChecker::new();
      let r = MemoryRecord::new(MemoryType::Temporal, "a".into(), "did".into(), "t".into(), serde_json::json!({}));
      let result = checker.check_delta(&CognitiveDelta::AddMemory(r));
      assert!(result.is_ok(), "{:?}", result.err());
  }

  #[test]
  fn test_check_delta_add_memory_empty_actor_err() {
      let checker = CoherenceChecker::new();
      let r = MemoryRecord::new(MemoryType::Temporal, "".into(), "did".into(), "t".into(), serde_json::json!({}));
      let result = checker.check_delta(&CognitiveDelta::AddMemory(r));
      assert!(result.is_err(), "empty actor should fail structural check");
  }

  #[test]
  fn test_check_delta_update_belief_bad_confidence_err() {
      let checker = CoherenceChecker::new();
      let payload = BeliefPayload {
          proposition: "test".into(),
          justification: "".into(),
          contradicts: vec![],
          confidence: 1.5,  // out of [0,1]
          epistemic_status: EpistemicStatus::Hypothetical,
          causal_source_ids: vec![],
          half_life_ms: 0,
          tx_origin: None,
      };
      let delta = CognitiveDelta::UpdateBelief { id: Uuid::new_v4(), payload };
      let result = checker.check_delta(&delta);
      assert!(result.is_err());
  }

  #[test]
  fn test_check_delta_consolidate_warns_not_implemented() {
      let checker = CoherenceChecker::new();
      let summary = MemoryRecord::new(MemoryType::Reflexion, "a".into(), "consolidate".into(), "s".into(), serde_json::json!({}));
      let delta = CognitiveDelta::Consolidate { source_ids: vec![], summary };
      let result = checker.check_delta(&delta);
      assert!(result.is_ok(), "stub variants return Ok with warning");
      assert!(result.unwrap().iter().any(|w| w.contains("not implemented")));
  }
  ```

- [ ] **Step 4.2: Run to verify failure**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_check_delta 2>&1 | tail -10
  ```
  Expected: `error[E0599]: no method named 'check_delta'`

- [ ] **Step 4.3: Implement check_delta in src/modules/coherence/mod.rs**

  Add to `impl CoherenceChecker`:

  ```rust
  /// Structural validation of a CognitiveDelta before it is applied.
  /// Returns Ok(warnings) on success. Phase-4 stubs return Ok with a warning
  /// so transact() can reject them with NotImplemented without coherence noise.
  pub fn check_delta(
      &self,
      delta: &crate::cognitive_state::CognitiveDelta,
  ) -> Result<Vec<String>, String> {
      use crate::cognitive_state::CognitiveDelta;
      let mut warnings = Vec::new();
      match delta {
          CognitiveDelta::AddMemory(r) => {
              if r.actor.is_empty() {
                  return Err("AddMemory: actor must not be empty".into());
              }
          }
          CognitiveDelta::UpdateBelief { id, payload } => {
              if payload.confidence < 0.0 || payload.confidence > 1.0 {
                  return Err(format!(
                      "UpdateBelief {id}: confidence {:.3} out of [0,1]",
                      payload.confidence
                  ));
              }
          }
          CognitiveDelta::AdvanceGoal { .. } | CognitiveDelta::RegisterSkill(_) => {}
          CognitiveDelta::Consolidate { .. }
          | CognitiveDelta::ForgetActor(_)
          | CognitiveDelta::ArchiveRecord(_) => {
              warnings.push(format!("{} not implemented in Phase 0", delta.label()));
          }
      }
      Ok(warnings)
  }
  ```

- [ ] **Step 4.4: Run tests to verify pass**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_check_delta 2>&1 | tail -10
  ```
  Expected: 4 tests pass.

- [ ] **Step 4.5: Commit**

  ```sh
  git add src/modules/coherence/mod.rs tests/unit/cognitive_state_tests.rs
  git commit -m "feat(cognitive): add CoherenceChecker::check_delta for delta structural validation"
  ```

---

## Task 5: CognitiveHandle::new + transact + apply_delta

**Files:** `src/cognitive_state.rs`

- [ ] **Step 5.1: Write failing tests**

  Add to `tests/unit/cognitive_state_tests.rs`:

  ```rust
  use hipcortex::cognitive_state::CognitiveHandle;
  use hipcortex::memory_store::MemoryStore;
  use hipcortex::persistence::InMemoryBackend;
  use hipcortex::modules::world_model_enhanced::WorldModelEnhanced;
  use hipcortex::modules::self_model::SelfModel;
  use hipcortex::modules::self_model::calibration::CalibrationTracker;
  use hipcortex::cognitive_gc::CognitiveGC;
  use std::sync::{Arc, Mutex, RwLock};

  fn make_handle() -> CognitiveHandle<InMemoryBackend> {
      CognitiveHandle::new(
          Arc::new(Mutex::new(MemoryStore::new_in_memory())),
          Arc::new(RwLock::new(WorldModelEnhanced::new())),
          Arc::new(SelfModel::new()),
          None,
          Arc::new(CoherenceChecker::new()),
          Arc::new(CalibrationTracker::new()),
          Arc::new(CognitiveGC::new()),
      )
  }

  #[test]
  fn test_transact_add_memory_ok() {
      let handle = make_handle();
      let r = MemoryRecord::new(MemoryType::Temporal, "agent-a".into(), "did".into(), "task".into(), serde_json::json!({}));
      assert!(handle.transact(CognitiveDelta::AddMemory(r), "agent-a").is_ok());
  }

  #[test]
  fn test_transact_add_memory_persists() {
      let handle = make_handle();
      let r = MemoryRecord::new(MemoryType::Temporal, "agent-b".into(), "did".into(), "task".into(), serde_json::json!({}));
      handle.transact(CognitiveDelta::AddMemory(r), "agent-b").unwrap();
      assert_eq!(handle.memory.lock().unwrap().record_count(), 1);
  }

  #[test]
  fn test_transact_advance_goal_not_found_err() {
      let handle = make_handle();
      let delta = CognitiveDelta::AdvanceGoal { id: Uuid::new_v4(), status: GoalStatus::Succeeded };
      let err = handle.transact(delta, "a").unwrap_err();
      assert!(matches!(err, CognitiveError::StoreError(_)));
  }

  #[test]
  fn test_transact_consolidate_not_implemented() {
      let handle = make_handle();
      let summary = MemoryRecord::new(MemoryType::Reflexion, "a".into(), "consolidate".into(), "s".into(), serde_json::json!({}));
      let delta = CognitiveDelta::Consolidate { source_ids: vec![], summary };
      let err = handle.transact(delta, "a").unwrap_err();
      assert!(matches!(err, CognitiveError::NotImplemented(_)));
  }

  #[test]
  fn test_transact_register_skill() {
      let handle = make_handle();
      let skill = SkillPayload { procedure: "step A then step B".into(), preconditions: vec![], expected_outcomes: vec![] };
      handle.transact(CognitiveDelta::RegisterSkill(skill), "agent-a").unwrap();
      assert_eq!(handle.memory.lock().unwrap().all_by_type(MemoryType::Skill).len(), 1);
  }

  #[test]
  fn test_transact_advance_goal_illegal_transition_err() {
      let handle = make_handle();
      let goal_payload = hipcortex::payloads::GoalPayload {
          target_state: "done".into(),
          acceptance_criteria: vec![],
          success_factors: vec![],
          max_react_iterations: 5,
          status: GoalStatus::Pending,
          current_iteration: 0,
      };
      let meta = serde_json::to_value(&goal_payload).unwrap();
      let r = MemoryRecord::new(MemoryType::Goal, "a".into(), "create".into(), "goal".into(), meta);
      let goal_id = r.id;
      handle.transact(CognitiveDelta::AddMemory(r), "a").unwrap();
      // Pending → Succeeded (must go via InProgress first)
      let err = handle.transact(
          CognitiveDelta::AdvanceGoal { id: goal_id, status: GoalStatus::Succeeded },
          "a",
      ).unwrap_err();
      assert!(matches!(err, CognitiveError::DeltaInvalid(_)));
  }
  ```

- [ ] **Step 5.2: Run to verify failure**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_transact 2>&1 | tail -10
  ```
  Expected: `error[E0599]: no method named 'transact'`

- [ ] **Step 5.3: Implement CognitiveHandle::new + transact + apply_delta in src/cognitive_state.rs**

  Append to the file (after the struct definition):

  ```rust
  impl<B: MemoryBackend + Clone + Send + Sync + 'static> CognitiveHandle<B> {
      pub fn new(
          memory: Arc<Mutex<MemoryStore<B>>>,
          world: Arc<std::sync::RwLock<WorldModelEnhanced>>,
          self_model: Arc<SelfModel>,
          tx_log: Option<Arc<TxLog>>,
          coherence: Arc<CoherenceChecker>,
          calibration: Arc<CalibrationTracker>,
          gc: Arc<CognitiveGC>,
      ) -> Self {
          Self { memory, world, self_model, tx_log, coherence, calibration, gc }
      }

      /// Apply a CognitiveDelta transactionally.
      ///
      /// Pipeline: safety gate → structural check → Phase-4 guard → apply → tx log → calibration ping.
      pub fn transact(&self, delta: CognitiveDelta, actor: &str) -> Result<(), CognitiveError> {
          // Step 1: Safety gate — WriteRejection.reason holds the summary string
          self.coherence
              .gate_write(delta.label())
              .map_err(|r| CognitiveError::CoherenceRejection(r.reason))?;

          // Step 2: Structural coherence
          self.coherence
              .check_delta(&delta)
              .map_err(CognitiveError::DeltaInvalid)?;

          // Step 3: Phase-4 stubs reject before touching any store
          match &delta {
              CognitiveDelta::Consolidate { .. }
              | CognitiveDelta::ForgetActor(_)
              | CognitiveDelta::ArchiveRecord(_) => {
                  return Err(CognitiveError::NotImplemented(delta.label().into()));
              }
              _ => {}
          }

          // Step 4: Apply to underlying store(s)
          let affected_ids = self.apply_delta(&delta)?;

          // Step 5: TxLog append (no-op when tx_log is None)
          if let Some(tx) = &self.tx_log {
              let kind = match &delta {
                  CognitiveDelta::AddMemory(_) => TxKind::MemoryAdd,
                  CognitiveDelta::UpdateBelief { .. } => TxKind::BeliefAssert,
                  CognitiveDelta::AdvanceGoal { .. } => TxKind::GoalStatusChange,
                  CognitiveDelta::RegisterSkill(_) => TxKind::MemoryAdd,
                  _ => unreachable!(),
              };
              tx.append(kind, affected_ids, actor);
          }

          // Step 6: Calibration ping — mutation happened; expected, so error = 0
          self.calibration.record_prediction_error(0.0);

          Ok(())
      }

      fn apply_delta(&self, delta: &CognitiveDelta) -> Result<Vec<Uuid>, CognitiveError> {
          match delta {
              CognitiveDelta::AddMemory(record) => {
                  let id = record.id;
                  self.memory
                      .lock()
                      .map_err(|_| CognitiveError::LockError)?
                      .add(record.clone())
                      .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                  Ok(vec![id])
              }

              CognitiveDelta::UpdateBelief { id, payload } => {
                  let new_meta = serde_json::to_value(payload)
                      .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                  self.memory
                      .lock()
                      .map_err(|_| CognitiveError::LockError)?
                      .update_record(*id, None, None, Some(payload.confidence), None, Some(new_meta))
                      .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                  Ok(vec![*id])
              }

              CognitiveDelta::AdvanceGoal { id, status } => {
                  let mut store = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
                  // Read current payload — find_by_id returns &MemoryRecord so clone it before &mut ops
                  let meta = store
                      .find_by_id(*id)
                      .ok_or_else(|| CognitiveError::StoreError(format!("goal {id} not found")))?
                      .metadata
                      .clone();
                  let mut payload: GoalPayload = serde_json::from_value(meta)
                      .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                  Self::validate_goal_transition(&payload.status, status)?;
                  payload.status = status.clone();
                  let new_meta = serde_json::to_value(&payload)
                      .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                  store
                      .update_record(*id, None, None, None, None, Some(new_meta))
                      .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                  Ok(vec![*id])
              }

              CognitiveDelta::RegisterSkill(skill) => {
                  let meta = serde_json::to_value(skill)
                      .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                  let record = MemoryRecord::new(
                      MemoryType::Skill,
                      "system".into(),
                      "register".into(),
                      skill.procedure.clone(),
                      meta,
                  );
                  let id = record.id;
                  self.memory
                      .lock()
                      .map_err(|_| CognitiveError::LockError)?
                      .add(record)
                      .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
                  Ok(vec![id])
              }

              _ => unreachable!("Phase-4 stubs rejected in transact() before apply_delta"),
          }
      }

      fn validate_goal_transition(
          from: &GoalStatus,
          to: &GoalStatus,
      ) -> Result<(), CognitiveError> {
          let ok = matches!(
              (from, to),
              (GoalStatus::Pending, GoalStatus::InProgress)
              | (GoalStatus::InProgress, GoalStatus::Succeeded)
              | (GoalStatus::InProgress, GoalStatus::Failed)
              | (GoalStatus::Succeeded, GoalStatus::Succeeded)   // idempotent
              | (GoalStatus::Failed, GoalStatus::Failed)         // idempotent
          );
          if ok {
              Ok(())
          } else {
              Err(CognitiveError::DeltaInvalid(format!(
                  "illegal status transition {from:?} → {to:?}"
              )))
          }
      }
  }
  ```

- [ ] **Step 5.4: Run tests to verify pass**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_transact 2>&1 | tail -15
  ```
  Expected: 6 tests pass, 0 failures.

- [ ] **Step 5.5: Commit**

  ```sh
  git add src/cognitive_state.rs tests/unit/cognitive_state_tests.rs
  git commit -m "feat(cognitive): implement CognitiveHandle::new + transact + apply_delta"
  ```

---

## Task 6: snapshot() + materializers + entropy

**Files:** `src/cognitive_state.rs`

- [ ] **Step 6.1: Write failing tests**

  Add to `tests/unit/cognitive_state_tests.rs`:

  ```rust
  use hipcortex::cognitive_state::compute_epistemic_entropy;

  #[test]
  fn test_snapshot_empty_store() {
      let handle = make_handle();
      let s = handle.snapshot("agent-a").unwrap();
      assert_eq!(s.actor, "agent-a");
      assert_eq!(s.temporal.record_count, 0);
      assert_eq!(s.beliefs.count, 0);
      assert!(s.goals.is_empty());
      assert!(s.skills.is_empty());
  }

  #[test]
  fn test_snapshot_temporal_count() {
      let handle = make_handle();
      let r = MemoryRecord::new(MemoryType::Temporal, "agent-c".into(), "did".into(), "task".into(), serde_json::json!({}));
      handle.transact(CognitiveDelta::AddMemory(r), "agent-c").unwrap();
      let s = handle.snapshot("agent-c").unwrap();
      assert_eq!(s.temporal.record_count, 1);
  }

  #[test]
  fn test_snapshot_skill_appears() {
      let handle = make_handle();
      let skill = SkillPayload { procedure: "plan → act".into(), preconditions: vec![], expected_outcomes: vec![] };
      handle.transact(CognitiveDelta::RegisterSkill(skill), "agent-d").unwrap();
      let s = handle.snapshot("agent-d").unwrap();
      assert_eq!(s.skills.len(), 1);
      assert_eq!(s.skills[0].procedure, "plan → act");
  }

  #[test]
  fn test_snapshot_latency_1k_records() {
      let handle = make_handle();
      for i in 0..1000u32 {
          let r = MemoryRecord::new(MemoryType::Temporal, "perf".into(), "did".into(), format!("t{i}"), serde_json::json!({}));
          handle.memory.lock().unwrap().add(r).unwrap();
      }
      let t0 = std::time::Instant::now();
      let s = handle.snapshot("perf").unwrap();
      let ms = t0.elapsed().as_millis();
      assert!(ms < 10, "snapshot took {ms}ms; G0-7 requires < 10ms for ≤1k records");
      assert_eq!(s.temporal.record_count, 1000);
  }

  // G0-2 entropy tests
  #[test]
  fn test_entropy_all_certain() {
      // h(1.0) = 0; mean = 0
      assert!(compute_epistemic_entropy(&[1.0, 1.0, 1.0]) < 1e-4);
  }

  #[test]
  fn test_entropy_all_uncertain() {
      // h(0.5) = 1.0; mean = 1.0
      let e = compute_epistemic_entropy(&[0.5, 0.5, 0.5]);
      assert!((e - 1.0).abs() < 1e-4, "got {e}");
  }

  #[test]
  fn test_entropy_empty() {
      assert_eq!(compute_epistemic_entropy(&[]), 0.0);
  }
  ```

- [ ] **Step 6.2: Run to verify failure**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_snapshot 2>&1 | tail -10
  ```
  Expected: `error[E0599]: no method named 'snapshot'`

- [ ] **Step 6.3: Implement compute_epistemic_entropy + snapshot in src/cognitive_state.rs**

  Add before the `impl<B>` block:

  ```rust
  /// Mean binary entropy over a slice of confidence values in [0,1].
  /// h(p) = -p*log2(p) - (1-p)*log2(1-p). Returns 0.0 for empty slice.
  pub fn compute_epistemic_entropy(confs: &[f32]) -> f32 {
      if confs.is_empty() {
          return 0.0;
      }
      let h_sum: f32 = confs
          .iter()
          .map(|&c| {
              let c = c.clamp(1e-9, 1.0 - 1e-9);
              -(c * c.log2() + (1.0 - c) * (1.0 - c).log2())
          })
          .sum();
      h_sum / confs.len() as f32
  }
  ```

  Add to the `impl<B>` block (after `validate_goal_transition`):

  ```rust
  /// Materialise a complete CognitiveSnapshot for the given actor.
  /// actor = "" → include all actors.
  pub fn snapshot(&self, actor: &str) -> Result<CognitiveSnapshot, CognitiveError> {
      let mem = self.memory.lock().map_err(|_| CognitiveError::LockError)?;

      // ── Temporal view ────────────────────────────────────────────────────
      let temporal_recs: Vec<_> = mem
          .all()
          .iter()
          .filter(|r| {
              r.record_type == MemoryType::Temporal
                  && r.status == "active"
                  && (actor.is_empty() || r.actor == actor)
          })
          .collect();
      let temporal_span_ms = match temporal_recs.as_slice() {
          [] | [_] => 0,
          recs => {
              let oldest = recs.first().unwrap().timestamp;
              let newest = recs.last().unwrap().timestamp;
              (newest - oldest).num_milliseconds().max(0) as u64
          }
      };
      let temporal = TemporalView {
          record_count: temporal_recs.len(),
          recent_actions: temporal_recs
              .iter()
              .rev()
              .take(5)
              .map(|r| r.action.clone())
              .collect(),
          temporal_span_ms,
      };

      // ── Goal view ────────────────────────────────────────────────────────
      let goals: Vec<GoalSnapshot> = mem
          .all()
          .iter()
          .filter(|r| {
              r.record_type == MemoryType::Goal
                  && r.status == "active"
                  && (actor.is_empty() || r.actor == actor)
          })
          .filter_map(|r| {
              serde_json::from_value::<GoalPayload>(r.metadata.clone())
                  .ok()
                  .map(|p| GoalSnapshot {
                      id: r.id,
                      target_state: p.target_state,
                      status: p.status,
                      iteration: p.current_iteration,
                  })
          })
          .collect();

      // ── Skill view ───────────────────────────────────────────────────────
      let skills: Vec<SkillSnapshot> = mem
          .all_by_type(MemoryType::Skill)
          .iter()
          .filter_map(|r| {
              serde_json::from_value::<SkillPayload>(r.metadata.clone())
                  .ok()
                  .map(|p| SkillSnapshot { id: r.id, procedure: p.procedure })
          })
          .collect();

      // ── Belief distribution ──────────────────────────────────────────────
      let belief_summaries: Vec<BeliefSummary> = mem
          .all()
          .iter()
          .filter(|r| {
              r.record_type == MemoryType::Belief
                  && r.status == "active"
                  && (actor.is_empty() || r.actor == actor)
          })
          .filter_map(|r| {
              serde_json::from_value::<BeliefPayload>(r.metadata.clone())
                  .ok()
                  .map(|p| BeliefSummary {
                      id: r.id,
                      proposition: p.proposition,
                      confidence: p.confidence,
                      epistemic_status: p.epistemic_status,
                  })
          })
          .collect();
      let confs: Vec<f32> = belief_summaries.iter().map(|b| b.confidence).collect();
      let mean_confidence = if confs.is_empty() {
          0.0
      } else {
          confs.iter().sum::<f32>() / confs.len() as f32
      };
      let beliefs = BeliefDistribution {
          count: belief_summaries.len(),
          mean_confidence,
          epistemic_entropy: compute_epistemic_entropy(&confs),
          beliefs: belief_summaries,
      };

      // ── Provenance ───────────────────────────────────────────────────────
      let provenance = ProvenanceSummary {
          merkle_root_hex: mem.merkle_root_hex(),
          record_count: mem.record_count(),
          evidence_edge_count: mem.evidence_edge_count(),
      };

      drop(mem); // release MemoryStore lock before acquiring WorldModel read lock

      // ── World state ──────────────────────────────────────────────────────
      let wm = self.world.read().map_err(|_| CognitiveError::LockError)?;
      let world = WorldStateView {
          node_count: wm.causal_node_count(),
          edge_count: wm.causal_edge_count(),
          dag_verified: true, // CausalGraph::add_edge enforces acyclicity at write time
      };
      drop(wm);

      // ── Self-model ───────────────────────────────────────────────────────
      let cal = self.calibration.snapshot();
      let self_model = SelfStateView {
          calibration_score: cal.calibration_score,
          prediction_error_ewma: cal.prediction_error_ewma,
          consolidation_pressure: cal.consolidation_pressure,
          epistemic_entropy: cal.epistemic_entropy,
          healthy: cal.healthy,
      };

      // ── Tx cursor ────────────────────────────────────────────────────────
      let tx_cursor = self.tx_log.as_ref().map(|t| t.current_tx()).unwrap_or(0);

      Ok(CognitiveSnapshot {
          id: Uuid::new_v4(),
          tx_cursor,
          actor: actor.to_string(),
          temporal,
          world,
          self_model,
          goals,
          skills,
          beliefs,
          provenance,
      })
  }
  ```

  Required import at top of `src/cognitive_state.rs`:
  ```rust
  use chrono::Utc as _; // for .num_milliseconds() on Duration — chrono already in Cargo.toml
  ```
  (Chrono `DateTime` subtraction returns `chrono::Duration`; `.num_milliseconds()` is on `chrono::Duration`.)

- [ ] **Step 6.4: Run tests to verify pass**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_snapshot 2>&1 | tail -15
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_entropy 2>&1 | tail -10
  ```
  Expected: 7 tests pass (4 snapshot + 3 entropy).

- [ ] **Step 6.5: Commit**

  ```sh
  git add src/cognitive_state.rs tests/unit/cognitive_state_tests.rs
  git commit -m "feat(cognitive): implement CognitiveHandle::snapshot + compute_epistemic_entropy"
  ```

---

## Task 7: SimulationFork stub + fork()

**Files:**
- Create: `src/simulation_fork.rs`
- Modify: `src/cognitive_state.rs` — add `fork()` + `use crate::simulation_fork::SimulationFork;`
- Modify: `src/lib.rs` — `pub mod simulation_fork;`

- [ ] **Step 7.1: Write failing tests**

  Add to `tests/unit/cognitive_state_tests.rs`:

  ```rust
  use hipcortex::simulation_fork::SimulationFork;

  #[test]
  fn test_fork_constructs_no_panic() {
      let handle = make_handle();
      let fork = handle.fork();
      assert!(fork.is_ok(), "fork() should return Ok(SimulationFork)");
  }

  #[test]
  fn test_fork_step_not_implemented() {
      let handle = make_handle();
      let fork = handle.fork().unwrap();
      let err = fork.step("some action").unwrap_err();
      assert!(matches!(err, CognitiveError::NotImplemented(_)));
  }
  ```

- [ ] **Step 7.2: Run to verify failure**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_fork 2>&1 | tail -10
  ```
  Expected: `error[E0432]: unresolved import 'hipcortex::simulation_fork'`

- [ ] **Step 7.3: Create src/simulation_fork.rs**

  ```rust
  //! Phase-2 stub. Real copy-on-write digital-twin semantics ship with the
  //! DigitalTwin spec. All methods return NotImplemented until then.

  use crate::cognitive_state::CognitiveError;
  use crate::persistence::MemoryBackend;

  pub struct SimulationFork<B: MemoryBackend + Clone + Send + Sync + 'static> {
      _marker: std::marker::PhantomData<B>,
  }

  impl<B: MemoryBackend + Clone + Send + Sync + 'static> SimulationFork<B> {
      pub(crate) fn new_stub() -> Self {
          Self { _marker: std::marker::PhantomData }
      }

      pub fn step(&self, _action: &str) -> Result<(), CognitiveError> {
          Err(CognitiveError::NotImplemented("SimulationFork::step (Phase 2)".into()))
      }

      pub fn rollout(&self, _steps: usize) -> Result<Vec<String>, CognitiveError> {
          Err(CognitiveError::NotImplemented("SimulationFork::rollout (Phase 2)".into()))
      }
  }
  ```

- [ ] **Step 7.4: Add fork() to CognitiveHandle in src/cognitive_state.rs**

  Add at top of file:
  ```rust
  use crate::simulation_fork::SimulationFork;
  ```

  Add to `impl<B>` block:
  ```rust
  pub fn fork(&self) -> Result<SimulationFork<B>, CognitiveError> {
      Ok(SimulationFork::new_stub())
  }
  ```

- [ ] **Step 7.5: Register in src/lib.rs**

  ```rust
  pub mod simulation_fork;
  ```

- [ ] **Step 7.6: Run tests to verify pass**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_fork 2>&1 | tail -10
  ```
  Expected: 2 tests pass.

- [ ] **Step 7.7: Commit**

  ```sh
  git add src/simulation_fork.rs src/cognitive_state.rs src/lib.rs tests/unit/cognitive_state_tests.rs
  git commit -m "feat(cognitive): add SimulationFork Phase-2 stub + CognitiveHandle::fork"
  ```

---

## Task 8: AppState wiring + REST endpoint

**Files:** `src/web_server.rs`

- [ ] **Step 8.1: Add cognitive field to AppState**

  In `src/web_server.rs` at lines 145-179, add to `pub struct AppState<B>`:
  ```rust
  pub cognitive: std::sync::Arc<crate::cognitive_state::CognitiveHandle<B>>,
  ```

  Add to imports at the top of `src/web_server.rs`:
  ```rust
  use crate::cognitive_state::{CognitiveHandle, CognitiveSnapshot};
  ```

- [ ] **Step 8.2: Construct cognitive in run_with_memory (line 505-518)**

  In `run_with_memory`, before `let state = AppState {`, add:
  ```rust
  let cognitive = std::sync::Arc::new(CognitiveHandle::new(
      Arc::clone(&memory_store),
      Arc::new(RwLock::new(WorldModelEnhanced::new())),
      Arc::new(SelfModel::new()),
      None,
      Arc::new(crate::modules::coherence::CoherenceChecker::new()),
      Arc::new(crate::modules::self_model::calibration::CalibrationTracker::new()),
      Arc::new(crate::cognitive_gc::CognitiveGC::new()),
  ));
  ```

  Add `cognitive` field to the `AppState { ... }` literal:
  ```rust
  cognitive,
  ```

- [ ] **Step 8.3: Add GET /v1/cognitive/snapshot handler**

  Add handler function before `run_with_state`:

  ```rust
  #[cfg(feature = "web-server")]
  async fn get_cognitive_snapshot<B: MemoryBackend + Send + Sync + 'static>(
      axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState<B>>>,
      axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
  ) -> impl axum::response::IntoResponse {
      let actor = params.get("actor").map(|s| s.as_str()).unwrap_or("");
      match state.cognitive.snapshot(actor) {
          Ok(snap) => (axum::http::StatusCode::OK, axum::Json(snap)).into_response(),
          Err(e) => (
              axum::http::StatusCode::INTERNAL_SERVER_ERROR,
              axum::Json(serde_json::json!({ "error": e.to_string() })),
          )
              .into_response(),
      }
  }
  ```

- [ ] **Step 8.4: Register route in Router::new() block (line 895)**

  Add to the route list:
  ```rust
  .route("/v1/cognitive/snapshot", axum::routing::get(get_cognitive_snapshot::<B>))
  ```

  Place it near the other `/worldmodel/` routes for logical grouping.

- [ ] **Step 8.5: Build with web-server feature**

  ```sh
  cargo build --no-default-features --features "web-server,petgraph_backend" 2>&1 | tail -20
  ```
  Expected: `Finished` with no errors. Fix any compile errors before continuing.

- [ ] **Step 8.6: Commit**

  ```sh
  git add src/web_server.rs
  git commit -m "feat(cognitive): wire CognitiveHandle into AppState + GET /v1/cognitive/snapshot"
  ```

---

## Task 9: Final verification — all G0 gates

- [ ] **G0-1 + G0-2: All unit tests pass**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --test unit_suite cognitive_state_tests 2>&1 | tail -30
  ```
  Expected: ≥15 tests pass (12 unit + 3 entropy), 0 failures.

- [ ] **G0-3: Pre-existing lib tests unchanged**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
  ```
  Count must equal or exceed the v0.7.0 baseline. If any pre-existing test now fails, fix before proceeding.

- [ ] **G0-4: clippy clean**

  ```sh
  cargo clippy --no-default-features --features "petgraph_backend" --all-targets -- -D warnings 2>&1 | grep "^error\|^warning" | head -20
  ```
  Expected: 0 lines. Fix all warnings — common issues: unused imports, missing `#[allow(dead_code)]` on `_marker` (or prefix with `_`), needless borrow.

- [ ] **G0-5: REST endpoint returns 200 with all fields**

  ```sh
  cargo run --no-default-features --features "web-server,petgraph_backend" --bin webserver &
  sleep 2
  curl -s "http://127.0.0.1:3030/v1/cognitive/snapshot?actor=test" | python -m json.tool
  kill %1
  ```
  Expected: 200 response with JSON containing keys: `id`, `tx_cursor`, `actor`, `temporal`, `world`, `self_model`, `goals`, `skills`, `beliefs`, `provenance`.

- [ ] **G0-6: Consolidate returns NotImplemented (not panic)**

  Covered by `test_transact_consolidate_not_implemented` in G0-1.

- [ ] **G0-7: snapshot < 10ms for ≤1k records**

  Covered by `test_snapshot_latency_1k_records` in G0-1.

- [ ] **G0-8: fork constructs + step returns NotImplemented**

  Covered by `test_fork_constructs_no_panic` + `test_fork_step_not_implemented` in G0-1.

- [ ] **Final commit**

  ```sh
  cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
  cargo clippy --no-default-features --features "petgraph_backend" --all-targets -- -D warnings 2>&1 | tail -3
  git add -u
  git commit -m "feat(cognitive): v0.8.0 Phase 0 complete — CognitiveHandle + snapshot + REST

  All G0-1..G0-8 acceptance gates pass.
  New types: CognitiveHandle<B>, CognitiveDelta (7 variants), CognitiveSnapshot, SimulationFork stub.
  New endpoint: GET /v1/cognitive/snapshot.
  Phase-4 stubs (Consolidate, ForgetActor, ArchiveRecord) compile but return NotImplemented.
  "
  ```

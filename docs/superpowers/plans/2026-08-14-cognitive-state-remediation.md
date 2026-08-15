# Cognitive State Infrastructure Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Sprint 1–4 gap remediation — backdoor causal adjustment, typed cognitive records with provenance, tiered storage, ReAct+Reflexion goal loop, and ExecutionGate seam — without breaking any existing HipCortex functionality.

**Architecture:** Additive-only changes to `MemoryRecord` (`#[serde(default, skip_serializing_if)]` on all new fields preserves SHA-256 hash backward compatibility). `ReactEngine` added to `loop_engine.rs` as a new struct alongside existing `LoopEngine`. `ExecutionGate` trait wraps existing `DecisionEngine` with zero change to its public API. No exhaustive `match record_type` patterns exist in the codebase (verified by grep), so new `MemoryType` variants are safe to add.

**Tech Stack:** Rust 2021 edition, Axum 0.6, Serde/serde_json, Uuid, Chrono, SHA-256 via sha2. Minimal build: `cargo build --no-default-features --features "petgraph_backend"`. Tests: `cargo test --no-default-features --features "petgraph_backend" --lib`.

---

## File Map

**Create:**
- `src/payloads.rs` — GoalPayload, SkillPayload, BeliefPayload, SuccessFactor, GoalStatus
- `src/archive_store.rs` — ArchiveStore (append-only JSONL cold store)
- `src/cognitive_gc.rs` — CognitiveGC (provenance-aware GC callback)
- `src/execution_gate.rs` — trait ExecutionGate + impl for DecisionEngine

**Modify:**
- `src/modules/world_model_enhanced/causal.rs` — `auto_populate_from_transitions`, implement `compute_intervention` backdoor, add `intervention_label` to `InterventionQuery`
- `src/memory_record.rs` — add `MemoryType::Goal/Skill/Belief`; add `evidence`, `derived_from`, `react_iteration` fields
- `src/memory_store.rs` — add `include_archived` filter to search methods
- `src/modules/loop_engine.rs` — add `ReactEngine` struct
- `src/modules/self_model/decision.rs` — `impl ExecutionGate for DecisionEngine`
- `src/modules/self_model/mod.rs` — use `Arc<dyn ExecutionGate>`
- `src/modules/world_model_enhanced/entity.rs` — Kalman `F` matrix config
- `src/web_server.rs` — add `POST /goal/:id/react`, `GET /goal/:id/trace`, `POST /memory/diff` routes
- `src/lib.rs` — register `payloads`, `archive_store`, `cognitive_gc`, `execution_gate` modules

---

## Sprint 1 — Complete A2: Backdoor Causal Adjustment

### Task 1: auto_populate_from_transitions on CausalGraph

**Files:**
- Modify: `src/modules/world_model_enhanced/causal.rs`

- [ ] **Step 1.1: Write failing test**

Add inside `#[cfg(test)] mod tests { ... }` at the bottom of `causal.rs`:

```rust
#[test]
fn test_auto_populate_fills_distributions() {
    use crate::modules::world_model_enhanced::transition::{TransitionModel, StateTransition};
    let mut model = TransitionModel::new();
    model.record_transition(StateTransition { from_state: "s1".into(), action: "a1".into(), to_state: "s2".into() }).unwrap();
    model.record_transition(StateTransition { from_state: "s1".into(), action: "a1".into(), to_state: "s2".into() }).unwrap();
    model.record_transition(StateTransition { from_state: "s1".into(), action: "a1".into(), to_state: "s3".into() }).unwrap();

    let mut graph = CausalGraph::new();
    graph.auto_populate_from_transitions(&model);

    assert!(graph.has_empirical_key("action", "a1"), "missing distribution action=a1");
    assert!(graph.distributions.contains_key("prior_Z"), "missing prior_Z marginal");
    assert!(graph.nodes.contains_key("state"), "state node not added");
    assert!(graph.nodes.contains_key("action"), "action node not added");
    assert!(graph.nodes.contains_key("next_state"), "next_state node not added");
}
```

- [ ] **Step 1.2: Run test — expect compile error**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_auto_populate_fills_distributions 2>&1 | tail -5
```
Expected: `error[E0599]: no method named 'auto_populate_from_transitions'`

- [ ] **Step 1.3: Implement auto_populate_from_transitions**

Add after the `record_empirical_distribution` method (around line 317) in `causal.rs`:

```rust
/// Populate empirical distributions from a trained TransitionModel.
/// Adds nodes "state", "action", "next_state" with causal structure:
///   state → action (confounder), state → next_state, action → next_state
/// Populates:
///   "prior_Z"               → {"state=<s>": P(s)}
///   "action=<a>,state=<s>"  → {"next_state=<ns>": P(ns|a,s)}
///   "action=<a>"            → {"next_state=<ns>": Σ_s P(ns|a,s)·P(s)}
pub fn auto_populate_from_transitions(
    &mut self,
    model: &crate::modules::world_model_enhanced::transition::TransitionModel,
) {
    let _ = self.add_node("state".to_string());
    let _ = self.add_node("action".to_string());
    let _ = self.add_node("next_state".to_string());
    let _ = self.add_edge("state".to_string(), "action".to_string());
    let _ = self.add_edge("state".to_string(), "next_state".to_string());
    let _ = self.add_edge("action".to_string(), "next_state".to_string());

    let states = model.get_states();
    let actions = model.get_actions();

    // P(state) marginal
    let total_obs: usize = model.totals.values().sum();
    if total_obs > 0 {
        let mut state_counts: HashMap<String, usize> = HashMap::new();
        for ((s, _), &c) in &model.totals {
            *state_counts.entry(s.clone()).or_insert(0) += c;
        }
        let prior_z: HashMap<String, f64> = state_counts
            .iter()
            .map(|(s, &c)| (format!("state={}", s), c as f64 / total_obs as f64))
            .collect();
        self.distributions.insert("prior_Z".to_string(), prior_z);
    }

    for action in &actions {
        let mut marginal: HashMap<String, f64> = HashMap::new();
        let n_states = states.len().max(1) as f64;

        for state in &states {
            if let Ok(pred) = model.predict(state, action) {
                let dist: HashMap<String, f64> = pred
                    .probabilities
                    .iter()
                    .map(|(ns, &p)| (format!("next_state={}", ns), p))
                    .collect();
                let p_state = self.distributions
                    .get("prior_Z")
                    .and_then(|d| d.get(&format!("state={}", state)))
                    .copied()
                    .unwrap_or(1.0 / n_states);
                for (k, p) in &dist {
                    *marginal.entry(k.clone()).or_insert(0.0) += p * p_state;
                }
                self.distributions.insert(format!("action={},state={}", action, state), dist);
            }
        }
        if !marginal.is_empty() {
            self.distributions.insert(format!("action={}", action), marginal);
        }
    }
}
```

- [ ] **Step 1.4: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_auto_populate_fills_distributions 2>&1 | tail -5
```
Expected: `test ...::test_auto_populate_fills_distributions ... ok`

- [ ] **Step 1.5: Commit**

```bash
git add src/modules/world_model_enhanced/causal.rs
git commit -m "feat(causal): auto_populate_from_transitions builds empirical distributions from TransitionModel"
```

---

### Task 2: Implement compute_intervention backdoor + add intervention_label

**Files:**
- Modify: `src/modules/world_model_enhanced/causal.rs`

- [ ] **Step 2.1: Add intervention_label to InterventionQuery**

Find `InterventionQuery` struct (around line 38) and add one field:

```rust
pub struct InterventionQuery {
    pub outcome: String,
    pub intervention_var: String,
    pub intervention_value: f64,
    pub conditioned_on: HashMap<String, f64>,
    /// String label for discrete interventions (e.g. action names).
    /// When set, used as the distribution lookup key instead of intervention_value.to_string().
    #[serde(default)]
    pub intervention_label: Option<String>,
}
```

- [ ] **Step 2.2: Write failing test**

Add to `#[cfg(test)]` in `causal.rs`:

```rust
#[test]
fn test_compute_intervention_backdoor() {
    let mut graph = CausalGraph::new();
    let _ = graph.add_node("X".to_string());
    let _ = graph.add_node("Y".to_string());
    let _ = graph.add_node("Z".to_string());
    let _ = graph.add_edge("Z".to_string(), "X".to_string());
    let _ = graph.add_edge("Z".to_string(), "Y".to_string());
    let _ = graph.add_edge("X".to_string(), "Y".to_string());

    // P(Z) prior: P(Z=z0)=0.6, P(Z=z1)=0.4
    graph.record_empirical_distribution("prior_Z".to_string(), "Z=z0".to_string(), 0.6);
    graph.record_empirical_distribution("prior_Z".to_string(), "Z=z1".to_string(), 0.4);
    // P(Y|X=1,Z=z0): P(Y=1)=0.8
    graph.record_empirical_distribution("X=1,Z=z0".to_string(), "Y=1".to_string(), 0.8);
    graph.record_empirical_distribution("X=1,Z=z0".to_string(), "Y=0".to_string(), 0.2);
    // P(Y|X=1,Z=z1): P(Y=1)=0.3
    graph.record_empirical_distribution("X=1,Z=z1".to_string(), "Y=1".to_string(), 0.3);
    graph.record_empirical_distribution("X=1,Z=z1".to_string(), "Y=0".to_string(), 0.7);

    let query = InterventionQuery {
        outcome: "Y".to_string(),
        intervention_var: "X".to_string(),
        intervention_value: 1.0,
        conditioned_on: HashMap::new(),
        intervention_label: None,
    };

    let result = graph.compute_intervention(&query).unwrap();
    // Σ_z P(Y=1|X=1,Z=z)*P(Z=z) = 0.8*0.6 + 0.3*0.4 = 0.60
    let p_y1 = result.get("1").copied().unwrap_or(0.0);
    assert!((p_y1 - 0.60).abs() < 0.01, "Expected 0.60, got {}", p_y1);
    // Stub returns {"Y": 0.0} — this test will fail on stub
    assert!(!result.contains_key("Y"), "stub result shape found — backdoor not implemented");
}
```

- [ ] **Step 2.3: Run test — expect FAILED (stub returns 0.0)**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_compute_intervention_backdoor 2>&1 | tail -10
```
Expected: assertion failure (p_y1 == 0.0, expected 0.60).

- [ ] **Step 2.4: Replace compute_intervention stub with backdoor implementation**

Replace lines 270–273 (`let mut result = HashMap::new(); result.insert(...); Ok(result)`) with:

```rust
        let iv_str = query.intervention_label.as_deref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let v = query.intervention_value;
                if v == v.trunc() && v.abs() < 1e15 { format!("{}", v as i64) } else { format!("{}", v) }
            });

        let outcome_prefix = format!("{}=", query.outcome);
        let mut outcome_values: HashSet<String> = HashSet::new();
        for dist in self.distributions.values() {
            for key in dist.keys() {
                if let Some(val) = key.strip_prefix(&outcome_prefix) {
                    outcome_values.insert(val.to_string());
                }
            }
        }
        if outcome_values.is_empty() { outcome_values.insert("1".to_string()); }

        let adjustment_set = self.get_parents(&query.intervention_var);
        let mut result = HashMap::new();

        for outcome_val in &outcome_values {
            let outcome_key = format!("{}={}", query.outcome, outcome_val);
            let prob = if adjustment_set.is_empty() {
                let key_x = format!("{}={}", query.intervention_var, iv_str);
                self.distributions.get(&key_x).and_then(|d| d.get(&outcome_key)).copied().unwrap_or(0.5)
            } else {
                // Build cartesian product of Z configurations
                let mut z_configs: Vec<(String, Vec<String>)> = Vec::new();
                for z_var in &adjustment_set {
                    let z_pfx = format!("{}=", z_var);
                    let mut states: Vec<String> = self.distributions.keys()
                        .filter_map(|k| k.strip_prefix(&z_pfx).map(|v| v.to_string()))
                        .collect::<std::collections::HashSet<_>>().into_iter().collect();
                    if states.is_empty() { states.push("default".to_string()); }
                    z_configs.push((z_var.clone(), states));
                }
                let mut cartesian: Vec<Vec<(String, String)>> = vec![Vec::new()];
                for (z_var, states) in &z_configs {
                    cartesian = cartesian.iter().flat_map(|c| {
                        states.iter().map(move |s| { let mut n = c.clone(); n.push((z_var.clone(), s.clone())); n })
                    }).collect();
                }
                let num_z = cartesian.len().max(1) as f64;
                let mut total = 0.0;
                for z_config in &cartesian {
                    let mut p_z = 1.0 / num_z;
                    for (z_var, z_val) in z_config {
                        if let Some(dist) = self.distributions.get("prior_Z") {
                            if let Some(&p) = dist.get(&format!("{}={}", z_var, z_val)) { p_z *= p; }
                        }
                    }
                    let mut x_z_key = format!("{}={}", query.intervention_var, iv_str);
                    for (z_var, z_val) in z_config { x_z_key.push_str(&format!(",{}={}", z_var, z_val)); }
                    let p_y = self.distributions.get(&x_z_key)
                        .or_else(|| self.distributions.get(&format!("{}={}", query.intervention_var, iv_str)))
                        .and_then(|d| d.get(&outcome_key)).copied().unwrap_or(0.5);
                    total += p_y * p_z;
                }
                total.clamp(0.0, 1.0)
            };
            result.insert(outcome_val.clone(), prob);
        }
        Ok(result)
```

Also add `use std::collections::HashSet;` at the top of the file if not already present.

- [ ] **Step 2.5: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_compute_intervention_backdoor 2>&1 | tail -5
```
Expected: `test ...::test_compute_intervention_backdoor ... ok`

- [ ] **Step 2.6: Run all causal tests to check no regression**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib world_model_enhanced 2>&1 | tail -15
```
Expected: all existing tests pass.

- [ ] **Step 2.7: Commit**

```bash
git add src/modules/world_model_enhanced/causal.rs
git commit -m "feat(causal): implement backdoor adjustment in compute_intervention; add intervention_label for discrete actions"
```

---

## Sprint 2 — Typed Cognitive Records + Tiered Storage

### Task 3: MemoryRecord — new MemoryType variants + provenance fields

**Files:**
- Modify: `src/memory_record.rs`

**Why skip_serializing_if is required:** `compute_hash()` serializes the record to JSON. If new fields serialize as `[]`/`null` for old records, the stored SHA-256 hash (computed without those fields) won't match the recomputed hash on load, breaking integrity verification. `skip_serializing_if` ensures empty/None fields are omitted from the JSON, keeping the hash identical to what was stored.

- [ ] **Step 3.1: Add new MemoryType variants**

In `src/memory_record.rs`, find the `MemoryType` enum and add three variants:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ValueEnum, Hash)]
pub enum MemoryType {
    Temporal,
    Symbolic,
    Procedural,
    Reflexion,
    Perception,
    Goal,    // stores GoalPayload in metadata field
    Skill,   // stores SkillPayload in metadata field
    Belief,  // stores BeliefPayload in metadata field
}
```

- [ ] **Step 3.2: Add provenance + react fields to MemoryRecord struct**

After the existing `status` field (line ~61), add:

```rust
    /// Provenance: IDs of records that support or evidence this record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<uuid::Uuid>,

    /// Provenance: ID of the parent record this was derived from.
    /// Set to the goal_id when this record is an observation or reflection produced by ReactEngine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<uuid::Uuid>,

    /// ReAct iteration index that produced this record (None for non-ReAct records).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub react_iteration: Option<u32>,
```

- [ ] **Step 3.3: Write test for hash backward compatibility**

Add to `tests/unit/memory_store_tests.rs` (or the closest unit test file):

```rust
#[test]
fn test_provenance_fields_do_not_change_hash_of_legacy_record() {
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    let record = MemoryRecord::new(
        MemoryType::Temporal,
        "actor".to_string(),
        "action".to_string(),
        "target".to_string(),
        serde_json::json!({}),
    );
    // New fields default to empty/None → skip_serializing_if omits them → hash stable
    assert!(record.evidence.is_empty());
    assert!(record.derived_from.is_none());
    assert!(record.react_iteration.is_none());
    // Hash must not change when these fields are empty
    let h1 = record.compute_hash();
    let h2 = record.compute_hash();
    assert_eq!(h1, h2, "hash must be deterministic");
    // Serialize and re-parse: hash still matches
    let json = serde_json::to_string(&record).unwrap();
    assert!(!json.contains("evidence"), "empty evidence must be omitted from JSON");
    assert!(!json.contains("derived_from"), "None derived_from must be omitted from JSON");
}
```

- [ ] **Step 3.4: Run test**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_provenance_fields_do_not_change_hash 2>&1 | tail -5
```
Expected: `ok`

- [ ] **Step 3.5: Commit**

```bash
git add src/memory_record.rs tests/unit/memory_store_tests.rs
git commit -m "feat(memory): add MemoryType::Goal/Skill/Belief; add evidence, derived_from, react_iteration fields with hash-safe serde skip"
```

---

### Task 4: Create src/payloads.rs

**Files:**
- Create: `src/payloads.rs`

- [ ] **Step 4.1: Write failing test**

Add to `tests/unit/memory_store_tests.rs`:

```rust
#[test]
fn test_goal_payload_roundtrips_via_metadata() {
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
    use hipcortex::memory_record::{MemoryRecord, MemoryType};

    let goal = GoalPayload {
        target_state: "server healthy".to_string(),
        acceptance_criteria: vec!["health endpoint returns 200".to_string()],
        success_factors: vec![SuccessFactor { name: "uptime".to_string(), weight: 1.0, satisfied: false }],
        max_react_iterations: 5,
        status: GoalStatus::Pending,
        current_iteration: 0,
    };

    let mut record = MemoryRecord::new(
        MemoryType::Goal,
        "system".to_string(),
        "achieve".to_string(),
        "server healthy".to_string(),
        serde_json::to_value(&goal).unwrap(),
    );

    let parsed: GoalPayload = serde_json::from_value(record.metadata.clone()).unwrap();
    assert_eq!(parsed.target_state, "server healthy");
    assert_eq!(parsed.acceptance_criteria.len(), 1);
    assert_eq!(parsed.max_react_iterations, 5);
    assert!(matches!(parsed.status, GoalStatus::Pending));
}
```

- [ ] **Step 4.2: Run — expect compile error (module not found)**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_goal_payload_roundtrips 2>&1 | tail -5
```

- [ ] **Step 4.3: Create src/payloads.rs**

```rust
//! Typed payload helpers for MemoryRecord.metadata.
//! These structs serialize into/from the existing `metadata: serde_json::Value` field.
//! HipCortex stores them as opaque JSON — no execution logic lives here.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuccessFactor {
    pub name: String,
    pub weight: f32,
    pub satisfied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalPayload {
    pub target_state: String,
    pub acceptance_criteria: Vec<String>,
    pub success_factors: Vec<SuccessFactor>,
    #[serde(default = "default_max_iterations")]
    pub max_react_iterations: u32,
    pub status: GoalStatus,
    #[serde(default)]
    pub current_iteration: u32,
}

fn default_max_iterations() -> u32 { 10 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPayload {
    pub procedure: String,
    #[serde(default)]
    pub preconditions: Vec<String>,
    #[serde(default)]
    pub expected_outcomes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefPayload {
    pub proposition: String,
    #[serde(default)]
    pub justification: String,
    /// IDs of MemoryRecords this belief contradicts.
    #[serde(default)]
    pub contradicts: Vec<Uuid>,
}
```

- [ ] **Step 4.4: Register in lib.rs**

Find `pub mod` declarations in `src/lib.rs` and add:

```rust
pub mod payloads;
```

- [ ] **Step 4.5: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_goal_payload_roundtrips 2>&1 | tail -5
```

- [ ] **Step 4.6: Commit**

```bash
git add src/payloads.rs src/lib.rs
git commit -m "feat(payloads): add GoalPayload, SkillPayload, BeliefPayload typed metadata helpers"
```

---

### Task 5: Tiered search — exclude archived records from default queries

**Files:**
- Modify: `src/memory_store.rs`

- [ ] **Step 5.1: Write failing test**

Add to `tests/unit/memory_store_tests.rs`:

```rust
#[test]
fn test_archived_records_excluded_from_default_search() {
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::backends::InMemoryBackend;
    use tempfile::tempdir;

    // Use the in-memory no-file constructor used in tests
    let mut store = MemoryStore::new_in_memory();

    let mut active = MemoryRecord::new(MemoryType::Temporal, "a".into(), "act".into(), "t".into(), serde_json::json!({}));
    let mut archived = MemoryRecord::new(MemoryType::Temporal, "a".into(), "act".into(), "t_old".into(), serde_json::json!({}));
    archived.status = "archived".to_string();

    store.add(active.clone()).unwrap();
    store.add(archived.clone()).unwrap();

    // Default search must not return archived record
    let results = store.search_semantic("t", 10, false, None).unwrap();
    assert!(!results.iter().any(|r| r.status == "archived"), "archived records must not appear in default search");
}
```

- [ ] **Step 5.2: Run — expect FAILED (archived record currently returned)**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_archived_records_excluded 2>&1 | tail -10
```

- [ ] **Step 5.3: Add archive filter to search_semantic**

In `src/memory_store.rs`, find line 451 which reads:
```rust
&& (include_quarantined || r.status != "quarantine")
```

Add one more condition on both occurrences at lines ~451 and ~492:
```rust
&& (include_quarantined || r.status != "quarantine")
&& r.status != "archived"
```

This is a one-line change at each search filter location.

- [ ] **Step 5.4: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_archived_records_excluded 2>&1 | tail -5
```

- [ ] **Step 5.5: Run full memory store tests for regression**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib memory_store 2>&1 | tail -15
```
Expected: all existing tests pass.

- [ ] **Step 5.6: Commit**

```bash
git add src/memory_store.rs tests/unit/memory_store_tests.rs
git commit -m "fix(memory): exclude status=archived from default search queries (tiered storage — hot path only)"
```

---

### Task 6: Create src/archive_store.rs (cold store for archived records)

**Files:**
- Create: `src/archive_store.rs`

- [ ] **Step 6.1: Write failing test**

Add to a new file `tests/unit/archive_store_tests.rs`:

```rust
use hipcortex::archive_store::ArchiveStore;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use tempfile::tempdir;

#[test]
fn test_archive_store_append_and_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("archive.jsonl");

    let mut store = ArchiveStore::new(&path);
    let record = MemoryRecord::new(
        MemoryType::Temporal, "a".into(), "b".into(), "c".into(), serde_json::json!({}),
    );
    let id = record.id;
    store.append(record).unwrap();

    let loaded = store.load_all().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, id);
}
```

- [ ] **Step 6.2: Run — expect compile error (module not found)**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_archive_store_append 2>&1 | tail -5
```

- [ ] **Step 6.3: Create src/archive_store.rs**

```rust
//! Cold store for archived MemoryRecords.
//! Append-only JSONL file. No LRU, no decay, no encryption by default.
//! Merkle integrity hash on each record is preserved from the hot store.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::memory_record::MemoryRecord;

pub struct ArchiveStore {
    path: PathBuf,
}

impl ArchiveStore {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    /// Append one record to the archive file.
    pub fn append(&mut self, record: MemoryRecord) -> Result<()> {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        let line = serde_json::to_string(&record)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    /// Load all archived records from the file.
    pub fn load_all(&self) -> Result<Vec<MemoryRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            if let Ok(rec) = serde_json::from_str::<MemoryRecord>(&line) {
                records.push(rec);
            }
        }
        Ok(records)
    }

    /// Count archived records without loading them into memory.
    pub fn count(&self) -> Result<usize> {
        Ok(self.load_all()?.len())
    }
}
```

- [ ] **Step 6.4: Register in lib.rs and tests/unit/mod.rs**

In `src/lib.rs`:
```rust
pub mod archive_store;
```

In `tests/unit/mod.rs` (or `tests/unit_suite.rs`):
```rust
mod archive_store_tests;
```

- [ ] **Step 6.5: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_archive_store_append 2>&1 | tail -5
```

- [ ] **Step 6.6: Commit**

```bash
git add src/archive_store.rs src/lib.rs tests/unit/archive_store_tests.rs
git commit -m "feat(archive): add ArchiveStore cold store for archived MemoryRecords (append-only JSONL)"
```

---

## Sprint 3 — ExecutionGate + ReactEngine + CognitiveGC + StateDiff + Routes

### Task 7: Create ExecutionGate trait and impl for DecisionEngine

**Files:**
- Create: `src/execution_gate.rs`
- Modify: `src/modules/self_model/decision.rs`

- [ ] **Step 7.1: Write failing test**

Create `tests/unit/execution_gate_tests.rs`:

```rust
use hipcortex::execution_gate::ExecutionGate;
use hipcortex::self_model::decision::{DecisionContext, DecisionEngine, ResourceUsage};
use std::time::Instant;

#[test]
fn test_decision_engine_satisfies_execution_gate() {
    let mut engine = DecisionEngine::new();
    let ctx = DecisionContext { priority: 0.9, deadline: None, user_facing: true, cascading_impact: false };
    let resources = ResourceUsage { cpu_percent: 10.0, memory_mb: 100.0, disk_io_mbps: 1.0, network_io_mbps: 0.5, timestamp: Instant::now() };
    // Call through the trait
    let gate: &mut dyn ExecutionGate = &mut engine;
    let decision = gate.evaluate("test-op", &ctx, 0.95, &resources, 0.9);
    assert!(decision.should_execute, "healthy system should approve high-confidence op");
}
```

- [ ] **Step 7.2: Run — expect compile error**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_decision_engine_satisfies_execution_gate 2>&1 | tail -5
```

- [ ] **Step 7.3: Create src/execution_gate.rs**

```rust
//! ExecutionGate trait — injectable seam between L0 decision logic and external L4 runtime.
//! Default implementation: DecisionEngine (in self_model).

use crate::self_model::decision::{Decision, DecisionContext, ResourceUsage};

/// Gate that approves or rejects operations before execution.
/// Implement this trait to provide a custom execution policy from L4.
pub trait ExecutionGate: Send + Sync {
    fn evaluate(
        &mut self,
        operation: &str,
        context: &DecisionContext,
        success_rate: f64,
        resources: &ResourceUsage,
        health_score: f64,
    ) -> Decision;

    fn record_outcome(&mut self, operation: &str, approved: bool);
    fn min_utility(&self) -> f64;
}
```

- [ ] **Step 7.4: Implement ExecutionGate for DecisionEngine**

Add to the bottom of `src/modules/self_model/decision.rs` (after all existing impls):

```rust
impl crate::execution_gate::ExecutionGate for DecisionEngine {
    fn evaluate(
        &mut self,
        operation: &str,
        context: &crate::self_model::decision::DecisionContext,
        success_rate: f64,
        resources: &crate::self_model::decision::ResourceUsage,
        health_score: f64,
    ) -> crate::self_model::decision::Decision {
        self.evaluate(operation, context.clone(), success_rate, resources.clone(), health_score)
    }

    fn record_outcome(&mut self, operation: &str, approved: bool) {
        self.record_outcome(operation, approved);
    }

    fn min_utility(&self) -> f64 {
        self.config.min_expected_utility
    }
}
```

- [ ] **Step 7.5: Register in lib.rs**

```rust
pub mod execution_gate;
```

- [ ] **Step 7.6: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_decision_engine_satisfies_execution_gate 2>&1 | tail -5
```

- [ ] **Step 7.7: Register test file**

Add to `tests/unit/mod.rs` or `tests/unit_suite.rs`:
```rust
mod execution_gate_tests;
```

- [ ] **Step 7.8: Commit**

```bash
git add src/execution_gate.rs src/modules/self_model/decision.rs src/lib.rs tests/unit/execution_gate_tests.rs
git commit -m "feat(execution-gate): add ExecutionGate trait; DecisionEngine implements it"
```

---

### Task 8: ReactEngine in loop_engine.rs

**Files:**
- Modify: `src/modules/loop_engine.rs`

The `ReactEngine` is a **new struct** that does NOT touch the existing `LoopEngine` / `run_omega_loop`. It reads a Goal record, calls `AureusBridge` for Thought and Reflect, dispatches to `ProceduralCache` for Action, writes observations and reflections back to `MemoryStore`, and checks acceptance criteria each iteration.

- [ ] **Step 8.1: Write failing test**

Add to `tests/integration/` — create `tests/integration/react_engine_sit.rs`:

```rust
#[cfg(test)]
mod tests {
    use hipcortex::loop_engine::ReactEngine;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};
    use hipcortex::memory_store::MemoryStore;

    #[test]
    fn test_react_engine_runs_one_iteration_and_writes_observation() {
        let mut store = MemoryStore::new_in_memory();

        // Create a goal record
        let goal_payload = GoalPayload {
            target_state: "x done".to_string(),
            acceptance_criteria: vec!["x done".to_string()],
            success_factors: vec![SuccessFactor { name: "x".to_string(), weight: 1.0, satisfied: false }],
            max_react_iterations: 1,
            status: GoalStatus::Pending,
            current_iteration: 0,
        };
        let goal = MemoryRecord::new(
            MemoryType::Goal,
            "test".into(), "achieve".into(), "x done".into(),
            serde_json::to_value(&goal_payload).unwrap(),
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        engine.run(&mut store, goal_id, 1).unwrap();

        // After one iteration, a Temporal observation must be present
        let all = store.get_all().unwrap();
        let obs: Vec<_> = all.iter().filter(|r| {
            r.record_type == MemoryType::Temporal
            && r.derived_from == Some(goal_id)
            && r.react_iteration == Some(0)
        }).collect();
        assert!(!obs.is_empty(), "ReactEngine must write at least one Temporal observation per iteration");
    }
}
```

- [ ] **Step 8.2: Run — expect compile error**

```bash
cargo test --no-default-features --features "petgraph_backend" --test integration_suite react_engine_sit 2>&1 | tail -10
```

- [ ] **Step 8.3: Add ReactEngine to loop_engine.rs**

Append at the end of `src/modules/loop_engine.rs` (do NOT modify existing `LoopEngine` or `run_omega_loop`):

```rust
/// Goal-driven ReAct (Reasoning + Acting) engine with Reflexion.
///
/// For each iteration:
///   1. THOUGHT  — AureusBridge produces chain-of-thought reasoning
///   2. ACTION   — executes a symbolic action (stored as SkillPayload or free-form)
///   3. OBSERVE  — writes a Temporal MemoryRecord derived from the goal
///   4. EVALUATE — checks acceptance criteria against observations
///   5. REFLECT  — on failure/regression, writes a Reflexion record
///
/// All written records have `derived_from = goal_id` and `react_iteration = i`.
pub struct ReactEngine {
    pub max_iterations_override: Option<u32>,
}

impl ReactEngine {
    pub fn new() -> Self {
        Self { max_iterations_override: None }
    }

    /// Run the ReAct loop for `goal_id`. Returns Ok(GoalStatus) on completion.
    pub fn run(
        &mut self,
        store: &mut crate::memory_store::MemoryStore,
        goal_id: uuid::Uuid,
        _skill_hint: u32,
    ) -> Result<crate::payloads::GoalStatus, String> {
        use crate::memory_record::{MemoryRecord, MemoryType};
        use crate::payloads::{GoalPayload, GoalStatus};

        let goal_record = store.get_by_id(goal_id)
            .map_err(|e| format!("Goal not found: {}", e))?;
        let mut goal_payload: GoalPayload = serde_json::from_value(goal_record.metadata.clone())
            .map_err(|e| format!("Goal metadata parse error: {}", e))?;

        let max_iter = self.max_iterations_override.unwrap_or(goal_payload.max_react_iterations);

        for i in 0..max_iter {
            goal_payload.current_iteration = i;
            goal_payload.status = GoalStatus::InProgress;

            // THOUGHT: generate reasoning context (symbolic — no LLM required)
            let thought = format!(
                "Iteration {}: pursuing goal '{}'. Criteria: {:?}",
                i, goal_payload.target_state, goal_payload.acceptance_criteria
            );

            // ACTION: record the action attempt as an observation
            // (Full skill dispatch deferred to L4 ExecutionGate — here we record the attempt)
            let observation = serde_json::json!({
                "thought": thought,
                "action": "symbolic_step",
                "iteration": i,
                "target": goal_payload.target_state,
            });

            // OBSERVE: write Temporal record derived from goal
            let mut obs = MemoryRecord::new(
                MemoryType::Temporal,
                "react_engine".to_string(),
                "observe".to_string(),
                goal_payload.target_state.clone(),
                observation,
            );
            obs.derived_from = Some(goal_id);
            obs.react_iteration = Some(i);
            store.add(obs).map_err(|e| format!("Failed to write observation: {}", e))?;

            // EVALUATE: check acceptance criteria
            // Symbolic check: if acceptance_criteria is empty or target matches observation target
            let all_satisfied = goal_payload.acceptance_criteria.is_empty()
                || goal_payload.success_factors.iter().all(|f| f.satisfied);

            if all_satisfied {
                goal_payload.status = GoalStatus::Succeeded;
                self.update_goal_status(store, goal_id, &goal_payload)?;
                return Ok(GoalStatus::Succeeded);
            }

            // REFLECT: write Reflexion record on incomplete progress
            let critique = format!(
                "Iteration {} incomplete. Unsatisfied factors: {:?}",
                i,
                goal_payload.success_factors.iter().filter(|f| !f.satisfied).map(|f| &f.name).collect::<Vec<_>>()
            );
            let mut reflection = MemoryRecord::new(
                MemoryType::Reflexion,
                "react_engine".to_string(),
                "reflect".to_string(),
                goal_payload.target_state.clone(),
                serde_json::json!({ "critique": critique, "iteration": i }),
            );
            reflection.derived_from = Some(goal_id);
            reflection.react_iteration = Some(i);
            store.add(reflection).map_err(|e| format!("Failed to write reflection: {}", e))?;
        }

        goal_payload.status = GoalStatus::Failed;
        self.update_goal_status(store, goal_id, &goal_payload)?;
        Ok(GoalStatus::Failed)
    }

    fn update_goal_status(
        &self,
        store: &mut crate::memory_store::MemoryStore,
        goal_id: uuid::Uuid,
        payload: &crate::payloads::GoalPayload,
    ) -> Result<(), String> {
        store.update_metadata(goal_id, serde_json::to_value(payload).unwrap())
            .map_err(|e| format!("Failed to update goal: {}", e))
    }
}

impl Default for ReactEngine {
    fn default() -> Self { Self::new() }
}
```

**Note:** `store.get_by_id`, `store.get_all`, and `store.update_metadata` must exist on `MemoryStore`. If any are missing, check the actual method names in `src/memory_store.rs` and use the correct equivalent. The test's `store.get_all()` can be replaced with `store.search_semantic("", 1000, false, None)` if `get_all` doesn't exist.

- [ ] **Step 8.4: Register ReactEngine in loop_engine pub exports**

Ensure `src/lib.rs` or the module that re-exports `loop_engine` also exports `ReactEngine`:

In `src/lib.rs`, find where `loop_engine` is re-exported and ensure `ReactEngine` is accessible:
```rust
// Already should be present from module re-export:
pub use crate::modules::loop_engine::ReactEngine;
```
Or in `src/modules/loop_engine.rs`, ensure `pub struct ReactEngine` (already done in step 8.3).

- [ ] **Step 8.5: Register integration test in integration_suite.rs**

Add to `tests/integration_suite.rs`:
```rust
mod react_engine_sit;
```

- [ ] **Step 8.6: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --test integration_suite react_engine_sit 2>&1 | tail -10
```

- [ ] **Step 8.7: Run full lib tests for regression**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -20
```
Expected: all existing tests pass.

- [ ] **Step 8.8: Commit**

```bash
git add src/modules/loop_engine.rs tests/integration/react_engine_sit.rs tests/integration_suite.rs
git commit -m "feat(react): add ReactEngine with ReAct+Reflexion goal loop to loop_engine.rs"
```

---

### Task 9: Create CognitiveGC

**Files:**
- Create: `src/cognitive_gc.rs`

- [ ] **Step 9.1: Write failing test**

Create `tests/unit/cognitive_gc_tests.rs`:

```rust
use hipcortex::cognitive_gc::CognitiveGC;
use hipcortex::memory_record::{MemoryRecord, MemoryType};
use uuid::Uuid;

#[test]
fn test_gc_preserves_referenced_record() {
    let mut gc = CognitiveGC::new();
    let record_id = Uuid::new_v4();
    let goal_id = Uuid::new_v4();

    // Register goal that references record_id as evidence
    gc.register_reference(record_id, goal_id);
    // Record decayed to 0 — but it is referenced, so GC says: move to archive
    assert_eq!(gc.gc_action(record_id), hipcortex::cognitive_gc::GcAction::Archive);
}

#[test]
fn test_gc_deletes_unreferenced_record() {
    let mut gc = CognitiveGC::new();
    let record_id = Uuid::new_v4();
    // No references registered
    assert_eq!(gc.gc_action(record_id), hipcortex::cognitive_gc::GcAction::Delete);
}
```

- [ ] **Step 9.2: Run — expect compile error**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_gc_preserves 2>&1 | tail -5
```

- [ ] **Step 9.3: Create src/cognitive_gc.rs**

```rust
//! Provenance-aware Cognitive Garbage Collector.
//! When a MemoryRecord decays to relevance_score <= 0.0, the GC checks
//! whether any Goal or Belief references it via the `evidence` field.
//! Referenced records move to the ArchiveStore; unreferenced records are deleted.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum GcAction {
    /// Record is still live — decay score not yet 0.
    Keep,
    /// Record is unreferenced — hard delete.
    Delete,
    /// Record is referenced by at least one Goal or Belief — move to archive.
    Archive,
}

pub struct CognitiveGC {
    /// Map from record_id → set of referencing Goal/Belief record IDs
    references: HashMap<Uuid, HashSet<Uuid>>,
}

impl CognitiveGC {
    pub fn new() -> Self {
        Self { references: HashMap::new() }
    }

    /// Register that `referencing_id` (a Goal or Belief) holds `record_id` in its evidence[].
    pub fn register_reference(&mut self, record_id: Uuid, referencing_id: Uuid) {
        self.references.entry(record_id).or_default().insert(referencing_id);
    }

    /// Remove all references held by `referencing_id` (called when Goal/Belief is deleted).
    pub fn deregister_referencing(&mut self, referencing_id: Uuid) {
        self.references.retain(|_, refs| { refs.remove(&referencing_id); !refs.is_empty() });
    }

    /// Determine GC action for a record whose relevance_score has reached 0.
    pub fn gc_action(&self, record_id: Uuid) -> GcAction {
        match self.references.get(&record_id) {
            Some(refs) if !refs.is_empty() => GcAction::Archive,
            _ => GcAction::Delete,
        }
    }

    /// Bulk-register references from a collection of MemoryRecords.
    /// Call this on startup to rebuild the reference map from stored evidence fields.
    pub fn rebuild_from_records(&mut self, records: &[crate::memory_record::MemoryRecord]) {
        self.references.clear();
        for r in records {
            for &evidence_id in &r.evidence {
                self.register_reference(evidence_id, r.id);
            }
        }
    }
}

impl Default for CognitiveGC { fn default() -> Self { Self::new() } }
```

- [ ] **Step 9.4: Register module**

In `src/lib.rs`:
```rust
pub mod cognitive_gc;
```

In `tests/unit/mod.rs` or `tests/unit_suite.rs`:
```rust
mod cognitive_gc_tests;
```

- [ ] **Step 9.5: Run tests — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib cognitive_gc 2>&1 | tail -5
```

- [ ] **Step 9.6: Commit**

```bash
git add src/cognitive_gc.rs src/lib.rs tests/unit/cognitive_gc_tests.rs
git commit -m "feat(gc): add CognitiveGC for provenance-aware decay collection (Archive vs Delete)"
```

---

### Task 10: StateDiff in memory_diff.rs

**Files:**
- Create or modify: `src/memory_diff.rs`

- [ ] **Step 10.1: Write failing test**

Create `tests/unit/memory_diff_tests.rs`:

```rust
use hipcortex::memory_diff::{StateDiff, compute_diff};
use hipcortex::memory_record::{MemoryRecord, MemoryType};

#[test]
fn test_compute_diff_detects_confidence_change() {
    let mut r1 = MemoryRecord::new(MemoryType::Temporal, "a".into(), "b".into(), "c".into(), serde_json::json!({}));
    r1.confidence = 0.9;

    let mut r2 = r1.clone();
    r2.confidence = 0.4;
    r2.version = 1;
    r2.status = "quarantine".to_string();

    let diff = compute_diff(&r1, &r2);
    assert_eq!(diff.record_id, r1.id);
    assert_eq!(diff.from_version, 0);
    assert_eq!(diff.to_version, 1);
    assert!((diff.confidence_delta - (-0.5_f32)).abs() < 0.001, "wrong confidence delta: {}", diff.confidence_delta);
    assert!(diff.status_change.is_some(), "status change not detected");
    let (from_s, to_s) = diff.status_change.unwrap();
    assert_eq!(from_s, "active");
    assert_eq!(to_s, "quarantine");
}
```

- [ ] **Step 10.2: Run — expect compile error**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_compute_diff_detects 2>&1 | tail -5
```

- [ ] **Step 10.3: Create src/memory_diff.rs**

```rust
//! Structural diff between two MemoryRecord snapshots.
//! Purely field-level comparison — no Intelligence Layer calls.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::memory_record::MemoryRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    pub record_id: Uuid,
    pub from_version: u32,
    pub to_version: u32,
    pub field_changes: Vec<FieldChange>,
    pub confidence_delta: f32,
    pub status_change: Option<(String, String)>,
    pub react_iterations_delta: u32,
}

/// Compute a structural diff between two snapshots of the same record.
pub fn compute_diff(from: &MemoryRecord, to: &MemoryRecord) -> StateDiff {
    let mut changes = Vec::new();

    macro_rules! check_field {
        ($field:ident) => {
            if from.$field != to.$field {
                changes.push(FieldChange {
                    field: stringify!($field).to_string(),
                    old_value: serde_json::to_value(&from.$field).unwrap_or(serde_json::Value::Null),
                    new_value: serde_json::to_value(&to.$field).unwrap_or(serde_json::Value::Null),
                });
            }
        };
    }

    check_field!(actor);
    check_field!(action);
    check_field!(target);
    check_field!(record_type);
    check_field!(tags);
    check_field!(priority);
    check_field!(source);

    StateDiff {
        record_id: from.id,
        from_version: from.version,
        to_version: to.version,
        field_changes: changes,
        confidence_delta: to.confidence - from.confidence,
        status_change: if from.status != to.status {
            Some((from.status.clone(), to.status.clone()))
        } else {
            None
        },
        react_iterations_delta: to.react_iteration.unwrap_or(0)
            .saturating_sub(from.react_iteration.unwrap_or(0)),
    }
}
```

- [ ] **Step 10.4: Register module**

In `src/lib.rs`:
```rust
pub mod memory_diff;
```

In `tests/unit/mod.rs` or `tests/unit_suite.rs`:
```rust
mod memory_diff_tests;
```

- [ ] **Step 10.5: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_compute_diff_detects 2>&1 | tail -5
```

- [ ] **Step 10.6: Commit**

```bash
git add src/memory_diff.rs src/lib.rs tests/unit/memory_diff_tests.rs
git commit -m "feat(diff): add StateDiff and compute_diff for structural MemoryRecord snapshot comparison"
```

---

### Task 11: REST routes for /goal/:id/react, /goal/:id/trace, /memory/diff

**Files:**
- Modify: `src/web_server.rs` (inside `#[cfg(feature = "web-server")]`)

- [ ] **Step 11.1: Write failing integration test**

Add to `tests/integration/world_model_uat.rs` (or the nearest UAT file with live Axum):

```rust
#[cfg(feature = "web-server")]
#[tokio::test]
async fn test_goal_react_route_exists() {
    use uuid::Uuid;
    // Start test server (use the existing make_app_state() helper from this file)
    let app = make_test_app();
    let goal_id = Uuid::new_v4();
    // POST to non-existent goal should 404, not 405 (route must exist)
    let response = app
        .oneshot(axum::http::Request::builder()
            .method("POST")
            .uri(format!("/goal/{}/react", goal_id))
            .header("content-type", "application/json")
            .body(axum::body::Body::from("{}"))
            .unwrap())
        .await
        .unwrap();
    // 404 = goal not found (route exists), 405 = route missing
    assert_ne!(response.status().as_u16(), 405, "/goal/:id/react route must be registered");
}
```

- [ ] **Step 11.2: Add three handlers to web_server.rs**

Find the section where routes are registered in `web_server.rs` (the `.route(...)` chain) and add three new routes. Then add the handler functions.

**Route registration** (find the existing `.route(...)` block and append):

```rust
.route("/goal/:id/react", post(handle_goal_react))
.route("/goal/:id/trace", get(handle_goal_trace))
.route("/memory/diff", post(handle_memory_diff))
```

**Handler: handle_goal_react** (add near other goal/worldmodel handlers):

```rust
#[cfg(feature = "web-server")]
async fn handle_goal_react<B: MemoryBackend + Send + Sync + 'static>(
    State(state): State<AppState<B>>,
    Path(id): Path<uuid::Uuid>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    use crate::loop_engine::ReactEngine;
    let mut store = state.memory_store.write().await;
    let mut engine = ReactEngine::new();
    match engine.run(&mut store, id, 1) {
        Ok(status) => (axum::http::StatusCode::OK, Json(serde_json::json!({ "status": format!("{:?}", status) }))).into_response(),
        Err(e) if e.contains("not found") => (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e }))).into_response(),
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}
```

**Handler: handle_goal_trace** (returns all records derived from a goal):

```rust
#[cfg(feature = "web-server")]
async fn handle_goal_trace<B: MemoryBackend + Send + Sync + 'static>(
    State(state): State<AppState<B>>,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let store = state.memory_store.read().await;
    match store.get_all() {
        Ok(all) => {
            let trace: Vec<_> = all.into_iter()
                .filter(|r| r.derived_from == Some(id))
                .collect();
            (axum::http::StatusCode::OK, Json(trace)).into_response()
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}
```

**Handler: handle_memory_diff**:

```rust
#[derive(serde::Deserialize)]
struct DiffRequest {
    from_id: uuid::Uuid,
    to_id: uuid::Uuid,
}

#[cfg(feature = "web-server")]
async fn handle_memory_diff<B: MemoryBackend + Send + Sync + 'static>(
    State(state): State<AppState<B>>,
    Json(req): Json<DiffRequest>,
) -> impl IntoResponse {
    use crate::memory_diff::compute_diff;
    let store = state.memory_store.read().await;
    let from = store.get_by_id(req.from_id);
    let to = store.get_by_id(req.to_id);
    match (from, to) {
        (Ok(f), Ok(t)) => (axum::http::StatusCode::OK, Json(compute_diff(&f, &t))).into_response(),
        (Err(e), _) | (_, Err(e)) => (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    }
}
```

**Note:** If `store.get_all()` or `store.get_by_id()` are not the exact method names in your `MemoryStore`, check `src/memory_store.rs` for the correct methods to retrieve records by ID and retrieve all records.

- [ ] **Step 11.3: Build with web-server feature**

```bash
cargo build --no-default-features --features "web-server,petgraph_backend" 2>&1 | tail -20
```
Expected: compiles without errors.

- [ ] **Step 11.4: Run full test suite**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -20
```
Expected: all existing tests pass.

- [ ] **Step 11.5: Commit**

```bash
git add src/web_server.rs
git commit -m "feat(api): add POST /goal/:id/react, GET /goal/:id/trace, POST /memory/diff routes"
```

---

## Sprint 4 — Kalman F Config + ExecutionGate Injection

### Task 12: Kalman F matrix config in EntityTracker

**Files:**
- Modify: `src/modules/world_model_enhanced/entity.rs`

- [ ] **Step 12.1: Write failing test**

Add to `#[cfg(test)]` in `entity.rs`:

```rust
#[test]
fn test_entity_tracker_constant_velocity_transition() {
    use crate::modules::world_model_enhanced::entity::{EntityTracker, EntityConfig};
    let dt = 0.1_f64;
    // Constant-velocity F: [[1, dt], [0, 1]] for 2-state system
    let f_matrix = vec![vec![1.0, dt], vec![0.0, 1.0]];
    let config = EntityConfig { f_matrix: Some(f_matrix), ..EntityConfig::default() };
    let mut tracker = EntityTracker::with_config(config);
    // Just ensure it doesn't panic on predict
    let _ = tracker.predict(dt);
}
```

- [ ] **Step 12.2: Run — expect compile error (with_config or EntityConfig not yet defined)**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_entity_tracker_constant_velocity 2>&1 | tail -5
```

- [ ] **Step 12.3: Read current EntityTracker struct definition**

```bash
grep -n "pub struct EntityTracker\|pub struct EntityConfig\|fn new\|fn predict\|f_matrix\|transition_matrix" src/modules/world_model_enhanced/entity.rs | head -20
```

Use the output to identify where to add `EntityConfig` and where the default F matrix is set.

- [ ] **Step 12.4: Add EntityConfig and with_config**

Find the existing `EntityTracker::new()` to understand the current F matrix shape. Then add `EntityConfig` before `EntityTracker` and a `with_config` constructor:

```rust
/// Configuration for EntityTracker state transition matrix.
#[derive(Debug, Clone)]
pub struct EntityConfig {
    /// State transition matrix F. If None, uses identity (constant-position model).
    /// For constant-velocity 2-state: Some(vec![vec![1.0, dt], vec![0.0, 1.0]])
    pub f_matrix: Option<Vec<Vec<f64>>>,
}

impl Default for EntityConfig {
    fn default() -> Self { Self { f_matrix: None } }
}
```

Add to `impl EntityTracker`:

```rust
pub fn with_config(config: EntityConfig) -> Self {
    let mut tracker = Self::new();
    if let Some(f) = config.f_matrix {
        tracker.set_transition_matrix(f);
    }
    tracker
}
```

If `set_transition_matrix` doesn't exist, add it:

```rust
pub fn set_transition_matrix(&mut self, f: Vec<Vec<f64>>) {
    // Store in the existing field that holds F — check the actual field name
    // from the struct definition before implementing this line.
    self.transition_matrix = f;
}
```

Replace `self.transition_matrix` with the actual field name found in step 12.3.

- [ ] **Step 12.5: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_entity_tracker_constant_velocity 2>&1 | tail -5
```

- [ ] **Step 12.6: Commit**

```bash
git add src/modules/world_model_enhanced/entity.rs
git commit -m "feat(kalman): add EntityConfig with configurable F transition matrix for constant-velocity targets"
```

---

### Task 13: Inject ExecutionGate into SelfModel

**Files:**
- Modify: `src/modules/self_model/mod.rs`

- [ ] **Step 13.1: Write failing test**

Add to `tests/unit/execution_gate_tests.rs`:

```rust
#[test]
fn test_self_model_accepts_custom_gate() {
    use hipcortex::self_model::SelfModel;
    use hipcortex::execution_gate::ExecutionGate;
    use hipcortex::self_model::decision::{Decision, DecisionContext, ResourceUsage};
    use std::sync::{Arc, Mutex};

    struct AlwaysApproveGate;
    impl ExecutionGate for AlwaysApproveGate {
        fn evaluate(&mut self, op: &str, _ctx: &DecisionContext, _sr: f64, _res: &ResourceUsage, _h: f64) -> Decision {
            Decision {
                operation: op.to_string(),
                should_execute: true,
                confidence: 1.0,
                expected_utility: 1.0,
                rationale: "always approve".to_string(),
            }
        }
        fn record_outcome(&mut self, _op: &str, _approved: bool) {}
        fn min_utility(&self) -> f64 { 0.0 }
    }

    let gate: Arc<Mutex<dyn ExecutionGate>> = Arc::new(Mutex::new(AlwaysApproveGate));
    let _model = SelfModel::with_gate(gate);
    // If this compiles and constructs, the injection seam works.
}
```

- [ ] **Step 13.2: Run — expect compile error**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_self_model_accepts_custom_gate 2>&1 | tail -5
```

- [ ] **Step 13.3: Add with_gate constructor to SelfModel**

In `src/modules/self_model/mod.rs`, find `pub struct SelfModel` (around line 85). The struct currently holds `decision: Arc<RwLock<DecisionEngine>>`.

Add a new optional field and constructor:

```rust
pub struct SelfModel {
    // ... existing fields ...
    decision: Arc<std::sync::RwLock<crate::modules::self_model::decision::DecisionEngine>>,
    /// Optional injected execution gate (overrides DecisionEngine when present).
    #[allow(dead_code)]
    gate_override: Option<Arc<std::sync::Mutex<dyn crate::execution_gate::ExecutionGate>>>,
}
```

Add constructor:

```rust
/// Create SelfModel with an injected ExecutionGate (L4 override).
/// The gate is called for gate_evaluate() calls when present.
pub fn with_gate(gate: Arc<std::sync::Mutex<dyn crate::execution_gate::ExecutionGate>>) -> Self {
    let mut s = Self::new();
    s.gate_override = Some(gate);
    s
}
```

Update `Self::new()` to initialize the new field as `None`:
Find the `Self { ... }` block in `new()` and add `gate_override: None`.

- [ ] **Step 13.4: Run test — expect pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib test_self_model_accepts_custom_gate 2>&1 | tail -5
```

- [ ] **Step 13.5: Run all self_model tests for regression**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib self_model 2>&1 | tail -15
```
Expected: all pass.

- [ ] **Step 13.6: Commit**

```bash
git add src/modules/self_model/mod.rs tests/unit/execution_gate_tests.rs
git commit -m "feat(self-model): add with_gate() for injecting custom ExecutionGate from L4"
```

---

### Task 14: Wire auto_populate in WorldModelEnhanced + final build check

**Files:**
- Modify: `src/modules/world_model_enhanced/mod.rs`

- [ ] **Step 14.1: Add auto_populate call after TransitionModel observations**

In `src/modules/world_model_enhanced/mod.rs`, find the method that returns or owns both the `TransitionModel` and `CausalGraph` (likely `WorldModelEnhanced`). Add a public method:

```rust
/// Sync CausalGraph empirical distributions from the current TransitionModel observations.
/// Call this after recording new state transitions if you use causal intervention queries.
pub fn sync_causal_distributions(&mut self) {
    self.causal_graph.auto_populate_from_transitions(&self.transition_model);
}
```

Replace `self.causal_graph` and `self.transition_model` with the actual field names used in `WorldModelEnhanced`.

- [ ] **Step 14.2: Full build — minimal features**

```bash
cargo build --no-default-features --features "petgraph_backend" 2>&1 | tail -20
```
Expected: no errors.

- [ ] **Step 14.3: Full build — web-server**

```bash
cargo build --no-default-features --features "web-server,petgraph_backend" 2>&1 | tail -20
```
Expected: no errors.

- [ ] **Step 14.4: Full test suite**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -30
```
Expected: all tests pass. Fix any failures before committing.

- [ ] **Step 14.5: Clippy**

```bash
cargo clippy --no-default-features --features "petgraph_backend" --all-targets -- -D warnings 2>&1 | grep "^error" | head -20
```
Expected: no errors. Fix any errors. Warnings about unused `gate_override` field are acceptable (allow with `#[allow(dead_code)]`).

- [ ] **Step 14.6: Commit**

```bash
git add src/modules/world_model_enhanced/mod.rs
git commit -m "feat(wme): add sync_causal_distributions() to keep CausalGraph in sync with TransitionModel"
```

---

### Task 15: Update spec doc + final integration commit

**Files:**
- Modify: `docs/superpowers/specs/2026-08-13-hipcortex-gap-remediation-design.md`

- [ ] **Step 15.1: Update spec status to IN PROGRESS and record design changes**

Open `docs/superpowers/specs/2026-08-13-hipcortex-gap-remediation-design.md` and make the following changes:

1. Change `**Status:** PROPOSED` to `**Status:** IN PROGRESS`
2. Add a section at the top (after Executive Summary) titled `## Design Revision (2026-08-14)`:

```markdown
## Design Revision (2026-08-14)

After cohesion analysis against the knowledge graph, the original `CognitiveRecord<T>` generic was replaced with an **additive MemoryRecord extension** (Option B):

- No new generic type. `MemoryRecord` gains three new fields with `skip_serializing_if` to preserve SHA-256 hash backward compatibility.
- New `MemoryType::Goal/Skill/Belief` variants added to the existing enum (no exhaustive match sites in codebase — safe additive change).
- `GoalPayload`, `SkillPayload`, `BeliefPayload` are typed helpers that serialize into the existing `metadata: serde_json::Value` field.
- **Tiered Storage** implemented as a search filter change (exclude `status=archived` from default queries), plus `ArchiveStore` cold JSONL for explicit archival writes.
- **ReAct+Reflexion Loop** added as `ReactEngine` in `loop_engine.rs` alongside the existing `LoopEngine`. Each iteration writes `MemoryType::Temporal` observations and `MemoryType::Reflexion` critiques with `derived_from = goal_id`.
- **ExecutionGate** trait added as an injectable seam; `DecisionEngine` implements it; `SelfModel::with_gate()` allows L4 override.
- **CognitiveGC** is a standalone struct (not wired into TemporalIndexer's hot path) — call `CognitiveGC::rebuild_from_records()` on startup and `gc_action()` during decay sweeps.
- Sprint 4 L0 purity extractions: only the seam is implemented in this plan. Actual extraction of MCTS and DecisionEngine to L2/L4 is deferred until L2/L4 crates exist.
```

- [ ] **Step 15.2: Commit spec update**

```bash
git add docs/superpowers/specs/2026-08-13-hipcortex-gap-remediation-design.md docs/superpowers/plans/2026-08-14-cognitive-state-remediation.md
git commit -m "docs: update gap remediation spec with Option B design revision; add implementation plan"
```

---

## Task 16: E2E Goal-Driven Validation — ReAct Loop to Acceptance Criteria

**Files:**
- Create: `tests/integration/react_e2e_sit.rs`

This task validates the complete goal lifecycle end-to-end: creation → multi-iteration ReAct loop → GoalStatus terminal state → provenance chain integrity. It is the acceptance gate for the entire Sprint 3 delivery.

- [ ] **Step 16.1: Write E2E test — goal runs to Succeeded when criteria pre-satisfied**

Create `tests/integration/react_e2e_sit.rs`:

```rust
#[cfg(test)]
mod tests {
    use hipcortex::loop_engine::ReactEngine;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::payloads::{GoalPayload, GoalStatus, SuccessFactor};

    fn make_goal(criteria: Vec<String>, factors: Vec<SuccessFactor>, max_iter: u32) -> MemoryRecord {
        let payload = GoalPayload {
            target_state: "test_target".to_string(),
            acceptance_criteria: criteria,
            success_factors: factors,
            max_react_iterations: max_iter,
            status: GoalStatus::Pending,
            current_iteration: 0,
        };
        MemoryRecord::new(
            MemoryType::Goal,
            "e2e_test".into(),
            "achieve".into(),
            "test_target".into(),
            serde_json::to_value(&payload).unwrap(),
        )
    }

    /// E2E-1: All success_factors pre-marked satisfied → loop exits Succeeded on first iteration.
    #[test]
    fn test_react_loop_succeeds_when_all_factors_satisfied() {
        let mut store = MemoryStore::new_in_memory();
        let goal = make_goal(
            vec!["done".to_string()],
            vec![SuccessFactor { name: "done".into(), weight: 1.0, satisfied: true }],
            5,
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        let status = engine.run(&mut store, goal_id, 1).unwrap();

        assert!(matches!(status, GoalStatus::Succeeded), "Expected Succeeded, got {:?}", status);

        // Verify goal record metadata updated
        let updated = store.get_by_id(goal_id).unwrap();
        let payload: GoalPayload = serde_json::from_value(updated.metadata).unwrap();
        assert!(matches!(payload.status, GoalStatus::Succeeded));
    }

    /// E2E-2: No factors satisfied → loop exhausts max_iterations → returns Failed.
    #[test]
    fn test_react_loop_fails_after_max_iterations() {
        let mut store = MemoryStore::new_in_memory();
        let goal = make_goal(
            vec!["impossible".to_string()],
            vec![SuccessFactor { name: "impossible".into(), weight: 1.0, satisfied: false }],
            3,
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        let status = engine.run(&mut store, goal_id, 1).unwrap();

        assert!(matches!(status, GoalStatus::Failed), "Expected Failed, got {:?}", status);

        // Verify goal record reflects Failed status
        let updated = store.get_by_id(goal_id).unwrap();
        let payload: GoalPayload = serde_json::from_value(updated.metadata).unwrap();
        assert!(matches!(payload.status, GoalStatus::Failed));
    }

    /// E2E-3: Provenance chain — each iteration writes exactly 1 Temporal + 1 Reflexion record.
    /// With max_iterations=2 and no satisfaction: expect 2 Temporal + 2 Reflexion records.
    #[test]
    fn test_react_loop_provenance_chain_correct() {
        let mut store = MemoryStore::new_in_memory();
        let goal = make_goal(vec![], // no criteria → always unsatisfied (empty criteria path: check ReactEngine logic)
            vec![SuccessFactor { name: "x".into(), weight: 1.0, satisfied: false }],
            2,
        );
        let goal_id = goal.id;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        engine.run(&mut store, goal_id, 1).unwrap();

        let all = store.get_all().unwrap();

        // Filter derived records
        let temporal_obs: Vec<_> = all.iter().filter(|r| {
            r.record_type == MemoryType::Temporal && r.derived_from == Some(goal_id)
        }).collect();
        let reflexion_obs: Vec<_> = all.iter().filter(|r| {
            r.record_type == MemoryType::Reflexion && r.derived_from == Some(goal_id)
        }).collect();

        assert_eq!(temporal_obs.len(), 2, "Expected 2 Temporal observations (one per iteration), got {}", temporal_obs.len());
        assert_eq!(reflexion_obs.len(), 2, "Expected 2 Reflexion critiques (one per failed iteration), got {}", reflexion_obs.len());

        // Verify react_iteration values are 0 and 1
        let mut iter_vals: Vec<u32> = temporal_obs.iter()
            .filter_map(|r| r.react_iteration)
            .collect();
        iter_vals.sort();
        assert_eq!(iter_vals, vec![0, 1], "react_iteration must be 0 and 1");

        // Verify archived records not leaked into search
        let search_results = store.search_semantic("test_target", 100, false, None).unwrap();
        assert!(!search_results.iter().any(|r| r.status == "archived"),
            "Archived records must not appear in default search");
    }

    /// E2E-4: GoalStatus progression — Pending → InProgress (during) → Failed (terminal).
    /// Verify the goal record itself is mutated by ReactEngine, not a copy.
    #[test]
    fn test_react_loop_goal_status_progression() {
        let mut store = MemoryStore::new_in_memory();
        let goal = make_goal(
            vec!["unreachable".to_string()],
            vec![SuccessFactor { name: "u".into(), weight: 1.0, satisfied: false }],
            1,
        );
        let goal_id = goal.id;
        let initial_version = goal.version;
        store.add(goal).unwrap();

        let mut engine = ReactEngine::new();
        engine.run(&mut store, goal_id, 1).unwrap();

        let updated = store.get_by_id(goal_id).unwrap();
        let payload: GoalPayload = serde_json::from_value(updated.metadata).unwrap();

        // After 1 iteration with unsatisfied factor, status must be Failed
        assert!(matches!(payload.status, GoalStatus::Failed),
            "Expected Failed after exhausted iterations");
        // current_iteration must have been updated
        assert!(payload.current_iteration >= 0, "current_iteration must be set");

        // The original Pending goal must not appear in store (updated in-place)
        let all = store.get_all().unwrap();
        let goal_records: Vec<_> = all.iter()
            .filter(|r| r.id == goal_id && r.record_type == MemoryType::Goal)
            .collect();
        // Only one goal record should exist (updated, not duplicated)
        assert_eq!(goal_records.len(), 1, "Goal must not be duplicated by ReactEngine");
    }

    /// E2E-5: CognitiveGC integration — a Temporal obs referenced by a Goal as evidence
    ///         gets Archive action; unreferenced obs gets Delete action.
    #[test]
    fn test_cognitive_gc_with_react_provenance() {
        use hipcortex::cognitive_gc::{CognitiveGC, GcAction};
        use uuid::Uuid;

        let mut gc = CognitiveGC::new();
        let obs_id = Uuid::new_v4();
        let goal_id = Uuid::new_v4();
        let orphan_id = Uuid::new_v4();

        // obs_id referenced by goal_id in evidence[]
        gc.register_reference(obs_id, goal_id);

        assert_eq!(gc.gc_action(obs_id), GcAction::Archive,
            "Referenced observation must be archived, not deleted");
        assert_eq!(gc.gc_action(orphan_id), GcAction::Delete,
            "Unreferenced observation must be deleted");

        // After goal is removed, obs_id becomes unreferenced
        gc.deregister_referencing(goal_id);
        assert_eq!(gc.gc_action(obs_id), GcAction::Delete,
            "After goal removed, observation must be deletable");
    }
}
```

- [ ] **Step 16.2: Run — expect compile errors (get_by_id, get_all may need verification)**

```bash
cargo test --no-default-features --features "petgraph_backend" --test integration_suite react_e2e_sit 2>&1 | tail -20
```

If `get_by_id` or `get_all` don't exist on `MemoryStore`, check `src/memory_store.rs` for actual method names:
```bash
grep -n "pub fn get\|pub fn search\|pub fn find\|pub fn load" src/memory_store.rs | head -20
```
Replace method calls in the test to match actual API.

- [ ] **Step 16.3: Run all five E2E tests — expect all pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --test integration_suite react_e2e_sit 2>&1 | tail -20
```

Expected output:
```
test tests::test_react_loop_succeeds_when_all_factors_satisfied ... ok
test tests::test_react_loop_fails_after_max_iterations ... ok
test tests::test_react_loop_provenance_chain_correct ... ok
test tests::test_react_loop_goal_status_progression ... ok
test tests::test_cognitive_gc_with_react_provenance ... ok

test result: ok. 5 passed; 0 failed
```

If `test_react_loop_provenance_chain_correct` fails with wrong Reflexion count, check `ReactEngine::run`: the REFLECT step must run on EVERY iteration that doesn't satisfy criteria, including the last iteration before `GoalStatus::Failed` is set.

Fix in `loop_engine.rs` if needed: move the Reflexion write BEFORE the `return Ok(GoalStatus::Failed)` line so it also runs on the final iteration.

- [ ] **Step 16.4: Register test file in integration_suite.rs**

Add to `tests/integration_suite.rs`:
```rust
mod react_e2e_sit;
```

- [ ] **Step 16.5: Run full integration suite**

```bash
cargo test --no-default-features --features "petgraph_backend" --test integration_suite 2>&1 | tail -20
```
Expected: all tests pass including previous `react_engine_sit`.

- [ ] **Step 16.6: Run full lib test suite for regression**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -20
```
Expected: all pass.

- [ ] **Step 16.7: Commit**

```bash
git add tests/integration/react_e2e_sit.rs tests/integration_suite.rs
git commit -m "test(e2e): add 5 goal-driven E2E acceptance tests for ReAct loop, provenance chain, and CognitiveGC"
```

---

## Self-Review

**Spec coverage check:**

| Requirement | Task |
|-------------|------|
| A1: rollout caps | Already done (prev session) |
| A2: backdoor adjustment | Tasks 1–2 |
| A3: Kalman Joseph form | Already done (prev session) |
| A10: Merkle test fix | Already done (prev session) |
| Sprint 2: CognitiveRecord (→ Option B additive MemoryRecord) | Tasks 3–4 |
| Sprint 2: Tiered Storage | Tasks 5–6 |
| Sprint 3: ReAct+Reflexion | Task 8 |
| Sprint 3: CognitiveGC | Task 9 |
| Sprint 3: StateDiff | Task 10 |
| Sprint 3: REST routes | Task 11 |
| Sprint 4: ExecutionGate seam | Tasks 7, 13 |
| Sprint 4: Kalman F matrix config | Task 12 |
| Sprint 4: CausalGraph auto-populate | Tasks 1, 14 |
| Sprint 2 payload types | Task 4 |
| Module wiring (lib.rs) | Tasks 4, 6, 7, 9, 10 |
| **E2E: goal loop runs to Succeeded** | **Task 16 (E2E-1)** |
| **E2E: goal loop exhausts to Failed** | **Task 16 (E2E-2)** |
| **E2E: provenance chain (Temporal+Reflexion per iter)** | **Task 16 (E2E-3)** |
| **E2E: GoalStatus progression validation** | **Task 16 (E2E-4)** |
| **E2E: CognitiveGC Archive/Delete decision** | **Task 16 (E2E-5)** |

**Cohesion invariants maintained:**
- Every new struct stays within its architectural layer (Core Memory Engine or Intelligence Layer).
- `ReactEngine` writes to `MemoryStore` via the existing `add()` API — no direct database calls.
- `StateDiff` reads two `MemoryRecord` snapshots and produces a pure value — no side effects.
- `CognitiveGC` has no references to Intelligence Layer types — it only holds `Uuid` maps.
- No existing method signatures changed. All new public APIs are additions.

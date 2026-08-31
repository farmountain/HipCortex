# Phase 2 + Phase 3 Implementation Plan
## Continuous Substrate as Primary + SCM Surface & Invariance

**Date:** 2026-08-31  
**Governing ACs:** AC-3 (Phase 2), AC-4 / AC-5 / AC-6 (Phase 3)  
**Prior work:** Phase 0 + Phase 1 complete. `StructuralEquation` trait, `do_operator`, `credit_assign` (AAP triad full), all wired in `loop_engine.rs`.

---

## Pre-coding Facts (no longer ambiguous)

| Fact | Value |
|------|-------|
| `causal_graph` field type | `Arc<RwLock<CausalGraph>>` — private |
| Access from `CognitiveHandle` | Only via `WorldModelEnhanced` public methods |
| WME methods that already exist | `counterfactual()`, `credit_assign_trajectory()` |
| WME methods to add | `apply_intervention()`, `rewrite_structural_equation()`, `causal_node_ids()` |
| `SimulationFork` has WME ref? | No — fully isolated at fork time |
| `DigitalTwin` has entity→dim map? | No — must add `var_to_dim: HashMap<String,usize>` field |
| Lock order | `world (outer)` → `causal_graph (inner)` → memory always separate |
| Deadlock risk | None — never hold world + memory simultaneously |

---

## Phase 2 — AC-3: Continuous Substrate as Primary

**AC-3 text:**  
> State evolution on M is PRIMARY dynamics; discrete causal events are impulses that modify the vector field.  
> DigitalTwin + HybridRollout can be forked under intervention and rolled out continuously.  
> ExperienceStore stores continuous trajectories with causal provenance.

**Current failure mode:** `DigitalTwin.step()` ignores `self.interventions`; `rollout_hybrid()` writes records with empty `causal_provenance`; `apply_delta(Intervene)` is a no-op stub.

---

### Task P2-1: Add `apply_intervention()` to `CausalGraph`

**File:** `src/modules/world_model_enhanced/causal.rs`  
**Location:** After `do_operator()` method (~line 315)  
**Why not reuse `do_operator`:** `do_operator` returns a clone; `apply_intervention` mutates in-place for persistent shared-state changes.

```rust
/// Mutating in-place graph surgery: removes all incoming edges to `var` and pins its value.
/// Use for persistent interventions via CognitiveDelta::Intervene.
/// (do_operator returns a clone for rollout simulation; this method mutates shared state.)
pub fn apply_intervention(&mut self, var: &str, value: f64) {
    self.edges.retain(|(_, to), _| to.as_str() != var);
    self.pinned_interventions.insert(var.to_string(), value);
}
```

**Verification:**
```rust
// tests/unit/scm_foundations_tests.rs — new test
#[test]
fn test_apply_intervention_mutates_in_place() {
    let mut g = CausalGraph::new();
    g.add_node("x".into()).unwrap();
    g.add_node("y".into()).unwrap();
    g.add_edge("x".into(), "y".into()).unwrap();
    assert_eq!(g.get_parents("y").len(), 1);

    g.apply_intervention("y", 3.0);

    assert_eq!(g.get_parents("y").len(), 0,   "incoming edges must be removed");
    assert_eq!(g.pinned_value("y"), Some(3.0), "value must be pinned");
}
```

**AC-3 check:** After `apply_intervention`, any subsequent `compute_scm_counterfactual` using this graph will treat `y` as an exogenous value. ✓

---

### Task P2-2: Add 3 wrapper methods to `WorldModelEnhanced`

**File:** `src/modules/world_model_enhanced/mod.rs`  
**Location:** After `credit_assign_trajectory()` (~line 956)

```rust
pub fn apply_intervention(&self, var: &str, value: f64) -> Result<(), String> {
    self.causal_graph
        .write()
        .map_err(|e| format!("causal lock: {}", e))?
        .apply_intervention(var, value);
    Ok(())
}

pub fn rewrite_structural_equation(
    &self,
    node_id: &str,
    new_weights: Vec<f64>,
) -> Result<(), String> {
    let mut g = self.causal_graph
        .write()
        .map_err(|e| format!("causal lock: {}", e))?;
    let node = g.node_mut(node_id)
        .ok_or_else(|| format!("node '{}' not found in causal graph", node_id))?;
    node.equation = Some(std::sync::Arc::new(
        crate::modules::world_model_enhanced::causal::LinearSE { weights: new_weights },
    ));
    Ok(())
}

pub fn causal_node_ids(&self) -> Vec<String> {
    self.causal_graph
        .read()
        .map(|g| g.nodes.keys().cloned().collect())
        .unwrap_or_default()
}
```

**No new tests needed** — these are thin wrappers. Tested indirectly via P2-3 and P3-3 tests.

---

### Task P2-3: Implement `apply_delta(CognitiveDelta::Intervene)`

**File:** `src/cognitive_state.rs`  
**Location:** Lines ~628–631 (replace stub)

```rust
CognitiveDelta::Intervene { var, value } => {
    self.world
        .write()
        .map_err(|_| CognitiveError::LockError)?
        .apply_intervention(&var, value)
        .map_err(CognitiveError::StoreError)?;
    // world lock drops here — memory lock acquired separately below
    let mut rec = MemoryRecord::new(
        MemoryType::Reflexion,
        actor.to_string(),
        "causal_intervene".to_string(),
        format!("do({}={})", var, value),
        serde_json::json!({"var": var, "value": value}),
    );
    let id = rec.id;
    self.memory
        .lock()
        .map_err(|_| CognitiveError::LockError)?
        .add(rec)
        .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
    Ok(vec![id])
}
```

**Verification test (`tests/unit/cognitive_state_tests.rs`):**
```rust
#[test]
fn test_apply_delta_intervene_pins_causal_graph() {
    let handle = make_test_handle(); // existing helper
    handle.world.write().unwrap()
        .add_causal_edge("x".into(), "y".into()).unwrap();

    handle.transact(
        CognitiveDelta::Intervene { var: "y".into(), value: 7.0 },
        "test-actor",
    ).unwrap();

    let pinned = handle.world.read().unwrap()
        .causal_graph.read().unwrap()
        .pinned_value("y");
    assert_eq!(pinned, Some(7.0));

    // Audit Reflexion record must exist
    let recs = handle.memory.lock().unwrap();
    let found = recs.all().iter().any(|r|
        r.record_type == MemoryType::Reflexion && r.action == "causal_intervene"
    );
    assert!(found, "Reflexion audit record must be written");
}
```

**AC-3 check:** `POST /v1/causal/intervene` now mutates the shared causal graph and writes audit trail. ✓

---

### Task P2-4: Make `DigitalTwin.step()` respect pinned interventions

**Context:** `fork_under_intervention("x", 99.0)` stores into `self.interventions` but `step()` never reads it. The RK4 output ignores pinned vars.

**Two-part change:**

#### Part A — Add `var_to_dim` field to `DigitalTwin`

**File:** `src/digital_twin.rs`

```rust
pub struct DigitalTwin<B: MemoryBackend + Send + Sync + 'static> {
    pub id: Uuid,
    pub fork: SimulationFork<B>,
    pub dynamics: ContinuousDynamics,
    pub sync_policy: SyncPolicy,
    pub created_at_tx: u64,
    trajectory: Vec<Vec<f64>>,
    t: f64,
    interventions: std::collections::HashMap<String, f64>,
    var_to_dim: std::collections::HashMap<String, usize>,  // NEW
}
```

Change `new()` signature:
```rust
pub fn new(
    fork: SimulationFork<B>,
    dynamics: ContinuousDynamics,
    sync_policy: SyncPolicy,
    created_at_tx: u64,
    var_to_dim: std::collections::HashMap<String, usize>,  // NEW
) -> Self {
    Self {
        id: Uuid::new_v4(),
        fork,
        dynamics,
        sync_policy,
        created_at_tx,
        trajectory: Vec::new(),
        t: 0.0,
        interventions: std::collections::HashMap::new(),
        var_to_dim,  // NEW
    }
}
```

In `step()`, after `let mut next = self.dynamics.step(...)`:
```rust
// Apply causal impulses: clamp pinned dimensions to intervention values.
for (var, &val) in &self.interventions {
    if let Some(&idx) = self.var_to_dim.get(var) {
        if idx < next.len() {
            next[idx] = val;
        }
    }
}
```

#### Part B — Build `var_to_dim` in `CognitiveHandle::fork_hybrid()`

**File:** `src/cognitive_state.rs`  
**Location:** `fork_hybrid()` method (~line 811–840)

After calling `wm.entity_mean_vectors()` (which already happens to seed dynamics), add:
```rust
let entity_vecs = wm.entity_mean_vectors();
let mut var_to_dim = std::collections::HashMap::new();
let mut offset = 0usize;
for (entity_id, vec) in &entity_vecs {
    if vec.len() == 1 {
        var_to_dim.insert(entity_id.clone(), offset);
    }
    offset += vec.len();
}
// Pass var_to_dim to DigitalTwin::new(...)
```

Update `DigitalTwin::new(...)` call to include `var_to_dim`.

**Caller that must also be updated:** Any direct construction of `DigitalTwin::new()` in tests. Grep: `DigitalTwin::new(` — pass `HashMap::new()` (no-op, correct for tests that don't use interventions).

**Verification test:**
```rust
#[test]
fn test_fork_under_intervention_clamps_rk4_output() {
    // Build handle with a 1-dim entity at position x=0
    let handle = make_handle_with_entity("e0", vec![0.0]);
    let (mut twin, _dyn) = handle.fork_hybrid(SyncPolicy::ReadOnly).unwrap();
    twin.fork_under_intervention("e0", 99.0);
    twin.step("any_action").unwrap();
    let traj = twin.trajectory();
    // Dimension 0 must be clamped to 99.0 after any RK4 step
    assert_eq!(traj.last().unwrap()[0], 99.0,
        "intervention must override RK4 output for pinned entity dimension");
}
```

**AC-3 check:** Continuous trajectory under intervention is distinct from unforked trajectory. ✓

---

### Task P2-5: Populate `causal_provenance` in `rollout_hybrid()`

**Context:** `SimulationFork` has no WME reference — causal node IDs must be passed in.  
`ExperienceRecord.causal_provenance: Option<Vec<(String, String)>>` exists but is never set.

**File:** `src/simulation_fork.rs`

Change `rollout_hybrid()` signature:
```rust
pub fn rollout_hybrid(
    &mut self,
    actions: Vec<String>,
    dt: f64,
    dynamics: Option<ContinuousDynamics>,
    causal_nodes: Option<Vec<String>>,   // NEW — pass wm.causal_node_ids() or None
) -> Result<HybridRolloutResult, CognitiveError>
```

Inside, when writing each step's Temporal record (wherever `MemoryRecord::new(...)` is called for the step), extend metadata:
```rust
let base_meta = serde_json::json!({"step": step_idx, "action": action});
let meta = if let Some(ref nodes) = causal_nodes {
    let prov: Vec<_> = nodes.iter()
        .map(|n| serde_json::json!({"node": n, "equation": "LinearSE"}))
        .collect();
    let mut m = base_meta.as_object().cloned().unwrap_or_default();
    m.insert("causal_provenance".to_string(), serde_json::json!(prov));
    serde_json::Value::Object(m)
} else {
    base_meta
};
```

**Callers to update:**

| Caller | File | Change |
|--------|------|--------|
| `DigitalTwin::rollout()` | `digital_twin.rs:83` | Pass `None` |
| Any test calling `rollout_hybrid` directly | test files | Pass `None` |
| `CognitiveHandle::fork_hybrid()` (if it calls rollout_hybrid) | `cognitive_state.rs` | Pass `Some(wm.causal_node_ids())` |

Note: `fork_hybrid()` returns `(SimulationFork, ContinuousDynamics)` — it does not call `rollout_hybrid` itself. The provenance injection happens when the caller calls `twin.rollout(actions)` → `fork.rollout_hybrid(actions, 1.0, Some(dyn), None)`. To get provenance, callers that want it must pass `Some(node_ids)` directly.

**Simplest approach**: expose `causal_node_ids()` via `CognitiveHandle` as a public helper so callers can pass it in:
```rust
// cognitive_state.rs
pub fn causal_node_ids(&self) -> Vec<String> {
    self.world.read()
        .map(|w| w.causal_node_ids())
        .unwrap_or_default()
}
```

**Verification test:**
```rust
#[test]
fn test_rollout_hybrid_writes_causal_provenance_when_nodes_passed() {
    let mut fork = make_test_fork();
    let result = fork.rollout_hybrid(
        vec!["a".into(), "b".into()],
        0.1,
        None,
        Some(vec!["x".into(), "y".into()]),
    ).unwrap();
    // Inspect records in isolated store
    let recs = fork.all_records();
    let with_prov = recs.iter().filter(|r|
        r.metadata.get("causal_provenance").map(|v| !v.is_null()).unwrap_or(false)
    ).count();
    assert!(with_prov >= 1, "at least one record must carry causal_provenance");
}
```

**AC-3 check:** ExperienceStore records now carry `(node_id, equation_tag)` pairs. ✓

---

## Phase 3 — AC-4, AC-5, AC-6: Surface & Invariance

### AC-4: All SCM operators through transact gate + MCP + SDK

**Current state:** REST routes `/v1/causal/*` exist and parse correctly. MCP tools `causal_intervene / counterfactual / credit_assign / rewrite_equation` exist and call those routes. Python SDK handlers exist. All `apply_delta` bodies are stubs returning `Ok(vec![])`.

**Gap:** Only the stub bodies. P2-3 already fills `Intervene`. Three remaining stubs below.

---

### Task P3-1: Implement `apply_delta(CognitiveDelta::Counterfactual)`

**File:** `src/cognitive_state.rs`  
**Location:** Lines ~632–635 (replace stub)

```rust
CognitiveDelta::Counterfactual { actual_state, intervention_var, intervention_value } => {
    let outcome = self.world
        .read()
        .map_err(|_| CognitiveError::LockError)?
        .counterfactual(actual_state, intervention_var.clone(), intervention_value)
        .map_err(CognitiveError::StoreError)?;
    // world lock drops here
    let mut rec = MemoryRecord::new(
        MemoryType::Reflexion,
        actor.to_string(),
        "counterfactual".to_string(),
        format!("cf({}={})", intervention_var, intervention_value),
        serde_json::json!({"counterfactual_outcome": outcome,
                           "intervention_var": intervention_var,
                           "intervention_value": intervention_value}),
    );
    let id = rec.id;
    self.memory
        .lock()
        .map_err(|_| CognitiveError::LockError)?
        .add(rec)
        .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
    Ok(vec![id])
}
```

**Verification test:**
```rust
#[test]
fn test_apply_delta_counterfactual_writes_outcome_record() {
    let handle = make_test_handle_with_scm(); // x→y, LinearSE weight=1
    handle.transact(CognitiveDelta::Counterfactual {
        actual_state: HashMap::from([("x".to_string(), 1.0), ("y".to_string(), 2.5)]),
        intervention_var: "y".to_string(),
        intervention_value: 0.0,
    }, "actor").unwrap();

    let mem = handle.memory.lock().unwrap();
    let rec = mem.all().iter()
        .find(|r| r.action == "counterfactual")
        .expect("Reflexion record must exist");
    let outcome = rec.metadata["counterfactual_outcome"].as_object().unwrap();
    assert!(outcome.contains_key("y"), "counterfactual_outcome must contain intervened node");
}
```

**AC-4 check:** `POST /v1/causal/counterfactual` now returns persisted outcome. ✓

---

### Task P3-2: Implement `apply_delta(CognitiveDelta::CreditAssign)`

**File:** `src/cognitive_state.rs`  
**Location:** Lines ~636–639 (replace stub)

**Lock discipline:** Memory lock acquired → released → world lock acquired → released → memory lock acquired again. Never held simultaneously.

```rust
CognitiveDelta::CreditAssign(signal) => {
    // Step 1: build trajectory from recent Temporal records (memory lock, then drop)
    let traj: Vec<std::collections::HashMap<String, f64>> = {
        let mem = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
        mem.all()
            .iter()
            .filter(|r| r.record_type == MemoryType::Temporal && r.actor == actor)
            .rev()
            .take(50)
            .map(|r| {
                r.metadata
                    .as_object()
                    .map(|m| {
                        m.iter()
                            .filter_map(|(k, v)| v.as_f64().map(|f| (k.clone(), f)))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect()
        // mem lock drops here
    };

    // Step 2: run AAP attribution (world read lock, then drop)
    let report = self.world
        .read()
        .map_err(|_| CognitiveError::LockError)?
        .credit_assign_trajectory(&traj, signal)
        .map_err(CognitiveError::StoreError)?;
    // world lock drops here

    // Step 3: write AttributionReport as Reflexion record (memory lock again)
    let mut rec = MemoryRecord::new(
        MemoryType::Reflexion,
        actor.to_string(),
        "credit_assign".to_string(),
        report.broken_equation.clone().unwrap_or_else(|| "none".to_string()),
        serde_json::json!({
            "broken_equation": report.broken_equation,
            "confidence": report.confidence,
            "single_intervention_sufficient": report.single_intervention_sufficient,
            "counterfactual_outcome": report.counterfactual_outcome,
        }),
    );
    let id = rec.id;
    self.memory
        .lock()
        .map_err(|_| CognitiveError::LockError)?
        .add(rec)
        .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
    Ok(vec![id])
}
```

**Verification test:**
```rust
#[test]
fn test_apply_delta_credit_assign_writes_attribution_record() {
    let handle = make_test_handle_with_scm();
    // Add some Temporal records so trajectory is non-empty
    handle.transact(CognitiveDelta::AddMemory(make_temporal_record("actor")), "actor").unwrap();

    handle.transact(
        CognitiveDelta::CreditAssign(FailureSignal::MaxIterations),
        "actor",
    ).unwrap();

    let mem = handle.memory.lock().unwrap();
    let rec = mem.all().iter()
        .find(|r| r.action == "credit_assign")
        .expect("attribution Reflexion record must exist");
    assert!(rec.metadata["confidence"].as_f64().unwrap() >= 0.0);
}
```

**AC-4 check:** `POST /v1/causal/credit_assign` now runs full AAP and persists result. ✓

---

### Task P3-3: Implement `apply_delta(CognitiveDelta::RewriteStructuralEquation)`

**File:** `src/cognitive_state.rs`  
**Location:** Lines ~640–643 (replace stub)

```rust
CognitiveDelta::RewriteStructuralEquation { node_id, new_weights } => {
    self.world
        .write()
        .map_err(|_| CognitiveError::LockError)?
        .rewrite_structural_equation(&node_id, new_weights.clone())
        .map_err(CognitiveError::StoreError)?;
    // world lock drops here
    let mut rec = MemoryRecord::new(
        MemoryType::Reflexion,
        actor.to_string(),
        "rewrite_equation".to_string(),
        node_id.clone(),
        serde_json::json!({"node_id": node_id, "new_weights": new_weights}),
    );
    let id = rec.id;
    self.memory
        .lock()
        .map_err(|_| CognitiveError::LockError)?
        .add(rec)
        .map_err(|e| CognitiveError::StoreError(e.to_string()))?;
    Ok(vec![id])
}
```

**Verification test:**
```rust
#[test]
fn test_apply_delta_rewrite_equation_changes_structural_equation() {
    let handle = make_test_handle_with_scm(); // has node "y" with LinearSE weight=[1.0]
    handle.transact(CognitiveDelta::RewriteStructuralEquation {
        node_id: "y".to_string(),
        new_weights: vec![2.0],
    }, "actor").unwrap();

    let world = handle.world.read().unwrap();
    let g = world.causal_graph.read().unwrap();
    let node = g.nodes.get("y").expect("node must exist");
    let eq = node.equation.as_ref().expect("equation must be set");
    assert_eq!(eq.evaluate(&[1.0], 0.0), 2.0,
        "new equation f(x)=2x must evaluate correctly");
}
```

**AC-4 check:** `POST /v1/causal/rewrite_equation` now permanently rewrites the structural equation. ✓

---

### Task P3-4: OOD Invariance Test (AC-5)

**AC-5 text:**  
> When environment dynamics change, the system isolates and rewires only the perturbed structural equations while preserving the rest of the topological world model.

**Test strategy:** 5 SCM environments sharing structure `x→y→z`. Environments 1–4: `z.noise_var = 0.1`. Environment 5 (OOD): `z.noise_var = 2.0`. Run `credit_assign` on a 50-step trajectory from env 5. Assert `broken_equation == Some("z")` — the perturbed node — with confidence ≥ 0.5.

**File:** `tests/unit/scm_foundations_tests.rs`

```rust
#[test]
fn test_ood_invariance_credit_assign_isolates_perturbed_node() {
    use std::collections::HashMap;
    use std::sync::Arc;

    let build_env = |z_noise: f64| {
        let mut g = CausalGraph::new();
        g.add_node("x".into()).unwrap();
        g.add_node("y".into()).unwrap();
        g.add_node("z".into()).unwrap();
        g.add_edge("x".into(), "y".into()).unwrap();
        g.add_edge("y".into(), "z".into()).unwrap();
        if let Some(n) = g.node_mut("y") {
            n.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
            n.noise_var = 0.1;
        }
        if let Some(n) = g.node_mut("z") {
            n.equation = Some(Arc::new(LinearSE { weights: vec![1.0] }));
            n.noise_var = z_noise;
        }
        g
    };

    // Envs 1-4: stable (z_noise = 0.1). Env 5: perturbed (z_noise = 2.0).
    // Claim: credit_assign on env-5 trajectory identifies "z" as the broken equation.
    let ood_graph = build_env(2.0);

    // Generate 50-step trajectory under env-5 dynamics:
    // x increments linearly; y ≈ x; z deviates from y by large sinusoidal noise
    let traj: Vec<HashMap<String, f64>> = (0..50)
        .map(|i| {
            let x = i as f64 * 0.1;
            let y = x + 0.05;
            let z = y + 2.5 * (i as f64 * 0.3).sin(); // large noise for OOD env
            HashMap::from([
                ("x".to_string(), x),
                ("y".to_string(), y),
                ("z".to_string(), z),
            ])
        })
        .collect();

    let report = ood_graph
        .credit_assign(&traj, &FailureSignal::ExplicitFail("ood".to_string()))
        .expect("credit_assign must succeed");

    // AC-5: must isolate the perturbed node
    assert_eq!(
        report.broken_equation.as_deref(), Some("z"),
        "OOD invariance: credit_assign must isolate 'z' as the perturbed structural equation"
    );
    assert!(
        report.confidence >= 0.5,
        "confidence must be meaningful: got {}", report.confidence
    );
    assert!(
        !report.counterfactual_outcome.is_empty(),
        "Prediction step must have run (AC-2 dependency)"
    );
    // Non-perturbed nodes must NOT be identified as broken
    assert_ne!(
        report.broken_equation.as_deref(), Some("x"),
        "stable node 'x' must not be blamed"
    );
    assert_ne!(
        report.broken_equation.as_deref(), Some("y"),
        "stable node 'y' must not be blamed"
    );
}
```

**AC-5 check:** System isolates perturbed `z`, preserves `x` and `y`. ✓

---

### Task P3-5: Non-Regression Run (AC-6)

**Commands to run after all above tasks complete:**

```sh
cargo build --no-default-features --features "petgraph_backend"
cargo test --no-default-features --features "petgraph_backend" --lib
cargo test --no-default-features --features "petgraph_backend" --test unit_suite
cargo test --no-default-features --features "petgraph_backend" --test integration_suite
```

**Acceptance:** Exit code 0 on all four. Zero new failures introduced.

**Write latency check (AC-6 p50 < 1ms):** Add to existing benchmark or check with:
```sh
cargo bench --no-default-features --features "petgraph_backend" -- temporal_indexer_bench
```

---

## Execution Order (dependency-safe)

```
P2-1  causal.rs          apply_intervention() mutating method         build → unit test
P2-2  mod.rs             3 WME wrappers                               build (no new test)
P2-3  cognitive_state.rs Intervene body                               build → unit test
P2-4a digital_twin.rs    var_to_dim field + new() sig + step() clamp  build
P2-4b cognitive_state.rs fork_hybrid() builds var_to_dim              build → unit test
P2-5  simulation_fork.rs rollout_hybrid() causal_nodes param          build → unit test
P3-1  cognitive_state.rs Counterfactual body                          build → unit test
P3-2  cognitive_state.rs CreditAssign body                            build → unit test
P3-3  cognitive_state.rs RewriteStructuralEquation body               build → unit test
P3-4  scm_tests.rs       OOD invariance test                          unit test passes
P3-5  all suites         cargo test (non-regression)                  all green
```

**Breaking change surface:** Only `rollout_hybrid()` signature and `DigitalTwin::new()` signature.  
Both are crate-internal. Compiler enforces all callsites — no silent breakage possible.

---

## ReAct Review Against ACs

| AC | Gap addressed by | Falsifiable check |
|----|-----------------|-------------------|
| AC-3 continuous primary | P2-3, P2-4, P2-5 | `fork_under_intervention("e", v).step()` → trajectory[dim] == v |
| AC-3 ExperienceStore provenance | P2-5 | step records have `causal_provenance` key in metadata |
| AC-4 transact gate SCM operators | P2-3, P3-1, P3-2, P3-3 | All 4 apply_delta arms return Ok(vec![id]) with Reflexion record |
| AC-4 MCP + SDK | no code change needed | Routes already exist; stubs → real bodies via above tasks |
| AC-5 OOD invariance | P3-4 | `credit_assign` on env-5 traj returns broken_equation=Some("z") |
| AC-6 non-regression | P3-5 | 0 test failures; p50 write < 1ms |

**Inversion checks (what must not be true after implementation):**

| Inversion | Verified by |
|-----------|-------------|
| Intervention applied but causal graph unchanged | P2-3 test asserts pinned_value("y") == Some(7.0) |
| RK4 output ignores pinned vars | P2-4 test asserts trajectory[0] == 99.0 |
| Counterfactual returns empty outcome | P3-1 test asserts counterfactual_outcome contains intervened node |
| CreditAssign writes no record | P3-2 test asserts Reflexion record with confidence ≥ 0.0 exists |
| Stable node blamed in OOD scenario | P3-4 asserts broken_equation != "x" and != "y" |
| Any existing test fails | P3-5 cargo test exits 0 |

**Total: 10 tasks. ~280 lines new code. 8 new tests. 2 signature changes (compiler-enforced).**

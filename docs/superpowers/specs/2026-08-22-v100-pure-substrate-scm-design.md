# HipCortex v1.0 — Pure Cognitive State Substrate + Causal SCM Design

**Date:** 2026-08-22  
**Status:** Approved for implementation  
**Governs:** Phases 0–4 (Workstreams A–F reconciled with roadmap ACs)  
**Acceptance authority:** `HipCortex_Development_Gaps_Context_Roadmap.md` — AC-1 through AC-6  

---

## 0. Governing Model

Tripartite architecture (outer → inner):

```
Causal Reasoning Layer (external orchestrators, KARM)
    ↓ uses
Kakeya Abstraction Action Model (future, deferred)
    ↓ sits on
HipCortex Cognitive State Substrate  ← this spec
```

HipCortex owns: memory, continuous dynamics (M), epistemic beliefs, causal SCM, health metrics.  
HipCortex never: goal-seeks, plans, or orchestrates. External agents do that.  
Exception (Option C): `ReactEngine` stays in Phase 0–3 but becomes SCM-aware; extracted behind `pure-substrate` flag in Phase 4.

---

## 1. AC Status Baseline

| AC | Meaning | Status | Blocker |
|----|---------|--------|---------|
| AC-1 | SCM structural completeness | 🟡 Partial | No `StructuralEquation` trait; `u_v` is linear residual only |
| AC-2 | Counterfactual credit assignment | 🔴 Red | `loop_engine.rs:638` blind `GoalStatus::Failed` |
| AC-3 | Continuous substrate integration | 🟡 Partial | RK4 residual; DigitalTwin not forkable under `do` |
| AC-4 | Surface exposure of SCM operators | 🔴 Red | `Intervene/Counterfactual/CreditAssign/RewriteEquation` absent from CognitiveDelta |
| AC-5 | Invariance under distribution shift | 🔴 Red | No local equation rewiring |
| AC-6 | Non-regression | 🟢 Green | 82/82 E2E, latency p50 < 1 ms |

---

## 2. ReAct Operating Protocol (for the implementing agent)

```
Observe: Which AC is currently red?
Think:   Which structural equation is missing or under-specified in the code?
Act:     Write failing test first (TDD). Implement minimal fix. (Karpathy: surgical only.)
Reflect: Inversion test — "Can the system still fail in the old way?"
         YES → revisit Act. Never advance.
         NO  → commit. Advance to next AC.
```

Never advance a phase until the Reflexion checkpoint of the previous phase is green.  
Never touch code outside the targeted module for a given task.

---

## 3. Phase Architecture

### Phase 0 — SCM Foundations (1–2 weeks) → AC-1

**Goal:** Formalize `StructuralEquation` trait + graph surgery. Backward-compatible.

**P0.1** — Add `StructuralEquation` trait in `causal.rs`:
```rust
pub trait StructuralEquation: Send + Sync {
    fn evaluate(&self, parents: &[f64], u: f64) -> f64;
    fn invert_for_u(&self, parents: &[f64], observed: f64) -> f64;
}

pub struct LinearSE { pub weights: Vec<f64> }
impl StructuralEquation for LinearSE {
    fn evaluate(&self, parents: &[f64], u: f64) -> f64 {
        self.weights.iter().zip(parents).map(|(w, p)| w * p).sum::<f64>() + u
    }
    fn invert_for_u(&self, parents: &[f64], observed: f64) -> f64 {
        observed - self.weights.iter().zip(parents).map(|(w, p)| w * p).sum::<f64>()
    }
}
```

**P0.2** — Extend `CausalNode` (additive only):
```rust
pub struct CausalNode {
    pub id: String,
    pub properties: HashMap<String, String>,
    pub embedding: Option<[f32; 128]>,
    // NEW
    pub equation: Option<Arc<dyn StructuralEquation>>,
    pub noise_var: f64,   // variance of U_i; 0.0 = deterministic
}
```

**P0.3** — Add `CausalGraph::do_operator(var: &str, value: f64) -> CausalGraph`:
- Returns new graph (no mutation of original).
- Removes all edges pointing INTO `var`.
- Pins `var` value in a `pinned: HashMap<String, f64>` field.
- Downstream nodes recompute via topological forward-pass.

**P0.4** — Refactor `compute_scm_counterfactual` to use `equation.evaluate()` when `equation.is_some()`, fall back to existing linear-weight path. Zero API change.

**P0.5** — Unit tests (new file `tests/unit/scm_foundations_tests.rs`):
- Linear SCM: `invert_for_u(evaluate(pa, u)) ≈ u` roundtrip.
- Graph surgery: after `do(X=x)`, X has no parents, value = x.
- Counterfactual consistency: same U, different intervention → different outcome.
- Non-linear SCM (quadratic `LinearSE` extension): attribution still correct.

**Reflexion checkpoint P0:** Can we hold U fixed and get a different outcome under intervention?  
→ Prove with `test_counterfactual_u_fixed`.

---

### Phase 1 — Credit Assignment in ReactEngine (2–3 weeks) → AC-2

**Goal:** Wire `credit_assign` into `loop_engine.rs:638` failure path. MAT caches results.

**P1.1** — Add types in `causal.rs`:
```rust
pub enum FailureSignal { MaxIterations, CoherenceViolation, ExplicitFail(String) }

pub struct AttributionReport {
    pub broken_equation: Option<String>,   // node_id of f_i that caused failure
    pub confidence: f64,                   // 0.0–1.0
    pub counterfactual_outcome: HashMap<String, f64>,
    pub single_intervention_sufficient: bool,
}
```

**P1.2** — Implement `CausalGraph::credit_assign(trajectory: &[HashMap<String,f64>], signal: FailureSignal) -> Result<AttributionReport, String>`:
1. Abduction: recover `U` from factual trajectory via `invert_for_u`.
2. For each candidate node: apply `do_operator`, re-forward with same `U`.
3. Score: how much does single intervention restore success_factors?
4. Return highest-scoring node if confidence ≥ threshold.

**P1.3** — `AttributionCache` (MAT) in `src/mat.rs`:
```rust
pub struct AttributionCache {
    entries: HashMap<ConflictSignature, AttributionReport>,
}
pub struct ConflictSignature { hash: u64 }  // hash of (goal_type, unsatisfied_factors, failure_pattern)
```
Exposed: `GET /v1/mat` (list recent attributions) + MCP resource `hipcortex://mat/recent`.  
Never executes tie-breaker itself — passive read-only surface.

**P1.4** — Wire into `loop_engine.rs::ReactEngine::run` at line 638.  
Pre-requisite: add `pub fn credit_assign_trajectory(...)` method on `WorldModelEnhanced` that acquires the `causal_graph` read lock and delegates to `CausalGraph::credit_assign`. Do NOT reach into the private field directly.

```rust
// Before GoalStatus::Failed — attempt counterfactual attribution
let traj = collect_trajectory_states(store, goal_id, i);
if let Ok(report) = self.wm.credit_assign_trajectory(&traj, FailureSignal::MaxIterations) {
    store_attribution_reflexion(store, goal_id, &report)?;
    self.mat.insert(ConflictSignature::from_goal(&goal_payload), report.clone());
    if report.confidence >= 0.85 && report.single_intervention_sufficient {
        return self.targeted_retry(store, goal_id, &goal_payload, &report);
    }
}
goal_payload.status = GoalStatus::Failed;
```

**P1.5** — `targeted_retry`: ONE additional iteration with the broken equation rewritten. Never recurse.

**P1.6** — Attribution stored as `MemoryType::Reflexion`, `derived_from = goal_id`, tagged `attribution: true`.

**P1.7** — Synthetic failure suite `tests/integration/credit_assign_sit.rs`:
- 10-step, 50-step, 100-step trajectories with known broken equations.
- Measure: single-shot attribution accuracy. Target: ≥ 85%.

**Reflexion checkpoint P1:** Does the loop still blind-retry when attribution is possible?  
→ Fail test: `test_no_blind_retry_when_attribution_available`.

---

### Phase 2 — Continuous Substrate Integration (2 weeks) → AC-3

**Goal:** DigitalTwin forkable under `do`; ExperienceStore records causal provenance.

**P2.1** — `DigitalTwin::fork_under_intervention(var: &str, value: f64) -> DigitalTwin`:
- Calls `causal_graph.do_operator(var, value)` → mutilated graph.
- New DigitalTwin uses mutilated graph for its vector field perturbations.
- Original twin unaffected (clone, not borrow).

**P2.2** — `HybridRollout` accepts `mutilated_graph: Option<&CausalGraph>`:
- When present, causal impulses respect the mutilated structure.
- Discrete causal events = instantaneous modifications to the continuous vector field.

**P2.3** — `ExperienceStore` raw record extended with `causal_provenance: Option<Vec<(NodeId, EquationTag)>>`:
- Records which structural equations were active during each trajectory segment.
- Backward-compatible: `None` for old records.

**P2.4** — Integration test: fork twin under `do(decision="A")`, rollout 10 steps, verify divergence from factual twin > 0 when equation active.

**Reflexion checkpoint P2:** Can continuous dynamics primary path be activated without discrete memory records as the sole driver?

---

### Phase 3 — Surface Exposure + Invariance (1–2 weeks) → AC-4, AC-5

**Goal:** SCM operators on all surfaces. Local equation rewiring under distribution shift.

**P3.1–P3.4** — Add 4 `CognitiveDeltaOp` variants in `cognitive_state.rs`:
```rust
Intervene { var: String, value: f64 },
Counterfactual { actual_state: HashMap<String,f64>, intervention_var: String, intervention_value: f64 },
CreditAssign { trajectory_id: Uuid, failure_signal: FailureSignal },
RewriteStructuralEquation { node_id: String, new_weights: Vec<f64> },
```

**P3.5** — Wire all 4 through `POST /v1/cognitive/transact` gate (existing atomicity + SafetyGuardrail preserved).

**P3.6** — REST aliases: `POST /v1/causal/intervene`, `/v1/causal/counterfactual`, `/v1/causal/credit-assign`, `/v1/causal/rewrite-equation`.

**P3.7** — 4 new MCP tools: `intervene`, `counterfactual`, `credit_assign`, `rewrite_equation`. Total: 46 tools.

**P3.8** — Python SDK: `client.intervene(var, value)`, `client.counterfactual(actual, var, value)`, `client.credit_assign(traj_id)`, `client.rewrite_equation(node_id, weights)`.

**P3.9** — OOD benchmark `tests/integration/ood_invariance_sit.rs`:
- Perturb 2/10 structural equations deliberately.
- Verify: only those 2 rewired; remaining 8 + topology preserved.
- OOD success rate drop ≤ 5% (vs ≥ 40% for pure-associational baseline).

**Reflexion checkpoint P3:** Can an external orchestrator perform a causal intervention without touching anything inside `ReactEngine`?

---

### Phase 4 — Hardening (2 weeks) → AC-6 + Workstreams C, D, B

**Type-2 SDT (Workstream C):**

**P4.1** — Extend `CalibrationTracker` with:
```rust
pub meta_d_prime: f64,    // Fleming & Dolan (2012)
pub d_prime: f64,
pub m_ratio: f64,         // meta_d_prime / d_prime
pub c2_star: f64,         // optimal Type-2 criterion
pub withdraw_delta: f64,  // rate of withdrawing incorrect vs correct
```

**P4.2** — `MMBPhenotype` classifier (k-means on `(withdraw_delta, m_ratio)`):
```rust
pub enum MMBPhenotype { BlanketConfidence, BlanketWithdrawal, SelectiveSensitivity }
```

**P4.3** — Health gate on `credit_assign`: return `Err("calibration_gate")` when `phenotype != SelectiveSensitivity || calibration_score < 0.70 || m_ratio < 0.5`.

**MGV (Workstream D):**

**P4.4** — `MGVOperator` in `src/mgv.rs`:
- **Monitor**: `FOK = jtms_justification_strength × calibration_score × historical_success_rate`
- **Generate**: trigger `simulate_rollout` (existing HybridRollout)
- **Verify**: `JOL = empirical_cognitive_delta_outcome × jtms_consistency_score`
- On `|FOK - JOL| > 0.3`: `JTMS::retract_belief(belief_id)` + quarantine flag on record.

**P4.5** — MCP tool `mgv_check` + REST `POST /v1/mgv/check`.

**pure-substrate flag (Workstream B):**

**P4.6** — Feature flag in `Cargo.toml`:
```toml
[features]
pure-substrate = []
```

**P4.7** — In `loop_engine.rs`:
```rust
#[cfg(feature = "pure-substrate")]
compile_error!("ReactEngine disabled. Use external hipcortex-meta-orchestrator crate.");
```
Default = off for v0.10.x (backward compat). Default = on for v1.0.0.

**P4.8** — Deprecation notice when `ReactEngine::run` is called with `HIPCORTEX_PURE_SUBSTRATE=1` env var.

**Property tests (Workstream F):**

**P4.9** — `tests/property/scm_props.rs`:
- DAG acyclicity after every `do_operator`.
- `invert_for_u ∘ evaluate ≈ id` (roundtrip, tolerance 1e-6).
- Counterfactual consistency under same U.
- JTMS: no belief without valid justification after retraction.
- Attribution accuracy ≥ 85% on 50-step synthetic suite.
- Conservation of causal edges not involved in rewiring.

**P4.10** — Single-command acceptance gate:
```sh
cargo test --test acceptance_suite --no-default-features --features "petgraph_backend"
```
Produces machine-readable pass/fail per AC.

**P4.11** — Update: `docs/operators.md` (new, 6 canonical + 4 SCM operators), `docs/capabilities.md` (v1.0.0 stamp), `docs/roadmap.md`, whitepaper §3 + §7.

---

## 4. Testing & Validation Strategy

| Layer | Scope | Command |
|-------|-------|---------|
| Unit | Each trait, struct, operator | `cargo test --lib` |
| Property | Invariants (proptest, 1000 cases each) | `cargo test --test property_suite` |
| Integration | Phase gate suites (82 existing + ~30 new) | `cargo test --test integration_suite` |
| Acceptance | All 6 ACs, machine-readable | `cargo test --test acceptance_suite` |
| Regression | Alice/Bob/Carol longitudinal + latency benchmark | `cargo bench` + existing E2E |

**Non-negotiable regression bar:** 82/82 E2E pass, write p50 < 1ms, zero JTMS integrity violations.

---

## 5. Migration & Deprecation

| Version | Change | Impact |
|---------|--------|--------|
| v0.10.x | Additive: `StructuralEquation`, `do_operator`, SCM ops on all surfaces | Zero breaking changes |
| v0.10.x | `ReactEngine` stays; `credit_assign` wired into failure path | Transparent improvement |
| v0.11.x | `pure-substrate` flag available (opt-in) | External orchestrators can switch |
| v1.0.0 | `pure-substrate` default = true; deprecation warning on internal `ReactEngine` use | One-version window |
| v1.1.0 | `ReactEngine` moved to `hipcortex-meta-orchestrator` crate | Migration guide in CHANGELOG |

`compute_scm_counterfactual` remains public, now internally calls `equation.evaluate()` — transparent upgrade.  
All 4 new `CognitiveDeltaOp` variants are `#[non_exhaustive]`-protected — existing match arms unaffected.

---

## 6. Atomic Task Order for Claude Code

Execute strictly in this sequence. Each item = one PR-sized unit. Write failing test first.

```
P0.1 → P0.2 → P0.3 → P0.4 → P0.5 → [Reflexion P0] →
P1.1 → P1.2 → P1.3 → P1.4 → P1.5 → P1.6 → P1.7 → [Reflexion P1] →
P2.1 → P2.2 → P2.3 → P2.4 → [Reflexion P2] →
P3.1–P3.4 → P3.5 → P3.6 → P3.7 → P3.8 → P3.9 → [Reflexion P3] →
P4.1 → P4.2 → P4.3 → P4.4 → P4.5 → P4.6 → P4.7 → P4.8 → P4.9 → P4.10 → P4.11
```

Never skip a Reflexion checkpoint. Never touch code outside the targeted file for a given task.

---

## 7. What Is Explicitly Out of Scope

Per roadmap §6 — deferred to KARM handover:
- Kakeya geometric minimal-measure action sweeping.
- Distillation of planning into lightweight continuous operator.
- Edge/CPU-only numeric solver.
- Outcome-based pricing instrumentation.
- Directional `StructuralEquation` (vector parents) — interface designed for it (P0.1 uses `&[f64]`), implementation deferred.

---

**End of spec.**  
Success = all 6 ACs green + single `cargo test --test acceptance_suite` produces PASS.

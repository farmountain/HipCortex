# HipCortex Development Handover
## Gaps · Context · Roadmap (Causal SCM Completion Focus)
**Target Agent:** Claude Code  
**Date:** 2026-08-21  
**Repo:** https://github.com/farmountain/HipCortex (current HEAD ≈ v0.9.x Continuous Substrate)  
**Governing Vision:** Tripartite Architecture = Causal Reasoning Layer → Kakeya Abstraction Action Model → HipCortex Cognitive State Substrate  

---

## 0. First-Principles Premises (Non-Negotiable)

1. **State is a continuous topological manifold \(\mathcal{M}\), not a token buffer.**  
   Discrete KV-caches produce attention drift, catastrophic forgetting and degenerative attractors.  
   Formal requirement: \(\dot{s} = f(s,a,u)\) evolves on a differentiable manifold with attractor stability guarantees (Lyapunov / contraction mapping).

2. **Causation is not correlation.**  
   Pearl’s ladder:  
   - Tier 1 Association \(P(y|x)\)  
   - Tier 2 Intervention \(P(y|\mathrm{do}(x))\)  
   - Tier 3 Counterfactual \(P(y_x|x',y')\)  
   Only Tier 2+3 yield OOD invariance and single-shot credit assignment.

3. **Memory must be causal and provenance-preserving.**  
   Every state transition is a structural equation \(X_i := f_i(\mathrm{PA}_i, U_i)\).  
   Counterfactual credit assignment requires the ability to hold background noise \(U\) fixed while intervening on a single decision variable.

4. **Inversion test (what must not be true):**  
   - Context length grows unboundedly with horizon.  
   - Failure recovery requires combinatorial re-prompting.  
   - Causal structure is only post-hoc linguistic rationalisation.  
   - Continuous dynamics exist but are never the primary substrate of planning or credit assignment.

---

## 1. Current State Snapshot (Accurate as of repo inspection)

### 1.1 Strengths already present
| Component | Location | Maturity | Notes |
|-----------|----------|----------|-------|
| Continuous Substrate | `cognitive_state.rs`, `digital_twin.rs`, `continuous_dynamics.rs` | High | RK4 integrator, HybridRollout, ExperienceStore 3-tier pyramid, transactional `CognitiveDelta` |
| Causal Graph core | `src/modules/world_model_enhanced/causal.rs` | Medium-High | DAG, path queries, backdoor adjustment, `compute_intervention`, `compute_scm_counterfactual` (Abduction-Action-Prediction) |
| World-Model Enhanced | `world_model_enhanced/` | Medium | TransitionModel, entity tracking, uncertainty, predictor, simulator |
| Coherence / Self-Model | `modules/coherence/`, `modules/self_model/` | High | Invariants, health, decision gating |
| Surfaces | REST, MCP (42 tools), Python/TS SDK, VS Code | High | Transactional cognitive API exists |

### 1.2 Critical Gaps (HipCortex-internal)

**G1 – Causal SCM is not the executive layer**  
Causal graph lives inside World-Model Enhanced. It is not the top-level operator that intervenes on action selection and state evolution.  
Acceptance failure mode: interventions remain observational statistics rather than structural `do`-operators that rewrite the execution graph.

**G2 – Counterfactual credit assignment is not the default failure-recovery path**  
`compute_scm_counterfactual` exists but is not wired into the ReAct / loop_engine / procedural backtracking path as the *single-shot* attribution mechanism that isolates the exact broken structural equation \(f_i\).

**G3 – Continuous dynamics are residual, not primary**  
RK4 residual sits on top of discrete steps. The primary state update is still discrete memory records.  
Goal: every long-horizon transition is a continuous flow on \(\mathcal{M}\) with discrete causal events as impulses.

**G4 – Missing explicit Structural Causal Model (SCM) object**  
Current CausalGraph stores nodes/edges/distributions. It lacks:
- Explicit structural equations \(f_i\) (callable, differentiable or at least evaluable).
- Independent noise terms \(U_i\) that can be held fixed under counterfactuals.
- Graph surgery operators that produce a mutilated model under `do(X=x)`.

**G5 – No formal causal invariance under distribution shift**  
When environment dynamics change, the system cannot yet isolate and rewire *only* the perturbed structural equations while preserving the rest of the topological world model.

**G6 – Transactional surface does not yet expose full SCM operators**  
`POST /v1/cognitive/transact` supports AddMemory, AdvanceGoal, UpdateBelief, etc., but not:
- `Intervene(var, value)`
- `Counterfactual(actual_state, intervention)`
- `RewriteStructuralEquation(i, new_f)`
- `CreditAssign(trajectory, failure_point)`

---

## 2. Goals-Driven Acceptance Criteria (Falsifiable)

### Primary Goal
Elevate the existing CausalGraph into a first-class, executable Structural Causal Model that sits above the continuous substrate and can be the source of truth for interventions and credit assignment.

### Acceptance Criteria (must all pass)

**AC-1 Structural Completeness**  
- Every node in the causal graph has an associated structural equation \(X_i := f_i(\mathrm{PA}_i, U_i)\).  
- \(U_i\) is an independent exogenous noise variable that can be sampled or held fixed.  
- Graph surgery (`do(X=x)`) produces a new valid SCM with the intervened node’s parents removed and value fixed.

**AC-2 Counterfactual Credit Assignment**  
Given a failed trajectory \(\tau = (s_0,a_0,s_1,\dots,s_T)\) and a failure signal at step \(k\):  
1. Abduction recovers the noise realisation \(U\) consistent with the factual path.  
2. Action intervenes on any decision variable at or before \(k\).  
3. Prediction yields the counterfactual outcome under the same \(U\).  
4. The system returns the *single* structural equation whose change restores success (or reports that no single intervention suffices).  
Measured by: single-shot attribution accuracy ≥ 85 % on a synthetic 50-step failure suite.

**AC-3 Continuous Substrate Integration**  
- State evolution on \(\mathcal{M}\) is the primary dynamics; discrete causal events are impulses that modify the vector field.  
- DigitalTwin + HybridRollout can be forked under an intervention and rolled out continuously.  
- ExperienceStore stores both continuous trajectories and the causal provenance that generated them.

**AC-4 Transactional & Surface Exposure**  
- All SCM operators are available through the single `CognitiveDelta` / transact gate.  
- MCP tools and Python SDK expose `intervene`, `counterfactual`, `credit_assign`, `rewrite_equation`.  
- REST OpenAPI is updated and contract tests pass.

**AC-5 Invariance under Shift**  
- When a subset of structural equations is deliberately perturbed, only those equations are re-estimated; the remainder of the causal graph and the topological manifold remain intact.  
- Measured by: OOD success rate drop ≤ 5 % after local rewiring vs. ≥ 40 % for a pure associational baseline.

**AC-6 Non-regression**  
- Existing continuous-substrate, memory, coherence, and surface tests continue to pass at 100 %.  
- Write latency p50 remains < 1 ms on the reference hardware.

---

## 3. Roadmap (Phased, Inversion-Aware)

### Phase 0 – Foundations (1–2 weeks)
- Formalise `StructuralEquation` trait / struct:  
  ```rust
  trait StructuralEquation {
      fn evaluate(&self, parents: &[f64], u: f64) -> f64;
      fn invert_for_u(&self, parents: &[f64], observed: f64) -> f64; // for abduction
  }
  ```
- Extend `CausalNode` with `equation: Option<Box<dyn StructuralEquation>>` and `noise_dist`.
- Implement graph surgery that returns a new `CausalGraph` under `do`.
- Unit tests for Abduction-Action-Prediction on linear and non-linear toy SCMs.
- **Reflexion checkpoint:** Can we hold \(U\) fixed and obtain different outcomes under intervention? If not, stop and fix.

### Phase 1 – Credit Assignment Loop (2–3 weeks)
- Implement `credit_assign(trajectory, failure_signal) -> AttributionReport`.
- Wire into `loop_engine.rs` / ReAct path: on failure, run counterfactual attribution before any re-prompt or tree expansion.
- Synthetic failure suite (10 / 50 / 100-step) with known ground-truth broken equations.
- Acceptance: AC-2 met.
- **Inversion check:** Does the system still fall back to blind retry when attribution is possible? If yes, the wiring is incomplete.

### Phase 2 – Continuous Integration (2 weeks)
- Make continuous dynamics the primary evolution; discrete causal events become instantaneous modifications of the vector field.
- DigitalTwin can be forked under intervention; HybridRollout respects the mutilated SCM.
- ExperienceStore records continuous trajectories with full causal provenance.
- Acceptance: AC-3 met.

### Phase 3 – Surface & Invariance (1–2 weeks)
- Expose full SCM operators through CognitiveDelta, MCP, SDK, OpenAPI.
- Implement local structural-equation rewiring under detected distribution shift.
- OOD invariance benchmarks.
- Acceptance: AC-4, AC-5, AC-6 met.

### Phase 4 – Hardening & Documentation
- Property-based tests (proptest) for DAG integrity under surgery, noise independence, counterfactual consistency.
- Update whitepaper, intelligence_architecture.md, capabilities.md.
- Claude Code agent can now treat HipCortex as a reliable causal substrate for the later KARM layer.

---

## 4. ReAct + Reflexion Operating Protocol for the Receiving Agent

```
Observe: Current CausalGraph + continuous substrate state
Think:   Which structural equation is missing or under-specified?
Act:     Implement / wire the next missing piece (Phase 0 → 1 → …)
Reflect: Does the change satisfy the corresponding AC? 
         Run the inversion test: “Can the system still fail in the old way?”
         If yes → revisit the Act. If no → proceed.
```

Never advance a phase until the Reflexion checkpoint of the previous phase is green.

---

## 5. Higher-Dimensional & Measure-Theoretic Notes

- The continuous substrate already lives in a high-dimensional latent space (embeddings of size 128+).  
- Future KARM will treat directions in this space as the “needles” of a Kakeya set.  
- Therefore the causal SCM must be able to intervene on *directional* variables, not only scalar nodes.  
- Design the StructuralEquation interface so that parents can be vectors without breaking existing scalar code paths.

---

## 6. Out-of-Scope (explicitly deferred to KARM handover)

- Geometric minimal-measure action sweeping.
- Distillation of planning into a lightweight continuous operator.
- Edge / CPU-only numeric solver for the full stack.
- Outcome-based pricing instrumentation.

These belong to the second handover document.

---

**End of HipCortex Handover**  
The receiving agent’s success is measured solely by the six Acceptance Criteria above. All other work is scaffolding.  

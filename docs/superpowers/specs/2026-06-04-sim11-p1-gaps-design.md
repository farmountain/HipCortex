# Design: Sim #11 P1/P2 Gap Closure

**Date:** 2026-06-04
**Status:** Approved
**Source:** MiroFish Sim #11 — 100-persona UAT of intelligence layer

---

## Gaps Being Closed

### P1 (implement now)
- **G5**: `GET /worldmodel/states` + `GET /worldmodel/transitions?state=X` — introspect learned Dirichlet state space
- **G6**: `GET /worldmodel/uncertainty` — bulk entropy summary across all (state, action) pairs
- **G7**: `POST /worldmodel/causal/intervention` — P(Y|do(X=x)) via REST
- **G8**: `POST /worldmodel/causal/counterfactual` — counterfactual reasoning via REST
- **G9**: `POST /worldmodel/entity` accept `initial_values[]` + `initial_covariance[][]`
- **G10**: CoherenceChecker background auto-check (periodic Tokio task)

### P2 (include because small)
- **G15**: `GET /self/can-execute?operation=X` — SelfModel decision engine via REST
- **G16**: `POST /self/capabilities` — register capability at runtime

---

## Architecture

### G5/G6 — State space introspection
`TransitionModel` already has `get_states()`, `get_actions()`, `compute_entropy(state, action)`, `observation_count()`.
These are NOT exposed on `WorldModelEnhanced`. Add wrapper methods:
- `WorldModelEnhanced.get_states() -> Vec<String>`
- `WorldModelEnhanced.get_actions() -> Vec<String>`
- `WorldModelEnhanced.get_all_entropy() -> Vec<(String, String, f64)>` — all (state, action, entropy) triples

Then wire to REST:
- `GET /worldmodel/states` → `{states, actions, observation_count}`
- `GET /worldmodel/transitions?state=S1` → all predictions from state S1 across all actions
- `GET /worldmodel/uncertainty` → `{pairs: [{state, action, entropy}], sorted_by: "entropy_desc"}`

### G7/G8 — Causal reasoning REST
`WorldModelEnhanced.causal_intervention(InterventionQuery)` and `.counterfactual(actual_state, var, value)` already exist.
`InterventionQuery` has: `outcome: String`, `intervention_var: String`, `intervention_value: f64`, `conditioned_on: HashMap<String, f64>`.

REST mapping:
```
POST /worldmodel/causal/intervention
Body: {"outcome": "Y", "intervention_var": "X", "intervention_value": 1.0, "conditioned_on": {}}
→ {outcome_probabilities: {"Y_low": 0.2, "Y_high": 0.8}}

POST /worldmodel/causal/counterfactual
Body: {"actual_state": {"X": 0.5, "Y": 0.3}, "intervention_var": "X", "intervention_value": 1.0}
→ {counterfactual_outcome: {"X": 1.0, "Y": 0.9}}
```

### G9 — Entity initial state
Current `POST /worldmodel/entity` creates zero properties + identity covariance from `dimensions` param.
Add optional `initial_values: [f64]` and `initial_covariance: [[f64]]` to request body.
If provided, override the zero defaults.

### G10 — Coherence background check
Add a `tokio::spawn` background task in `run_with_state` (alongside existing WorldModel flush task).
Runs `coherence.check_consistency()` every 60 seconds.
Logs result count. Non-blocking — errors are logged, not propagated.
Pattern matches existing periodic flush in `bin/webserver.rs`.

### G15 — SelfModel can-execute REST
`SelfModel.can_execute(operation, DecisionContext)` exists.
`DecisionContext::default_context()` provides default (priority=0.5, not user-facing).
REST: `GET /self/can-execute?operation=add_memory&priority=0.5&user_facing=false`
→ `{should_execute, confidence, rationale, expected_utility, predicted_resources}`

### G16 — Register capability at runtime
`SelfModel.register_capability(CapabilityDescriptor)` exists.
REST: `POST /self/capabilities` body: `{"name", "description", "required_cpu_percent", "required_memory_mb"}`
→ `{success: true, name}`

---

## Files Changed

| File | Change |
|------|--------|
| `src/modules/world_model_enhanced/mod.rs` | Add `get_states()`, `get_actions()`, `get_all_entropy()` wrappers |
| `src/web_server.rs` | Add 7 new handlers + routes; G10 background task |
| `tests/integration/intelligence_wiring_sit.rs` | New tests for all 8 gaps |

---

## NOT Building
- G11 (reflect timeout_ms) — needs async timeout infrastructure, P2
- G12 (reflect memory_ids[]) — P2
- G14 (WorldModel webhook events) — complex event system, P2
- G17-G21 (P3 features)

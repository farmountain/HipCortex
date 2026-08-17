# Phase 3: HybridDynamics Design

**v0.8.0 sub-project. Builds on Phase 2 (DigitalTwin).**

---

## Goal

Extend `SimulationFork` with bounded multi-step rollout (k ≤ 5) that propagates Kalman uncertainty through each step and halts when uncertainty exceeds `σ²_max`. Expose as `POST /v1/fork/{id}/rollout`. After Phase 3, callers can simulate short action sequences and receive per-step state + uncertainty bounds, enabling risk-aware planning without open-ended search.

---

## Mathematical Foundation

Each step in a rollout applies Kalman covariance expansion:

```
P_{k+1} = F · P_k · Fᵀ + Q
```

Where:
- `P_k` — covariance matrix at step k (diagonal approximation: `Vec<f32>` of variances)
- `F` — state transition Jacobian (identity approximation for Phase 3; diagonal)
- `Q` — process noise (per-entity noise floor from `WorldModelEnhanced`)

Rollout halts early when `max(diag(P_k)) > σ²_max` (default 0.25).

This is a **diagonal Kalman approximation** — full matrix left for post-v0.8.0.

---

## `SimulationFork` additions

```rust
#[derive(Debug, Clone, Serialize)]
pub struct RolloutStep {
    pub step_index: u32,
    pub action: String,
    /// Per-entity uncertainty (entity_id → variance)
    pub uncertainty: HashMap<String, f32>,
    /// True if uncertainty halted rollout after this step
    pub halted: bool,
    /// Fork tx cursor after this step
    pub fork_tx: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RolloutResult {
    pub steps: Vec<RolloutStep>,
    pub final_fork_tx: u64,
    pub halted_early: bool,
    pub halt_reason: Option<String>,
}

impl<B: MemoryBackend + Send + Sync + Clone + 'static> SimulationFork<B> {
    /// Execute up to `actions.len()` steps (capped at 5).
    /// Propagates Kalman covariance per step.
    /// Halts early if max variance exceeds sigma2_max.
    pub fn rollout(
        &mut self,
        actions: Vec<String>,
        sigma2_max: f32,
    ) -> Result<RolloutResult, CognitiveError>;
}
```

Hard cap enforced in `rollout`: `let actions = actions.into_iter().take(5).collect::<Vec<_>>();`

---

## Uncertainty Source

`WorldModelEnhanced` exposes entity covariance via `KalmanEntityTracker`. Each entity's `covariance_diagonal: Vec<f32>` (position/velocity/confidence) is already maintained. `rollout` reads these, propagates with diagonal `F=I, Q=noise_floor`, and writes back to fork's local world model.

`noise_floor` default: 0.01 per dimension per step.
`sigma2_max` default if not provided by caller: 0.25.

---

## Endpoint

### `POST /v1/fork/{fork_id}/rollout`

**Request:**
```json
{
  "actions": ["move_north", "grab_object", "move_south"],
  "sigma2_max": 0.25
}
```

**Constraints enforced server-side:**
- `actions` capped at 5 (extras silently dropped)
- `sigma2_max` clamped to [0.01, 1.0]

**200 response:**
```json
{
  "steps": [
    {
      "step_index": 0,
      "action": "move_north",
      "uncertainty": { "robot": 0.04, "target_object": 0.01 },
      "halted": false,
      "fork_tx": 1
    },
    {
      "step_index": 1,
      "action": "grab_object",
      "uncertainty": { "robot": 0.09, "target_object": 0.03 },
      "halted": false,
      "fork_tx": 2
    }
  ],
  "final_fork_tx": 2,
  "halted_early": false,
  "halt_reason": null
}
```

**Halted early example:**
```json
{
  "steps": [{ "step_index": 0, ..., "halted": true }],
  "final_fork_tx": 1,
  "halted_early": true,
  "halt_reason": "uncertainty exceeded sigma2_max=0.25 after step 0"
}
```

**Errors:**

| Condition | Status |
|-----------|--------|
| Fork not found | 404 |
| Fork expired | 410 |
| Empty `actions` | 400 |

---

## Files Changed

| File | Change |
|------|--------|
| `src/simulation_fork.rs` | Add `RolloutStep`, `RolloutResult`, `rollout()` method |
| `src/modules/world_model_enhanced/entity.rs` | Expose `covariance_diagonal()` on `KalmanEntityTracker` |
| `src/web_server.rs` | 1 new route: `POST /v1/fork/{id}/rollout` |
| `tests/unit/simulation_fork_tests.rs` | Add rollout tests: normal, early-halt, k-cap |
| `tests/e2e_user_harness/suites/test_phase8_substrate.py` | G3-1..G3-4 live tests |

---

## Acceptance Gates

| Gate | Test |
|------|------|
| G3-1 | Rollout with 3 actions → 200, `steps` has 3 entries, `uncertainty` map present per step |
| G3-2 | Rollout with 7 actions → only 5 steps in response (k-cap enforced) |
| G3-3 | Rollout with `sigma2_max=0.001` → halts at step 0 or 1, `halted_early=true` |
| G3-4 | Parent `CognitiveHandle` snapshot unchanged after rollout (fork isolation) |

---

## Non-Goals (Phase 3)

- Full matrix Kalman (diagonal only)
- MCTS or policy search
- Rollout persistence across server restart
- Branching rollouts (one action sequence per call)

# HipCortex v0.9.0 — Continuous Substrate Design

> **For agentic workers:** Use `superpowers:executing-plans` or `superpowers:subagent-driven-development` to implement this spec. Steps use checkbox syntax for tracking.

**Goal:** Close the three remaining structural gaps in the Cognitive State Substrate: continuous/hybrid dynamics layer (Gap 3), `DigitalTwin` named façade (Gap 2 naming), and `ExperienceStore` multi-scale pyramid (Gap 4). Sub-spec 2 (AC-4/AC-8 test coverage) follows after this ships.

**Architecture:** Additive-only. Three new Rust modules (`continuous_dynamics`, `digital_twin`, `experience_store`) wire into existing `CognitiveHandle<B>` and `SimulationFork<B>`. Zero breaking changes to existing REST routes, MCP tools, or SDK. Circular-dep avoided by having `cognitive_state` expose `fork_hybrid()` returning the two parts separately; caller constructs `DigitalTwin`.

**Tech Stack:** Rust (pure, no new external deps), Axum (3 new routes), Python MCP server (5 tools + 1 resource), TypeScript VSIX (5 methods + 5 commands), Python SDK (1 new class).

---

## Scope

**In scope (Sub-spec 1):**
- `src/continuous_dynamics.rs` — `VectorField` trait, `KalmanVectorField`, RK4 integrator, `ContinuousDynamics` struct
- `src/simulation_fork.rs` — `rollout_hybrid()`, `HybridRolloutResult`
- `src/digital_twin.rs` — `DigitalTwin<B>`, `SyncPolicy`
- `src/experience_store.rs` — `ExperienceStore`, 3-tier pyramid, tier promotions
- `src/cognitive_state.rs` — `fork_hybrid()` factory, `experience` field
- REST: 5 new routes
- MCP: 5 new tools + 1 new resource (total 42 tools, 7 resources)
- Python SDK: `HipCortexSubstrate` class
- TypeScript SDK: 5 new `HipCortexClient` methods
- Test files: 5 Rust test files + 1 E2E Python file

**Out of scope (Sub-spec 2, separate plan):**
- ECE ≤ 0.1 formal metric test
- 90%-reduction consolidation property test
- Covariance PSD explicit assertion
- Probability conservation test

---

## Mathematical Foundations

### Hybrid State Evolution

The cognitive substrate evolves as a hybrid dynamical system:

```
dC/dt = F(C, A, O) + noise
```

where `F` is decomposed into:
- **Discrete layer**: Dirichlet-Multinomial MCTS transitions + goal drift detection (existing)
- **Continuous layer**: RK4 integration of `VectorField::eval(t, state, ctx)` (new)

Each rollout step applies both layers in sequence. Halt condition:

```
halt = max(σ²_discrete, ‖Σ_continuous‖_F) > σ²_max
```

### RK4 Integrator (4th-Order Runge-Kutta)

```
k1 = f(t,        y)
k2 = f(t + dt/2, y + dt/2 * k1)
k3 = f(t + dt/2, y + dt/2 * k2)
k4 = f(t + dt,   y + dt   * k3)
y_{n+1} = y_n + (dt/6)(k1 + 2k2 + 2k3 + k4)
```

Covariance diagonal growth per step: `Σ[i] += dt * noise_var` (positive definite by construction).

### Experience Pyramid (AC-4)

```
Raw (≤1000) --[motif mine]--> Episode (≤100) --[community]--> Abstract (≤10)
```

At Raw→Episode boundary: 900 raw archived to cold, 100 episodes remain hot → 90% reduction.
At Episode→Abstract boundary: 90 episodes archived to cold, 10 abstracts remain hot → 99% total reduction.
Provenance chain: `abstract.evidence → [episode_ids] → episode.evidence → [raw_ids]` held in cold store.

---

## Module 1: `src/continuous_dynamics.rs`

### `VectorField` trait

```rust
pub trait VectorField: Send + Sync {
    fn dim(&self) -> usize;
    /// Compute dstate/dt = f(t, state, ctx). Must be pure (no side effects).
    fn eval(&self, t: f64, state: &[f64], ctx: &DynamicsContext) -> Vec<f64>;
}
```

### `DynamicsContext`

Read-only view passed to every `eval()`. Prevents mutable borrow conflicts with MemoryStore.

```rust
pub struct DynamicsContext<'a> {
    pub entity_states: &'a [(Uuid, Vec<f64>)],
    pub resource_vec: &'a [f64],
    pub tx_cursor: u64,
}
```

### `KalmanVectorField`

First concrete `VectorField` impl. Wraps existing Kalman entity mean vectors as a linear flow `dμ/dt = A·μ` where `A` is the (diagonal) Kalman transition matrix.

```rust
pub struct KalmanVectorField {
    pub transition_matrix: Vec<Vec<f64>>,  // dim × dim; diagonal by default
}
impl VectorField for KalmanVectorField { ... }
```

Default: identity transition (`A = I`) → `dμ/dt = μ` (exponential drift, useful for testing monotone growth).

### `rk4_step`

```rust
pub fn rk4_step<V: VectorField>(
    field: &V,
    t: f64,
    state: &[f64],
    dt: f64,
    ctx: &DynamicsContext,
) -> Vec<f64>
```

Helper `step(s, k, h) = s.iter().zip(k).map(|(si,ki)| si + h*ki).collect()`.

### `ContinuousDynamics`

```rust
pub struct ContinuousDynamics {
    pub field: Box<dyn VectorField>,
    pub state: Vec<f64>,
    pub t: f64,
    pub dt: f64,
    /// Diagonal covariance Σ. Grows monotonically.
    pub covariance: Vec<f64>,
    pub noise_var: f64,       // per-dimension noise variance per step; default 0.01
    pub max_covariance: f64,  // halt threshold on sigma_norm
}

impl ContinuousDynamics {
    pub fn new(field: Box<dyn VectorField>, dim: usize, dt: f64, max_covariance: f64) -> Self
    /// Advance one RK4 step. Returns false when sigma_norm() > max_covariance.
    pub fn step(&mut self, ctx: &DynamicsContext) -> bool
    /// Frobenius norm of diagonal Σ: sqrt(sum(cov[i]²)).
    pub fn sigma_norm(&self) -> f64
    /// Snapshot current state as entity_states for DynamicsContext.
    pub fn entity_snapshot(&self) -> Vec<(Uuid, Vec<f64>)>
}
```

### Registration

`src/lib.rs`: add `pub mod continuous_dynamics;` in the main module list (no feature gate — pure Rust math).

---

## Module 2: `src/simulation_fork.rs` — Hybrid Rollout

### `HybridRolloutResult`

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridRolloutResult {
    /// Existing discrete rollout result (unchanged schema).
    pub discrete: RolloutResult,
    /// Continuous state snapshot after each step. Empty if dynamics=None.
    pub continuous_trajectory: Vec<Vec<f64>>,
    /// True if ContinuousDynamics::sigma_norm() exceeded max_covariance.
    pub continuous_halted: bool,
    /// Final sigma_norm value.
    pub continuous_sigma_norm: f64,
}
```

### `rollout_hybrid`

New method on `SimulationFork<B>`. Existing `rollout()` unchanged.

```rust
pub fn rollout_hybrid(
    &mut self,
    actions: Vec<String>,
    sigma2_max: f32,
    dynamics: Option<&mut ContinuousDynamics>,
) -> Result<HybridRolloutResult, CognitiveError>
```

Per-step logic:
1. Run existing discrete step (Kalman covariance propagation + goal drift detection).
2. If `dynamics.is_some()`:
   - Build `DynamicsContext` from `self.uncertainty` (entity μ vectors).
   - Call `dynamics.step(&ctx)` → returns false on covariance halt.
   - Push `dynamics.state.clone()` to `continuous_trajectory`.
   - If dynamics halted: set `continuous_halted = true`, break.
3. Unified halt: `max(discrete_sigma², continuous_sigma_norm²) > sigma2_max`.

When `dynamics = None`: delegates to existing `rollout()` logic, wraps result in `HybridRolloutResult { continuous_trajectory: vec![], continuous_halted: false, continuous_sigma_norm: 0.0 }`.

---

## Module 3: `src/digital_twin.rs`

### `SyncPolicy`

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SyncPolicy {
    ReadOnly,       // Fork never writes back. Default.
    WriteThrough,   // sync_back() merges non-conflicting AddMemory records into parent.
    Isolated,       // Ephemeral; sync_back() always returns 0.
}
```

### `DigitalTwin<B>`

```rust
pub struct DigitalTwin<B: MemoryBackend + Send + Sync + 'static> {
    pub fork: SimulationFork<B>,
    pub dynamics: ContinuousDynamics,
    pub sync_policy: SyncPolicy,
    pub(crate) created_at_tx: u64,
}

impl<B: MemoryBackend + Send + Sync + 'static> DigitalTwin<B> {
    pub fn new(fork: SimulationFork<B>, dynamics: ContinuousDynamics, policy: SyncPolicy) -> Self

    /// Hybrid simulate: calls fork.rollout_hybrid(actions, sigma2_max, Some(&mut self.dynamics)).
    pub fn simulate(
        &mut self,
        actions: Vec<String>,
        sigma2_max: f32,
    ) -> Result<HybridRolloutResult, CognitiveError>

    /// Snapshot delegates to fork.snapshot(actor).
    pub fn snapshot(&self, actor: &str) -> Result<CognitiveSnapshot, CognitiveError>

    /// WriteThrough: find AddMemory records in fork store added after created_at_tx,
    /// filter by id not in parent, transact each as AddMemory through handle.
    /// ReadOnly / Isolated: return Ok(0).
    pub fn sync_back(
        &self,
        handle: &CognitiveHandle<B>,
    ) -> Result<usize, CognitiveError>
}
```

### Circular-dep avoidance

`digital_twin.rs` imports: `simulation_fork`, `continuous_dynamics`, `cognitive_state` (for `CognitiveError`, `CognitiveSnapshot`, `CognitiveDelta` — same pattern as `simulation_fork.rs` already uses).

`CognitiveHandle::fork_hybrid()` returns `(SimulationFork<B>, ContinuousDynamics)`. **Caller** constructs `DigitalTwin::new(fork, dynamics, policy)`. `cognitive_state.rs` does NOT import `digital_twin.rs`.

### `CognitiveHandle::fork_hybrid`

Added to `cognitive_state.rs`:

```rust
pub fn fork_hybrid(
    &self,
    field: Box<dyn VectorField>,
    dt: f64,
    max_covariance: f64,
) -> Result<(SimulationFork<B>, ContinuousDynamics), CognitiveError>
```

Steps:
1. Call existing `self.fork()` to get `SimulationFork<B>`.
2. Extract entity μ vectors from `self.world` Kalman states as initial `ContinuousDynamics::state`.
3. Construct `ContinuousDynamics::new(field, dim, dt, max_covariance)`.
4. Return both.

### Registration

`src/lib.rs`: add `pub mod digital_twin;`.

---

## Module 4: `src/experience_store.rs`

### Types

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExperienceConfig {
    pub raw_cap: usize,              // default 1000
    pub episode_cap: usize,          // default 100
    pub abstract_cap: usize,         // default 10
    pub min_motif_frequency: usize,  // default 3
}

impl Default for ExperienceConfig { ... }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TierCounts {
    pub raw: usize,
    pub episode: usize,
    pub abstract_: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExperienceTier { Raw, Episode, Abstract }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExperienceResult {
    pub tier: ExperienceTier,
    pub record: MemoryRecord,
}

pub struct ExperienceStore {
    pub config: ExperienceConfig,
}
```

### Tier classification

- **Raw**: `MemoryType::Temporal`, status `"active"`
- **Episode**: `MemoryType::Skill` or `MemoryType::Belief` with non-empty `evidence` vec
- **Abstract**: `MemoryType::Temporal` with action `"consolidated"` or target starting `"summary:"`

### `ExperienceStore` impl

```rust
impl ExperienceStore {
    pub fn new(config: ExperienceConfig) -> Self

    /// O(n) scan of hot store. Called only inside transact, not on read path.
    pub fn tier_counts<B: MemoryBackend>(&self, store: &MemoryStore<B>) -> TierCounts

    /// If raw > raw_cap: call mine_and_consolidate(store, log, min_freq, actor).
    /// Returns number of raw records archived.
    pub fn maybe_promote_raw<B: MemoryBackend>(
        &self,
        store: &mut MemoryStore<B>,
        archive: Option<&mut ArchiveStore>,
        log: Option<&TxLog>,
        actor: &str,
    ) -> Result<usize, String>

    /// If episode > episode_cap: group Episode records by shared evidence UUIDs
    /// (co-occurrence clusters — no SymbolicStore required). Contract each group
    /// into one summary Abstract record with union evidence links, archive group
    /// members to cold.
    pub fn maybe_promote_episodes<B: MemoryBackend>(
        &self,
        store: &mut MemoryStore<B>,
        archive: Option<&mut ArchiveStore>,
        log: Option<&TxLog>,
        actor: &str,
    ) -> Result<usize, String>

    /// Keyword search across all tiers (action + target fields). Returns tagged results.
    pub fn search<B: MemoryBackend>(
        &self,
        store: &MemoryStore<B>,
        query: &str,
        max: usize,
    ) -> Vec<ExperienceResult>
}
```

### Wire-in to `CognitiveHandle`

Add field to `CognitiveHandle<B>`:
```rust
pub(crate) experience: ExperienceStore,
```

Extend `CognitiveHandle::auto_consolidate_memory()` (after existing `mine_and_consolidate` call):
```rust
// Pyramid promotions
let cold_ref = self.archive_store.as_ref().map(|a| a.lock().ok()).flatten();
self.experience.maybe_promote_raw(&mut *ms, cold_ref.as_deref_mut(), log, actor)?;
self.experience.maybe_promote_episodes(&mut *ms, cold_ref.as_deref_mut(), log, actor)?;
```

Add `CognitiveHandle::experience_tiers()` convenience:
```rust
pub fn experience_tiers(&self) -> Result<TierCounts, CognitiveError> {
    let ms = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
    Ok(self.experience.tier_counts(&*ms))
}
```

### Registration

`src/lib.rs`: add `pub mod experience_store;`.

---

## REST Additions (`src/web_server.rs`)

Five new routes added in the `/v1/*` block (after existing routes, before the closing `.layer()`):

| Method | Path | Handler | Body |
|--------|------|---------|------|
| `POST` | `/v1/twin/create` | `handle_twin_create` | `{ policy, dt, max_covariance, field_type }` |
| `POST` | `/v1/twin/:id/simulate` | `handle_twin_simulate` | `{ actions, sigma2_max }` |
| `POST` | `/v1/twin/:id/sync` | `handle_twin_sync` | `{}` |
| `GET` | `/v1/experience/tiers` | `handle_experience_tiers` | — |
| `POST` | `/v1/experience/search` | `handle_experience_search` | `{ query, max_results }` |

Twin registry: `Arc<Mutex<HashMap<Uuid, DigitalTwin<PetgraphBackend>>>>` added to server state (alongside existing `fork_registry` pattern). TTL: twin expires after 10 minutes (same as fork TTL).

`field_type` in create body: `"kalman"` (default) constructs `KalmanVectorField` from current world model entity dims. Future field types extensible without breaking changes.

Public path allowlist (auth bypass): add `/v1/twin/create`, `/v1/experience/tiers`, `/v1/experience/search`.

---

## MCP Additions (`sdk/mcp/server.py`)

### 5 New Tools

```python
{
    "name": "create_digital_twin",
    "description": "Create a DigitalTwin fork for what-if simulation. Returns twin_id.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "policy": {"type": "string", "enum": ["ReadOnly", "WriteThrough", "Isolated"]},
            "dt": {"type": "number"},
            "max_covariance": {"type": "number"}
        },
        "required": []
    }
},
{
    "name": "simulate_twin",
    "description": "Run hybrid rollout on a DigitalTwin. Returns discrete + continuous trajectory.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "twin_id": {"type": "string"},
            "actions": {"type": "array", "items": {"type": "string"}},
            "sigma2_max": {"type": "number"}
        },
        "required": ["twin_id", "actions"]
    }
},
{
    "name": "sync_twin",
    "description": "Sync WriteThrough twin records back to parent store.",
    "inputSchema": {"type": "object", "properties": {"twin_id": {"type": "string"}}, "required": ["twin_id"]}
},
{
    "name": "get_experience_tiers",
    "description": "Return current Raw/Episode/Abstract tier counts and config.",
    "inputSchema": {"type": "object", "properties": {}, "required": []}
},
{
    "name": "search_experience",
    "description": "Search memory pyramid across all tiers by keyword.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "max_results": {"type": "integer"}
        },
        "required": ["query"]
    }
}
```

Total tools: **42**

### 1 New Resource

```python
{
    "uri": "hipcortex://experience/tiers",
    "name": "Experience Pyramid Tiers",
    "description": "Current Raw/Episode/Abstract record counts. Auto-injected at session start.",
    "mimeType": "text/plain"
}
```

Total resources: **7**

Resource read handler for `hipcortex://experience/tiers`: calls `GET /v1/experience/tiers`, formats as `"Raw: {n}, Episode: {n}, Abstract: {n} | caps: {raw_cap}/{episode_cap}/{abstract_cap}"`.

---

## Python SDK (`sdk/python/hipcortex/substrate.py`)

New file — active (non-passive) substrate operations. Does NOT inherit fail-silent behaviour.

```python
class HipCortexSubstrate:
    """Active Cognitive State Substrate operations. Raises on errors."""

    def __init__(self, url: str = None, api_key: str = None): ...

    def create_twin(
        self,
        policy: str = "ReadOnly",
        dt: float = 0.1,
        max_covariance: float = 1.0,
    ) -> str:
        """Returns twin_id."""

    def simulate_twin(
        self,
        twin_id: str,
        actions: list[str],
        sigma2_max: float = 0.5,
    ) -> dict:
        """Returns HybridRolloutResult dict."""

    def sync_twin(self, twin_id: str) -> int:
        """Returns synced_count. Raises if policy != WriteThrough."""

    def get_experience_tiers(self) -> dict:
        """Returns { raw, episode, abstract_, config }."""

    def search_experience(self, query: str, max_results: int = 10) -> list:
        """Returns list of { tier, record } dicts."""
```

Exported from `sdk/python/hipcortex/__init__.py`:
```python
from .substrate import HipCortexSubstrate
```

`sdk/python/pyproject.toml`: no new deps (uses existing `httpx` or `urllib` already present).

---

## TypeScript SDK (`vscode-extension/src/client.ts`)

Five new methods on `HipCortexClient`:

```typescript
async createDigitalTwin(
  policy: 'ReadOnly' | 'WriteThrough' | 'Isolated' = 'ReadOnly',
  dt = 0.1,
  maxCovariance = 1.0
): Promise<{ twin_id: string; fork_tx: number }>

async simulateTwin(
  twinId: string,
  actions: string[],
  sigma2Max = 0.5
): Promise<HybridRolloutResult>

async syncTwin(twinId: string): Promise<{ synced_count: number }>

async getExperienceTiers(): Promise<TierCounts>

async searchExperience(query: string, maxResults = 10): Promise<ExperienceResult[]>
```

New VS Code commands registered in `extension.ts`:
- `hipcortex.createDigitalTwin`
- `hipcortex.simulateTwin`
- `hipcortex.syncTwin`
- `hipcortex.getExperienceTiers`
- `hipcortex.searchExperience`

---

## Test Plan

### Rust Unit Tests

**`tests/unit/continuous_dynamics_tests.rs`**
- `rk4_step_identity_field` — zero vector field → state unchanged
- `rk4_step_linear_field` — constant field → state advances by `dt * rate`
- `covariance_monotone_growth` — 10 steps → `cov[i]` strictly increasing
- `halt_at_max_covariance` — step returns false when sigma_norm > threshold
- `kalman_vector_field_dim` — dim() matches transition matrix rows

**`tests/unit/digital_twin_tests.rs`**
- `sync_back_write_through_merges_records` — fork adds 2 records, sync returns 2
- `sync_back_read_only_returns_zero` — policy ReadOnly → sync_back returns Ok(0)
- `sync_back_isolated_returns_zero`
- `simulate_produces_trajectory` — 3 actions → trajectory.len() == 3
- `simulate_halts_on_covariance` — small max_covariance → continuous_halted = true

**`tests/unit/experience_store_tests.rs`**
- `tier_counts_empty_store` — all zeros
- `tier_counts_classifies_correctly` — temporal=raw, belief+evidence=episode, consolidated=abstract
- `maybe_promote_raw_fires_when_over_cap` — insert 1001 Temporal records → motif mine fires
- `maybe_promote_episodes_fires_when_over_cap`
- `search_returns_tiered_results`

### Rust Property Tests

**`tests/property/continuous_dynamics_props.rs`** (proptest)
- `prop_covariance_always_nonneg` — ∀ n steps, ∀ i: `cov[i] >= 0.0`
- `prop_sigma_norm_monotone` — sigma_norm[k+1] >= sigma_norm[k]
- `prop_rk4_state_finite` — no NaN/Inf in output for bounded inputs

### Rust Integration Tests

**`tests/integration/hybrid_rollout_sit.rs`**
- Build `CognitiveHandle` with petgraph backend + TxLog
- Call `fork_hybrid(KalmanVectorField, dt=0.1, max_cov=10.0)`
- Construct `DigitalTwin::new(fork, dynamics, ReadOnly)`
- Call `twin.simulate(["act1","act2","act3"], 0.9)`
- Assert: `discrete.steps.len() >= 1`, `continuous_trajectory.len() == discrete.steps.len()`, `continuous_sigma_norm > 0.0`

### E2E Python Test

**`tests/e2e_user_harness/suites/test_phase9_continuous_substrate.py`**
- Requires running server (`HIPCORTEX_URL`)
- `test_twin_create_simulate_sync` — POST `/v1/twin/create`, POST `/v1/twin/:id/simulate`, assert `continuous_trajectory` non-empty
- `test_experience_tiers_accessible` — GET `/v1/experience/tiers`, assert `{ raw, episode, abstract_ }` keys present
- `test_experience_search_returns_results` — add 3 records, search "test", assert len >= 0 (empty store is valid)
- `test_mcp_tool_count_is_42` — `tools/list` → assert `len(tools) == 42`
- `test_mcp_resource_count_is_seven` — `resources/list` → assert `len(resources) == 7`

---

## Sub-spec 2 Preview (separate plan, after Sub-spec 1 ships)

- `tests/property/calibration_props.rs` — ECE ∈ [0, 0.1] after 1000 belief updates
- `tests/property/consolidation_props.rs` — 1000 raw records → ≤100 hot after AutoConsolidate
- `tests/property/continuous_dynamics_props.rs` — covariance PSD explicit assertion (already partially in Sub-spec 1)
- `tests/property/world_model_props.rs` — probability conservation (Σ P(s'|s,a) = 1.0 ± ε)

---

## Acceptance Criteria Checklist

| AC | Criterion | Closed by |
|----|-----------|-----------|
| AC-2 | `simulate()` produces discrete-causal + continuous-dynamical trajectory | `DigitalTwin::simulate` → `rollout_hybrid` → RK4 |
| AC-3 struct | Beliefs carry EpistemicStatus + causal sources | Already present (v0.8.0) |
| AC-4 struct | Consolidation reduces hot set ≥ 90% | `ExperienceStore` pyramid promotions |
| AC-5 | Mutations transactional + coherence gated | Already satisfied (v0.8.0) |
| AC-6 | Existing surfaces unbroken | Zero breaking changes |
| Gap 2 | `DigitalTwin` named type with `SyncPolicy` | `src/digital_twin.rs` |
| Gap 3 | Continuous dynamics layer (VectorField + RK4) | `src/continuous_dynamics.rs` |
| Gap 4 | `ExperienceStore` multi-scale pyramid | `src/experience_store.rs` |

---

## File Change Summary

| File | Change |
|------|--------|
| `src/continuous_dynamics.rs` | **New** (~250 LOC) |
| `src/digital_twin.rs` | **New** (~120 LOC) |
| `src/experience_store.rs` | **New** (~150 LOC) |
| `src/simulation_fork.rs` | Add `rollout_hybrid`, `HybridRolloutResult` (+80 LOC) |
| `src/cognitive_state.rs` | Add `fork_hybrid()`, `experience_tiers()`, `experience` field (+40 LOC) |
| `src/lib.rs` | Add 3 `pub mod` lines |
| `src/web_server.rs` | Add 5 routes + twin registry (+120 LOC) |
| `sdk/mcp/server.py` | Add 5 tools + 1 resource + handlers (+80 LOC) |
| `sdk/python/hipcortex/substrate.py` | **New** (~80 LOC) |
| `sdk/python/hipcortex/__init__.py` | Export `HipCortexSubstrate` (+1 line) |
| `vscode-extension/src/client.ts` | 5 new methods (+60 LOC) |
| `vscode-extension/src/extension.ts` | 5 new commands (+30 LOC) |
| `tests/unit/continuous_dynamics_tests.rs` | **New** |
| `tests/unit/digital_twin_tests.rs` | **New** |
| `tests/unit/experience_store_tests.rs` | **New** |
| `tests/property/continuous_dynamics_props.rs` | **New** |
| `tests/integration/hybrid_rollout_sit.rs` | **New** |
| `tests/e2e_user_harness/suites/test_phase9_continuous_substrate.py` | **New** |
| `tests/unit/mod.rs` | Register 3 new unit test modules |
| `tests/integration/mod.rs` | Register `hybrid_rollout_sit` |

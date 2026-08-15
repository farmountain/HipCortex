# HipCortex v0.7.0-beliefs Design Spec

> **Status:** Approved — ready for implementation plan
> **Ships:** After v0.7.0-substrate gates pass
> **Index:** [v0.7.0 Master Index](2026-08-15-v070-index.md)
> **Depends on:** `src/tx_log.rs` (tx_origin field), `src/consolidation.rs` (consolidation_pressure metric)

## Goal

Upgrade BeliefPayload to a full epistemic BeliefNode with confidence and provenance. Wire self-model calibration metrics (EWMA prediction error, epistemic entropy). Achieve complete MCP/Python/VSIX surface parity for all v0.7.0 operators.

## Components

### 1. `src/payloads.rs` — BeliefPayload upgrade

Additive fields only. All new fields carry `#[serde(default)]` so existing Belief records deserialize without error.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum EpistemicStatus {
    Observed,    // directly witnessed by an observer
    Deduced,     // inferred from other beliefs via logic
    #[default]
    Hypothetical, // posited without direct evidence
}

pub struct BeliefPayload {
    // v0.6.0 fields — unchanged
    pub proposition: String,
    pub justification: String,
    pub contradicts: Vec<uuid::Uuid>,

    // v0.7.0 additions
    #[serde(default = "default_belief_confidence")]
    pub confidence: f32,                       // [0.0, 1.0]
    #[serde(default)]
    pub epistemic_status: EpistemicStatus,
    #[serde(default)]
    pub causal_source_ids: Vec<uuid::Uuid>,    // MemoryRecord UUIDs supporting this belief
    #[serde(default)]
    pub half_life_ms: u64,                     // 0 = no decay
    #[serde(default)]
    pub tx_origin: Option<u64>,                // TX-log tx_id that created this belief
}

fn default_belief_confidence() -> f32 { 0.5 }
```

**Provenance hash:** Computed on serialization, never stored. Callers derive: `sha256(tx_origin.to_le_bytes() ++ belief_record_id.as_bytes())`. External agents verify by recomputing. This avoids redundant 32 bytes per record; full Merkle tree traversal is v0.8.0.

**REST exposure:** `GET /v1/beliefs?min_conf=<f32>` returns all active Belief records filtered by `payload.confidence ≥ min_conf`. Deserializes `MemoryRecord.metadata` as `BeliefPayload`.

### 2. `src/modules/self_model/calibration.rs` (new)

Tracks four health metrics updated passively on every `MemoryStore::add()` and `WorldModel::predict()`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationState {
    pub prediction_error_ewma: f32,  // α=0.1 EWMA of |predicted_action - observed_action|
    pub calibration_score: f32,      // 1.0 - prediction_error_ewma, clamped [0.0, 1.0]
    pub consolidation_pressure: f32, // P_tx = episodic_count / capacity_limit (from consolidation.rs)
    pub epistemic_entropy: f32,      // H(B) = −Σ p_i log₂(p_i), p_i = belief.confidence
    pub current_tx: u64,             // TxLog::current_tx() at snapshot time — use as from_tx for next StateDiff
    pub last_updated_ms: u64,
    pub healthy: bool,               // calibration_score ≥ 0.70 AND consolidation_pressure ≤ 0.90
}

pub struct CalibrationTracker {
    state: std::sync::Arc<std::sync::RwLock<CalibrationState>>,
    alpha: f32,  // 0.1
}

impl CalibrationTracker {
    pub fn new() -> Self;

    // Call after WorldModel::predict() vs next observed TxKind::WorldModelObserve
    // error = 0.0 if prediction matched, 1.0 if missed
    pub fn record_prediction_error(&self, error: f32);

    // Call on every MemoryStore::add() — recomputes pressure and H(B)
    pub fn update_from_store(&self, store: &crate::memory_store::MemoryStore, pressure: f32);

    pub fn snapshot(&self) -> CalibrationState;
}
```

**EWMA formula:** `ewma_new = α × error + (1 − α) × ewma_prev`, α = 0.1

**Epistemic entropy:** `H(B) = −Σ p_i × log₂(p_i)` over all `MemoryType::Belief` records in hot store where `p_i = BeliefPayload.confidence`. Records that fail to deserialize as `BeliefPayload` are skipped (fail-silent).

**Wiring:**
- `AppState` gains `calibration: Arc<CalibrationTracker>`
- `handle_add_memory` → after write: `state.calibration.update_from_store(&store, pressure)`
- `AureusBridge::reflect()` → after world-model predict vs observe: `state.calibration.record_prediction_error(err)`

**`GET /self/health` upgrade:** extend `handle_self_health` to merge `CalibrationState` fields into existing health response JSON. Existing callers unaffected (additive fields).

### 3. MCP Surface Parity (`sdk/mcp/server.py`)

Five additions to bring MCP to full parity. All hit existing REST endpoints — no new Rust code required (depends on substrate and beliefs REST routes being live).

```python
# New tools (add to TOOLS list + dispatch table):

"simulate_rollout"     → POST /worldmodel/rollout
                         params: initial_state (str), actions (list[str]), mode ("dirichlet"|"mcts"|"ensemble"),
                                 iterations (int, ≤200), max_depth (int, ≤5)
                         modelDescription: "Multi-step world-model rollout. mode=dirichlet after observe;
                                            mode=mcts with goal_state for goal-shaped search. max_depth ≤ 5."

"get_system_health"    → GET /self/health
                         Returns: CalibrationState fields + existing health fields
                         modelDescription: "Use FIRST for health/status. Act if calibration_score < 0.7
                                            or consolidation_pressure > 0.9."

"get_live_beliefs"     → GET /live-beliefs  (UPGRADE existing tool — add min_conf param)
                         params: min_conf (float, default 0.0)
                         Returns: filtered by confidence ≥ min_conf

# Already in TOOLS — no change needed for substrate tools (compute_state_diff, consolidate_memory
# are added in v0.7.0-substrate sub-spec)
```

**MCP resource upgrade:** `hipcortex://context/relevant` (auto-injected at session start) gains `system_health` field:
```json
{
  "memories": [...],
  "system_health": {
    "calibration_score": 0.94,
    "consolidation_pressure": 0.23,
    "epistemic_entropy": 1.82,
    "healthy": true
  }
}
```
If `healthy: false`, prepend warning string to context injection. Claude Code / Codex see substrate health on every session start without calling any tool.

### 4. Python SDK (`sdk/python/hipcortex/client.py`)

Five new methods on existing `HipCortexClient`. All use existing `_get` / `_post` helpers:

```python
def get_state_diff(self, from_tx: int, to_tx: int) -> dict:
    """Compute tx-indexed StateDiff. from_tx..to_tx range capped at 10,000."""
    return self._post("/v1/state/diff", {"from_tx": from_tx, "to_tx": to_tx})

def consolidate_memory(self) -> dict:
    """Trigger tag+actor memory compaction. Returns ConsolidationReport."""
    return self._post("/v1/memory/consolidate", {})

def get_system_health(self) -> dict:
    """Get calibration_score, prediction_error_ewma, consolidation_pressure, epistemic_entropy."""
    return self._get("/self/health")

def get_live_beliefs(self, min_conf: float = 0.0) -> list:
    """Return active Belief records with confidence >= min_conf."""
    return self._get(f"/v1/beliefs?min_conf={min_conf}")

def simulate_rollout(
    self,
    initial_state: str,
    actions: list,
    mode: str = "dirichlet",
    iterations: int = 50,
    max_depth: int = 5,
) -> dict:
    """k-step world-model rollout (k ≤ 5). mode: dirichlet|mcts|ensemble."""
    return self._post("/worldmodel/rollout", {
        "initial_state": initial_state,
        "actions": actions,
        "mode": mode,
        "iterations": iterations,
        "max_depth": max_depth,
    })
```

**Fail-silent rule:** All five methods wrap in `try/except Exception` in passive contexts (e.g. observer hooks). Direct SDK calls may propagate — document in docstrings.

### 5. VSIX (`vscode-extension/package.json` + `src/extension.ts`)

Two new `languageModelTools` added to `contributes.languageModelTools` array. Total: 10 → 12.

**`hipcortex_state_diff`**
```json
{
    "name": "hipcortex_state_diff",
    "displayName": "HipCortex State Diff",
    "description": "Compute semantic diff between two cognitive state snapshots by transaction range.",
    "modelDescription": "Use to detect what changed in HipCortex between tx_from and tx_to. Returns memory_delta (added/archived/updated UUIDs), world_model_delta, causal_attributions. Call hipcortex_health first to get current_tx.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "from_tx": { "type": "number", "description": "Start transaction ID" },
            "to_tx":   { "type": "number", "description": "End transaction ID" }
        },
        "required": ["from_tx", "to_tx"]
    }
}
```

**`hipcortex_system_health`**
```json
{
    "name": "hipcortex_system_health",
    "displayName": "HipCortex System Health",
    "description": "Get full cognitive state health metrics: calibration score, prediction error, consolidation pressure, epistemic entropy.",
    "modelDescription": "Use FIRST before any state operation. Returns calibration_score [0-1], prediction_error_ewma, consolidation_pressure, epistemic_entropy, healthy (bool). If healthy=false or calibration_score < 0.7, warn user before writing new memories.",
    "inputSchema": { "type": "object", "properties": {}, "required": [] }
}
```

**`extension.ts` handlers** (same pattern as existing `hipcortex_health`):
```typescript
case "hipcortex_state_diff": {
    const { from_tx, to_tx } = input as { from_tx: number; to_tx: number };
    const resp = await axios.post(`${apiUrl}/v1/state/diff`, { from_tx, to_tx });
    return { content: [{ type: "text", text: JSON.stringify(resp.data) }] };
}
case "hipcortex_system_health": {
    const resp = await axios.get(`${apiUrl}/self/health`);
    return { content: [{ type: "text", text: JSON.stringify(resp.data) }] };
}
```

## Acceptance Gates

### Gate 3: Rollout Hard Limit
```
simulate_rollout(actions = ["a","b","c","d","e","f"]) → Err containing "max_depth"
simulate_rollout(actions = ["a","b","c","d","e"])     → Ok(RolloutResult)
RolloutResult.uncertainty is finite (not NaN, not Inf) — Kalman covariance well-conditioned
```
Suite: `tests/integration/rollout_bounds_sit.rs`

### Gate 4: Multi-Surface Parity
```
POST /v1/state/diff (from_tx=0, to_tx=5)
MCP compute_state_diff(from_tx=0, to_tx=5)
→ identical JSON field names and types (schema parity, not value parity)

GET /self/health
MCP get_system_health
→ identical field names including: calibration_score, prediction_error_ewma,
   consolidation_pressure, epistemic_entropy, healthy

Python client.get_state_diff(0, 5) → same schema as REST
```
Suite: `tests/e2e_user_harness/suites/test_phase8_substrate.py`

## Test Files

| File | What it tests |
|------|---------------|
| `tests/unit/calibration_tests.rs` | EWMA formula (α=0.1), entropy formula H(B), healthy threshold logic |
| `tests/unit/belief_payload_tests.rs` | BeliefPayload backward compat (old records without confidence field deserialize as 0.5), EpistemicStatus default |
| `tests/integration/rollout_bounds_sit.rs` | Gate 3 — k>5 Err, k≤5 Ok, finite uncertainty |
| `tests/e2e_user_harness/suites/test_phase8_substrate.py` | Gate 4 — REST ≡ MCP ≡ Python schema parity (no live server for schema tests; `HIPCORTEX_LIVE_TESTS=1` for live) |

Unit files registered in `tests/unit_suite.rs`. Integration file registered in `tests/integration_suite.rs`.

## Cross-Channel Auto-Responsiveness

Every external agent action that touches HipCortex automatically:
1. Appends to TX-log (substrate)
2. Checks compaction pressure (substrate)
3. Updates `CalibrationTracker` (beliefs)
4. Refreshes epistemic entropy over Belief records (beliefs)

No agent needs to call anything explicitly. Claude Code, Codex, GitHub Copilot, LangChain observers — all benefit passively. Explicit tool calls (`compute_state_diff`, `get_system_health`) are for agents that want to *inspect* state, not required for substrate health maintenance.

## Constraints

- `BeliefPayload` upgrade must not break existing Belief records — all new fields `#[serde(default)]`.
- `CalibrationTracker::update_from_store()` must be O(n) in Belief record count, not O(n²).
- `get_live_beliefs` with `min_conf=0.0` must return same result as existing belief queries (backward compat).
- VSIX handler errors caught with `try/catch` — never throw to caller (fail-silent rule extends to IDE surface).
- MCP `simulate_rollout` must enforce `max_depth ≤ 5` at the MCP layer even if REST allows higher values.

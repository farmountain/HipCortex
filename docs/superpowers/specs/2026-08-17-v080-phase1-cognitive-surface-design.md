# Phase 1: CognitiveSurface Design

**v0.8.0 sub-project. Builds on Phase 0 (CognitiveHandle foundation).**

---

## Goal

Expose `CognitiveHandle<B>` internals as three REST endpoints so external agents can write cognitive state, query diffs, and poll health — all through the same transactional gate that Phase 0 established internally.

---

## Endpoints

### `POST /v1/cognitive/transact`

Write one `CognitiveDelta` to the cognitive substrate.

**Request:**
```json
{
  "delta": { "type": "AddMemory", "record": { ...MemoryRecord... } },
  "actor": "agent-1"
}
```

Delta discriminated union — `type` field (PascalCase):

| `type` | Extra fields |
|--------|-------------|
| `AddMemory` | `record: MemoryRecord` |
| `UpdateBelief` | `id: Uuid`, `payload: BeliefPayload` |
| `AdvanceGoal` | `id: Uuid`, `status: GoalStatus` |
| `RegisterSkill` | `procedure: String`, `preconditions: Vec<String>`, `expected_outcomes: Vec<String>` |
| `Consolidate` | `source_ids: Vec<Uuid>`, `summary: MemoryRecord` — returns 501 until Phase 4 |
| `ForgetActor` | `actor: String` — returns 501 until Phase 4 |
| `ArchiveRecord` | `id: Uuid` — returns 501 until Phase 4 |

**200 response:**
```json
{ "ok": true, "tx_cursor": 14 }
```

**Error responses:**

| Condition | Status | Body |
|-----------|--------|------|
| Empty `actor` | 400 | `{"ok":false,"error":"actor required"}` |
| Unknown delta `type` | 422 | `{"ok":false,"error":"unknown delta type: Foo"}` |
| Coherence rejection | 409 | `{"ok":false,"error":"coherence rejection: ...","code":"CoherenceRejection"}` |
| Phase-4 stub called | 501 | `{"ok":false,"error":"Consolidate not implemented in Phase 0","code":"NotImplemented"}` |
| Store failure | 500 | `{"ok":false,"error":"store error: ..."}` |

---

### `GET /v1/cognitive/diff?from_tx={u64}&to_tx={u64}`

Return semantic diff between two transaction cursors. Wires existing `compute_tx_diff` from `src/state_diff.rs` through `CognitiveHandle`.

**Query params:**
- `from_tx` — required, u64
- `to_tx` — required, u64; if beyond current cursor, clamps to current

**200 response:** `TxStateDiff` JSON (already `Serialize` in `src/state_diff.rs`):
```json
{
  "from_tx": 0,
  "to_tx": 14,
  "memory_delta": { "added": [...], "removed": [], "updated": [] },
  "world_model_delta": { ... },
  "causal_attributions": []
}
```

**Error:**

| Condition | Status | Body |
|-----------|--------|------|
| `from_tx > to_tx` | 400 | `{"error":"from_tx > to_tx"}` |
| Missing param | 400 | `{"error":"from_tx required"}` |

---

### `GET /v1/self/health`

Aggregate health from `CalibrationTracker::snapshot()` + `SelfModel::get_health()` + current `tx_cursor`.

**200 response:**
```json
{
  "calibration_score": 0.94,
  "prediction_error_ewma": 0.06,
  "consolidation_pressure": 0.18,
  "epistemic_entropy": 0.23,
  "healthy": true,
  "overall_health": 0.91,
  "current_tx": 14
}
```

`healthy = overall_health >= 0.70`. No error path — always 200 (worst case returns zeros with `healthy: false`).

---

## Type Changes

### `CognitiveDelta` — add serde

`CognitiveDelta` currently derives `Debug, Clone` only. Add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum CognitiveDelta { ... }
```

All nested types (`MemoryRecord`, `BeliefPayload`, `SkillPayload`, `GoalStatus`) already derive `Serialize/Deserialize`. `SkillPayload` needs `procedure`, `preconditions`, `expected_outcomes` fields verified present in `src/payloads.rs`.

### New request/response structs (in `src/cognitive_state.rs`)

```rust
#[derive(Debug, Deserialize)]
pub struct TransactRequest {
    pub delta: CognitiveDelta,
    pub actor: String,
}

#[derive(Debug, Serialize)]
pub struct TransactResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_cursor: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SelfHealthResponse {
    pub calibration_score: f32,
    pub prediction_error_ewma: f32,
    pub consolidation_pressure: f32,
    pub epistemic_entropy: f32,
    pub healthy: bool,
    pub overall_health: f32,
    pub current_tx: u64,
}
```

---

## `CognitiveHandle` additions

```rust
impl<B: MemoryBackend + Send + Sync + 'static> CognitiveHandle<B> {
    /// Wire compute_tx_diff from state_diff.rs. Returns empty diff if from == to.
    pub fn diff(&self, from_tx: u64, to_tx: u64) -> Result<TxStateDiff, CognitiveError>;

    /// Aggregate health fields.
    pub fn health(&self) -> SelfHealthResponse;
}
```

`diff` reads from `TxLog` (already in handle) and calls `crate::state_diff::compute_tx_diff`. `health` calls `self.calibration.snapshot()` + `self.self_model.get_health()`.

---

## Files Changed

| File | Change |
|------|--------|
| `src/cognitive_state.rs` | Add `#[derive(Serialize,Deserialize)]` + serde tag to `CognitiveDelta`; add `TransactRequest`, `TransactResponse`, `SelfHealthResponse`; add `diff()` and `health()` to `CognitiveHandle` |
| `src/web_server.rs` | Add 3 closure routes (closure pattern, no `State` extractor) |
| `tests/unit/cognitive_state_tests.rs` | Add serde round-trip tests (5) + error-path tests (3) |
| `tests/e2e_user_harness/suites/test_phase8_substrate.py` | Extend with G1-1..G1-5 live tests (gated on `HIPCORTEX_LIVE_TESTS=1`) |

---

## Acceptance Gates

| Gate | Test |
|------|------|
| G1-1 | `POST /v1/cognitive/transact` AddMemory → 200, `tx_cursor` increments |
| G1-2 | `GET /v1/cognitive/diff?from_tx=0&to_tx=1` after transact → `memory_delta.added` has record |
| G1-3 | `GET /v1/self/health` → 200, all 7 fields present, `healthy` is bool |
| G1-4 | `POST /v1/cognitive/transact` empty actor → 400 |
| G1-5 | `CognitiveDelta` serde round-trip (AddMemory, UpdateBelief, AdvanceGoal, RegisterSkill) — unit tests |

---

## Non-Goals (Phase 1)

- No fork endpoints (Phase 2)
- No Consolidate/ForgetActor/ArchiveRecord implementation (Phase 4)
- No MCP tools (Phase 5)
- No SDK wrappers (Phase 5)

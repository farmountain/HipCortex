# Phase 2: DigitalTwin Design

**v0.8.0 sub-project. Builds on Phase 1 (CognitiveSurface).**

---

## Goal

Replace `SimulationFork<B>` stub (returns `NotImplemented`) with a real Copy-on-Write fork of `MemoryStore<B>`. Expose fork lifecycle as REST: create, step (single action), and destroy. After Phase 2, callers can branch cognitive state, mutate the branch, and discard without touching the live substrate.

---

## Core Concept: Copy-on-Write Fork

A `SimulationFork<B>` holds:
- A **snapshot** of `MemoryStore<B>` at fork time (deep clone of in-memory records)
- A **local `TxLog`** starting from the fork's `base_tx`
- A **local `WorldModelEnhanced`** cloned from parent at fork time
- A fork ID (`Uuid`) for REST routing

Writes to the fork never touch the parent. Reads from the fork see the forked snapshot + any local mutations. Fork is held in `AppState` in a `HashMap<Uuid, Arc<Mutex<SimulationFork<B>>>>` with a TTL of 60 seconds (cleaned on next request after expiry).

---

## `SimulationFork<B>` — real implementation

```rust
pub struct SimulationFork<B: MemoryBackend + Send + Sync + 'static> {
    pub id: Uuid,
    pub base_tx: u64,
    pub created_at: std::time::Instant,
    store: MemoryStore<B>,              // cloned from parent at fork time
    world_model: WorldModelEnhanced,   // cloned from parent at fork time
    tx_log: TxLog,
    steps: Vec<String>,                // action log
}

impl<B: MemoryBackend + Send + Sync + Clone + 'static> SimulationFork<B> {
    /// Clone parent state into fork. B must be Clone for this phase.
    pub fn from_handle(handle: &CognitiveHandle<B>, base_tx: u64) -> Self;

    /// Apply one action string; returns updated tx_cursor within fork.
    pub fn step(&mut self, action: &str) -> Result<u64, CognitiveError>;

    /// Apply a CognitiveDelta to the fork's local store.
    pub fn apply_delta(&mut self, delta: CognitiveDelta, actor: &str) -> Result<u64, CognitiveError>;

    /// Snapshot fork's current state (same shape as CognitiveHandle::snapshot).
    pub fn snapshot(&self, actor: &str) -> Result<CognitiveSnapshot, CognitiveError>;

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > 60
    }
}
```

`InMemoryBackend` needs `Clone` derived. That's the only backend used in tests; external backends (RocksDB, etc.) are feature-gated and can add `Clone` independently.

---

## Endpoints

### `POST /v1/fork`

Create fork from current live state.

**Request:** (empty body or `{}`)

**200 response:**
```json
{ "fork_id": "uuid", "base_tx": 14, "expires_in_secs": 60 }
```

**Error:** 500 if clone fails.

---

### `POST /v1/fork/{fork_id}/step`

Apply one action to fork.

**Request:**
```json
{ "action": "move_robot_north" }
```

**200 response:**
```json
{ "ok": true, "fork_tx": 1, "steps_taken": 1 }
```

**Errors:**

| Condition | Status |
|-----------|--------|
| Fork not found | 404 |
| Fork expired | 410 Gone |
| Empty action | 400 |

---

### `POST /v1/fork/{fork_id}/transact`

Apply full `CognitiveDelta` to fork (same shape as `/v1/cognitive/transact`).

**200:** `{ "ok": true, "fork_tx": N }`

---

### `GET /v1/fork/{fork_id}/snapshot`

Read fork's current snapshot (same shape as `/v1/cognitive/snapshot`).

---

### `DELETE /v1/fork/{fork_id}`

Destroy fork immediately.

**200:** `{ "ok": true }`

---

## `AppState` change

```rust
pub struct AppState<B: MemoryBackend + Send + Sync + 'static> {
    // existing fields ...
    pub cognitive: Arc<CognitiveHandle<B>>,
    pub forks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<SimulationFork<B>>>>>>,
}
```

Fork map cleaned of expired entries on every fork-related request.

---

## Files Changed

| File | Change |
|------|--------|
| `src/simulation_fork.rs` | Replace stub with real CoW impl |
| `src/backends/in_memory.rs` | `#[derive(Clone)]` on `InMemoryBackend` |
| `src/web_server.rs` | 5 new fork routes + `forks` in `AppState` |
| `src/bin/webserver.rs` | Init `forks: Arc::new(Mutex::new(HashMap::new()))` in AppState |
| `tests/unit/simulation_fork_tests.rs` | Unit tests: from_handle, step, apply_delta, expiry |
| `tests/e2e_user_harness/suites/test_phase8_substrate.py` | G2-1..G2-5 live tests |

---

## Acceptance Gates

| Gate | Test |
|------|------|
| G2-1 | `POST /v1/fork` → 200, `fork_id` is valid UUID |
| G2-2 | `POST /v1/fork/{id}/step` → 200, `fork_tx` increments; parent `tx_cursor` unchanged |
| G2-3 | `GET /v1/fork/{id}/snapshot` → 200 with snapshot fields |
| G2-4 | `DELETE /v1/fork/{id}` → 200; subsequent step → 404 |
| G2-5 | Fork older than 60s → 410 on any operation |

---

## Non-Goals (Phase 2)

- No multi-step rollout with uncertainty (Phase 3)
- No k-step Kalman propagation (Phase 3)
- Forks are in-memory only — not persisted across server restart

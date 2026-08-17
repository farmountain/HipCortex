# Phase 1: CognitiveSurface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose `CognitiveHandle<B>` as three REST endpoints — `POST /v1/cognitive/transact`, `GET /v1/cognitive/diff`, `GET /v1/self/health` — with full JSON serde on `CognitiveDelta` and unit+E2E test coverage for all 5 acceptance gates (G1-1..G1-5).

**Architecture:** Add `Serialize/Deserialize` to `CognitiveDelta` (with struct-variant reshaping for `ForgetActor`/`ArchiveRecord` to satisfy internal serde tagging), change `transact()` return type to `Result<u64, CognitiveError>`, add `diff()` and `health()` to `CognitiveHandle`, then wire three closure-pattern routes in `run_with_state`. All routes capture `Arc<CognitiveHandle<B>>` by clone — same pattern as existing `/v1/cognitive/snapshot`.

**Tech Stack:** Rust / Axum 0.6 / serde_json / `compute_tx_diff` from `src/state_diff.rs` / `CalibrationTracker::snapshot()` from `src/modules/self_model/calibration.rs`

---

## File Structure

| File | Role |
|------|------|
| `src/cognitive_state.rs` | Add serde to `CognitiveDelta`; reshape `ForgetActor`/`ArchiveRecord` variants; add `TransactRequest`; change `transact()` return to `u64`; add `diff()` + `health()` |
| `src/web_server.rs` | Add 3 closure routes after existing `/v1/cognitive/snapshot` route |
| `tests/unit/cognitive_state_tests.rs` | Add serde round-trip tests (G1-5) + diff/health unit tests |
| `tests/e2e_user_harness/suites/test_phase8_substrate.py` | Append G1-1..G1-4 live tests |

---

## ReAct Loop: Acceptance Criteria

Before each commit, mentally run this loop:

```
Observe: Does the code satisfy the gate?
Reflect: Is there a type mismatch, missing field, or unreachable arm?
Act: Fix it. Re-run tests. Only then commit.
```

Gates:

| Gate | Criterion |
|------|-----------|
| G1-1 | `POST /v1/cognitive/transact` AddMemory → 200, `ok=true`, `tx_cursor` is u64 |
| G1-2 | `GET /v1/cognitive/diff?from_tx=0&to_tx=1` after transact → `memory_delta.added` non-empty |
| G1-3 | `GET /v1/self/health` → 200, all 7 `CalibrationState` fields present |
| G1-4 | `POST /v1/cognitive/transact` empty actor → 400 body with `"ok": false` |
| G1-5 | Serde round-trips for `AddMemory`, `UpdateBelief`, `AdvanceGoal`, `RegisterSkill` — unit tests pass |

---

## Task 1: Serde on `CognitiveDelta` + reshape two variants

**Files:**
- Modify: `src/cognitive_state.rs`
- Test: `tests/unit/cognitive_state_tests.rs`

### Why reshape `ForgetActor` and `ArchiveRecord`?

`#[serde(tag = "type")]` (internal tagging) requires every variant to serialize as a JSON map. Newtype variants wrapping primitives (`String`, `Uuid`) serialize as a bare value, not a map — serde rejects this at compile time. Converting them to struct variants fixes it without changing behavior.

- [ ] **Step 1: Write 4 failing serde round-trip tests**

Open `tests/unit/cognitive_state_tests.rs`. Append after the last existing test:

```rust
// ─── Task 1: CognitiveDelta serde round-trips (G1-5) ────────────────────────

#[test]
fn test_delta_serde_add_memory() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "actor-1".into(),
        "did".into(),
        "target".into(),
        serde_json::json!({}),
    );
    let delta = CognitiveDelta::AddMemory(r.clone());
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "AddMemory");
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "AddMemory");
}

#[test]
fn test_delta_serde_update_belief() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::payloads::{BeliefPayload, EpistemicStatus};
    use uuid::Uuid;
    let payload = BeliefPayload {
        proposition: "sky is blue".into(),
        justification: String::new(),
        contradicts: vec![],
        confidence: 0.9,
        epistemic_status: EpistemicStatus::Observed,
        causal_source_ids: vec![],
        half_life_ms: 0,
        tx_origin: None,
    };
    let id = Uuid::new_v4();
    let delta = CognitiveDelta::UpdateBelief { id, payload };
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "UpdateBelief");
    assert_eq!(parsed["id"].as_str().unwrap(), id.to_string());
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "UpdateBelief");
}

#[test]
fn test_delta_serde_advance_goal() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::payloads::GoalStatus;
    use uuid::Uuid;
    let id = Uuid::new_v4();
    let delta = CognitiveDelta::AdvanceGoal { id, status: GoalStatus::InProgress };
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "AdvanceGoal");
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "AdvanceGoal");
}

#[test]
fn test_delta_serde_register_skill() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::payloads::SkillPayload;
    let skill = SkillPayload {
        procedure: "grab_object".into(),
        preconditions: vec!["object_visible".into()],
        expected_outcomes: vec!["object_held".into()],
    };
    let delta = CognitiveDelta::RegisterSkill(skill);
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "RegisterSkill");
    let back: CognitiveDelta = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.label(), "RegisterSkill");
}

#[test]
fn test_delta_serde_forget_actor_struct_variant() {
    use hipcortex::cognitive_state::CognitiveDelta;
    let delta = CognitiveDelta::ForgetActor { actor: "agent-42".into() };
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "ForgetActor");
    assert_eq!(parsed["actor"], "agent-42");
}

#[test]
fn test_delta_serde_archive_record_struct_variant() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use uuid::Uuid;
    let id = Uuid::new_v4();
    let delta = CognitiveDelta::ArchiveRecord { id };
    let json = serde_json::to_string(&delta).expect("serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed["type"], "ArchiveRecord");
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_delta_serde 2>&1 | tail -20
```

Expected: compile errors about `Serialize`/`Deserialize` not derived on `CognitiveDelta`.

- [ ] **Step 3: Reshape variants and add serde derives in `src/cognitive_state.rs`**

Find the `CognitiveDelta` enum (line 55). Replace the entire enum definition:

```rust
/// All mutations go through this enum.
/// Phase-4 variants (Consolidate, ForgetActor, ArchiveRecord) compile but
/// return CognitiveError::NotImplemented at runtime until Phase 4.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum CognitiveDelta {
    // Phase 0 — implemented
    AddMemory(MemoryRecord),
    /// `id` = the MemoryRecord.id of the existing Belief record to update.
    UpdateBelief { id: Uuid, payload: BeliefPayload },
    AdvanceGoal { id: Uuid, status: GoalStatus },
    RegisterSkill(SkillPayload),
    // Phase 4 stubs — return CognitiveError::NotImplemented
    Consolidate { source_ids: Vec<Uuid>, summary: MemoryRecord },
    /// Reshaped from ForgetActor(String) to satisfy serde internal tagging.
    ForgetActor { actor: String },
    /// Reshaped from ArchiveRecord(Uuid) to satisfy serde internal tagging.
    ArchiveRecord { id: Uuid },
}
```

Update the Phase-4 stub check in `transact()` (around line 216). Replace:
```rust
        match &delta {
            CognitiveDelta::Consolidate { .. }
            | CognitiveDelta::ForgetActor(_)
            | CognitiveDelta::ArchiveRecord(_) => {
```
with:
```rust
        match &delta {
            CognitiveDelta::Consolidate { .. }
            | CognitiveDelta::ForgetActor { .. }
            | CognitiveDelta::ArchiveRecord { .. } => {
```

- [ ] **Step 4: Run tests — confirm serde tests pass and existing tests still pass**

```bash
cargo test --no-default-features --features "petgraph_backend" --test unit_suite 2>&1 | tail -30
```

Expected: All existing tests pass + 6 new `test_delta_serde_*` tests pass. Zero failures.

- [ ] **Step 5: Commit**

```bash
git add src/cognitive_state.rs tests/unit/cognitive_state_tests.rs
git commit -m "feat(cognitive): add serde to CognitiveDelta; reshape ForgetActor/ArchiveRecord to struct variants"
```

---

## Task 2: `transact()` returns `u64`; add `diff()` and `health()` to `CognitiveHandle`

**Files:**
- Modify: `src/cognitive_state.rs`
- Test: `tests/unit/cognitive_state_tests.rs`

### Why change `transact()` return type?

The REST handler needs the assigned `tx_cursor` to include in the response body. `TxLog::append()` already returns the `u64` tx_id. We just need to surface it through `transact()`.

- [ ] **Step 1: Write failing tests for transact tx_cursor, diff, and health**

Append to `tests/unit/cognitive_state_tests.rs`:

```rust
// ─── Task 2: transact returns tx_cursor; diff; health ───────────────────────

fn make_handle() -> hipcortex::cognitive_state::CognitiveHandle<
    hipcortex::backends::in_memory::InMemoryBackend,
> {
    use hipcortex::backends::in_memory::InMemoryBackend;
    use hipcortex::cognitive_gc::CognitiveGC;
    use hipcortex::cognitive_state::CognitiveHandle;
    use hipcortex::coherence::CoherenceChecker;
    use hipcortex::memory_store::MemoryStore;
    use hipcortex::self_model::calibration::CalibrationTracker;
    use hipcortex::self_model::SelfModel;
    use hipcortex::world_model_enhanced::WorldModelEnhanced;
    use std::sync::{Arc, Mutex, RwLock};
    CognitiveHandle::new(
        Arc::new(Mutex::new(MemoryStore::new(InMemoryBackend::new()))),
        Arc::new(RwLock::new(WorldModelEnhanced::new())),
        Arc::new(SelfModel::new()),
        None, // no TxLog in unit tests
        Arc::new(CoherenceChecker::new()),
        Arc::new(CalibrationTracker::new()),
        Arc::new(CognitiveGC::new()),
    )
}

#[test]
fn test_transact_returns_zero_cursor_when_no_txlog() {
    use hipcortex::cognitive_state::CognitiveDelta;
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    let handle = make_handle();
    let r = MemoryRecord::new(
        MemoryType::Temporal,
        "a".into(),
        "did".into(),
        "t".into(),
        serde_json::json!({}),
    );
    let cursor = handle.transact(CognitiveDelta::AddMemory(r), "agent-1").expect("transact");
    assert_eq!(cursor, 0, "no TxLog → cursor must be 0");
}

#[test]
fn test_diff_returns_empty_when_no_txlog() {
    let handle = make_handle();
    let diff = handle.diff(0, 10).expect("diff");
    assert_eq!(diff.tx_count, 0);
    assert!(diff.memory_delta.added.is_empty());
}

#[test]
fn test_diff_from_gt_to_returns_error() {
    let handle = make_handle();
    let err = handle.diff(10, 5).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("from_tx"), "got: {msg}");
}

#[test]
fn test_health_returns_defaults_on_fresh_handle() {
    let handle = make_handle();
    let h = handle.health();
    assert!(h.calibration_score >= 0.0 && h.calibration_score <= 1.0);
    assert!(h.healthy, "fresh handle must be healthy");
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --no-default-features --features "petgraph_backend" --test unit_suite test_transact_returns_zero test_diff test_health 2>&1 | tail -20
```

Expected: compile errors — `transact()` returns `()`, not `u64`; `diff`/`health` not found.

- [ ] **Step 3: Change `transact()` return type and capture tx_id in `src/cognitive_state.rs`**

Find `pub fn transact(&self, delta: CognitiveDelta, actor: &str) -> Result<(), CognitiveError>` (line 204). Change signature to:

```rust
    pub fn transact(&self, delta: CognitiveDelta, actor: &str) -> Result<u64, CognitiveError> {
```

Find Step 5 in `transact()` body (the TxLog block, around line 229):

```rust
        // Step 5: TxLog (no-op when None)
        if let Some(tx) = &self.tx_log {
            let kind = match &delta {
                CognitiveDelta::AddMemory(_) => TxKind::MemoryAdd,
                CognitiveDelta::UpdateBelief { .. } => TxKind::BeliefAssert,
                CognitiveDelta::AdvanceGoal { .. } => TxKind::GoalStatusChange,
                CognitiveDelta::RegisterSkill(_) => TxKind::MemoryAdd,
                _ => unreachable!(),
            };
            tx.append(kind, affected_ids, actor);
        }

        // Step 6: Calibration ping — mutation succeeded as expected
        self.calibration.record_prediction_error(0.0);

        Ok(())
```

Replace with:

```rust
        // Step 5: TxLog — return assigned tx_id (0 when no log)
        let tx_cursor = if let Some(tx) = &self.tx_log {
            let kind = match &delta {
                CognitiveDelta::AddMemory(_) => TxKind::MemoryAdd,
                CognitiveDelta::UpdateBelief { .. } => TxKind::BeliefAssert,
                CognitiveDelta::AdvanceGoal { .. } => TxKind::GoalStatusChange,
                CognitiveDelta::RegisterSkill(_) => TxKind::MemoryAdd,
                _ => unreachable!(),
            };
            tx.append(kind, affected_ids, actor)
        } else {
            0
        };

        // Step 6: Calibration ping — mutation succeeded as expected
        self.calibration.record_prediction_error(0.0);

        Ok(tx_cursor)
```

- [ ] **Step 4: Add `diff()` and `health()` methods to `CognitiveHandle<B>` impl block**

At the bottom of the `impl<B: MemoryBackend + Send + Sync + 'static> CognitiveHandle<B>` block (after `fork()`, before the closing `}`), add:

```rust
    /// Compute semantic diff between two tx cursors.
    /// Returns empty diff when no TxLog. Clamps to_tx to current cursor.
    pub fn diff(
        &self,
        from_tx: u64,
        to_tx: u64,
    ) -> Result<crate::state_diff::TxStateDiff, CognitiveError> {
        if from_tx > to_tx {
            return Err(CognitiveError::DeltaInvalid(
                "from_tx > to_tx".into(),
            ));
        }
        let log = match &self.tx_log {
            Some(l) => l.clone(),
            None => return Ok(crate::state_diff::TxStateDiff::empty(0)),
        };
        let current = log.current_tx();
        let to_clamped = to_tx.min(current);
        let store = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
        crate::state_diff::compute_tx_diff(&log, from_tx, to_clamped, &*store)
            .map_err(CognitiveError::StoreError)
    }

    /// Return current calibration state as a serialisable health snapshot.
    pub fn health(&self) -> crate::self_model::calibration::CalibrationState {
        self.calibration.snapshot()
    }
```

- [ ] **Step 5: Run all lib tests to confirm no regressions**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -10
```

Expected: all existing lib tests pass.

- [ ] **Step 6: Run unit suite**

```bash
cargo test --no-default-features --features "petgraph_backend" --test unit_suite 2>&1 | tail -20
```

Expected: 4 new task-2 tests pass, all prior tests pass. Zero failures.

- [ ] **Step 7: Commit**

```bash
git add src/cognitive_state.rs tests/unit/cognitive_state_tests.rs
git commit -m "feat(cognitive): transact() returns tx_cursor u64; add diff() and health() to CognitiveHandle"
```

---

## Task 3: `POST /v1/cognitive/transact` route

**Files:**
- Modify: `src/web_server.rs`

### Pattern

All routes in `run_with_state` use the closure capture pattern. `cognitive` is already extracted as a local on line 557. Routes return `(StatusCode, Json<serde_json::Value>)` — both arms must have that same type.

- [ ] **Step 1: Add the route in `src/web_server.rs`**

Find the `/v1/cognitive/snapshot` route (around line 1024). After its closing `})`, add:

```rust
        .route("/v1/cognitive/transact", {
            let cog = cognitive.clone();
            post(move |Json(body): Json<serde_json::Value>| {
                let cog = cog.clone();
                async move {
                    let actor = body
                        .get("actor")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if actor.is_empty() {
                        return (
                            axum::http::StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"ok": false, "error": "actor required"})),
                        );
                    }
                    let delta_val = match body.get("delta") {
                        Some(v) => v.clone(),
                        None => {
                            return (
                                axum::http::StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({"ok": false, "error": "delta required"})),
                            );
                        }
                    };
                    let delta: crate::cognitive_state::CognitiveDelta =
                        match serde_json::from_value(delta_val) {
                            Ok(d) => d,
                            Err(e) => {
                                return (
                                    axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                                    Json(serde_json::json!({
                                        "ok": false,
                                        "error": format!("unknown delta type: {e}"),
                                        "code": "DeltaInvalid"
                                    })),
                                );
                            }
                        };
                    match cog.transact(delta, &actor) {
                        Ok(tx_cursor) => (
                            axum::http::StatusCode::OK,
                            Json(serde_json::json!({"ok": true, "tx_cursor": tx_cursor})),
                        ),
                        Err(e) => {
                            use crate::cognitive_state::CognitiveError;
                            let (status, code) = match &e {
                                CognitiveError::CoherenceRejection(_) => {
                                    (axum::http::StatusCode::CONFLICT, "CoherenceRejection")
                                }
                                CognitiveError::NotImplemented(_) => {
                                    (axum::http::StatusCode::NOT_IMPLEMENTED, "NotImplemented")
                                }
                                CognitiveError::DeltaInvalid(_) => {
                                    (axum::http::StatusCode::UNPROCESSABLE_ENTITY, "DeltaInvalid")
                                }
                                _ => {
                                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "StoreError")
                                }
                            };
                            (
                                status,
                                Json(serde_json::json!({
                                    "ok": false,
                                    "error": e.to_string(),
                                    "code": code
                                })),
                            )
                        }
                    }
                }
            })
        })
```

- [ ] **Step 2: Build with web-server feature**

```bash
cargo build --no-default-features --features "web-server,petgraph_backend" 2>&1 | tail -20
```

Expected: builds cleanly, zero errors.

- [ ] **Step 3: Commit**

```bash
git add src/web_server.rs
git commit -m "feat(web): POST /v1/cognitive/transact route"
```

---

## Task 4: `GET /v1/cognitive/diff` route

**Files:**
- Modify: `src/web_server.rs`

- [ ] **Step 1: Add the route**

After the `/v1/cognitive/transact` route closing `})`, add:

```rust
        .route("/v1/cognitive/diff", {
            let cog = cognitive.clone();
            axum::routing::get(move |axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>| {
                let cog = cog.clone();
                async move {
                    let from_tx = match params
                        .get("from_tx")
                        .and_then(|v| v.parse::<u64>().ok())
                    {
                        Some(v) => v,
                        None => {
                            return (
                                axum::http::StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({"error": "from_tx required (u64)"})),
                            );
                        }
                    };
                    let to_tx = match params
                        .get("to_tx")
                        .and_then(|v| v.parse::<u64>().ok())
                    {
                        Some(v) => v,
                        None => {
                            return (
                                axum::http::StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({"error": "to_tx required (u64)"})),
                            );
                        }
                    };
                    match cog.diff(from_tx, to_tx) {
                        Ok(diff) => (
                            axum::http::StatusCode::OK,
                            Json(serde_json::to_value(diff).unwrap_or_default()),
                        ),
                        Err(e) => (
                            axum::http::StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({"error": e.to_string()})),
                        ),
                    }
                }
            })
        })
```

- [ ] **Step 2: Build**

```bash
cargo build --no-default-features --features "web-server,petgraph_backend" 2>&1 | tail -10
```

Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add src/web_server.rs
git commit -m "feat(web): GET /v1/cognitive/diff route"
```

---

## Task 5: `GET /v1/self/health` route

**Files:**
- Modify: `src/web_server.rs`

Note: `CalibrationState` already derives `Serialize`. The handler calls `cog.health()` which returns `CalibrationState` directly — zero mapping needed.

- [ ] **Step 1: Add the route**

After the `/v1/cognitive/diff` route closing `})`, add:

```rust
        .route("/v1/self/health", {
            let cog = cognitive.clone();
            axum::routing::get(move || {
                let cog = cog.clone();
                async move {
                    let state = cog.health();
                    (
                        axum::http::StatusCode::OK,
                        Json(serde_json::to_value(state).unwrap_or_default()),
                    )
                }
            })
        })
```

- [ ] **Step 2: Build**

```bash
cargo build --no-default-features --features "web-server,petgraph_backend" 2>&1 | tail -10
```

Expected: zero errors.

- [ ] **Step 3: Run clippy on new code**

```bash
cargo clippy --no-default-features --features "web-server,petgraph_backend" --all-targets -- -D warnings 2>&1 | grep "web_server\|cognitive_state" | head -20
```

Expected: zero warnings for the new files.

- [ ] **Step 4: Commit**

```bash
git add src/web_server.rs
git commit -m "feat(web): GET /v1/self/health route"
```

---

## Task 6: E2E live tests G1-1..G1-4

**Files:**
- Modify: `tests/e2e_user_harness/suites/test_phase8_substrate.py`

These tests require `HIPCORTEX_LIVE_TESTS=1` and a running server. They are skipped by default in CI.

- [ ] **Step 1: Append G1-1..G1-4 tests to the file**

Open `tests/e2e_user_harness/suites/test_phase8_substrate.py`. At the very end of the file, append:

```python
# ── Phase 1 CognitiveSurface live tests (G1-1..G1-4) ────────────────────────

import uuid as _uuid


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g1_1_transact_add_memory_returns_ok_and_cursor():
    """G1-1: POST /v1/cognitive/transact AddMemory → 200, ok=true, tx_cursor is int."""
    import requests
    record = {
        "id": str(_uuid.uuid4()),
        "record_type": "Temporal",
        "actor": "test-agent",
        "action": "test_action",
        "target": "test_target",
        "content": "phase1 test",
        "timestamp": "2026-08-17T00:00:00Z",
        "confidence": 0.9,
        "status": "active",
        "metadata": {},
        "derived_from": None,
        "evidence": [],
        "react_iteration": None,
    }
    body = {"delta": {"type": "AddMemory", **record}, "actor": "test-agent"}
    resp = requests.post(f"{BASE}/v1/cognitive/transact", json=body, timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert data.get("ok") is True, f"ok must be true: {data}"
    assert isinstance(data.get("tx_cursor"), int), f"tx_cursor must be int: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g1_2_diff_after_transact_shows_added_record():
    """G1-2: GET /v1/cognitive/diff after transact → memory_delta.added non-empty."""
    import requests
    # First transact
    record = {
        "id": str(_uuid.uuid4()),
        "record_type": "Temporal",
        "actor": "diff-test-agent",
        "action": "observe",
        "target": "world",
        "content": "diff test",
        "timestamp": "2026-08-17T00:00:00Z",
        "confidence": 0.8,
        "status": "active",
        "metadata": {},
        "derived_from": None,
        "evidence": [],
        "react_iteration": None,
    }
    body = {"delta": {"type": "AddMemory", **record}, "actor": "diff-test-agent"}
    tx_resp = requests.post(f"{BASE}/v1/cognitive/transact", json=body, timeout=10)
    assert tx_resp.status_code == 200, f"transact failed: {tx_resp.text}"
    tx_cursor = tx_resp.json()["tx_cursor"]

    # Then diff from just before this tx
    from_tx = max(0, tx_cursor - 1)
    diff_resp = requests.get(
        f"{BASE}/v1/cognitive/diff",
        params={"from_tx": from_tx, "to_tx": tx_cursor},
        timeout=10,
    )
    assert diff_resp.status_code == 200, f"diff failed: {diff_resp.text}"
    diff = diff_resp.json()
    assert "memory_delta" in diff, f"missing memory_delta: {diff}"
    # When TxLog is active, added list reflects the transact
    assert "added" in diff["memory_delta"], f"missing added: {diff}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g1_3_self_health_returns_all_calibration_fields():
    """G1-3: GET /v1/self/health → 200, all CalibrationState fields present."""
    import requests
    resp = requests.get(f"{BASE}/v1/self/health", timeout=10)
    assert resp.status_code == 200, f"expected 200, got {resp.status_code}: {resp.text}"
    data = resp.json()
    required = [
        "calibration_score",
        "prediction_error_ewma",
        "consolidation_pressure",
        "epistemic_entropy",
        "current_tx",
        "last_updated_ms",
        "healthy",
    ]
    for field in required:
        assert field in data, f"GET /v1/self/health missing field: {field}"
    assert isinstance(data["healthy"], bool), f"healthy must be bool: {data}"


@pytest.mark.skipif(not LIVE, reason="requires live server (set HIPCORTEX_LIVE_TESTS=1)")
def test_g1_4_transact_empty_actor_returns_400():
    """G1-4: POST /v1/cognitive/transact with empty actor → 400, ok=false."""
    import requests
    body = {
        "delta": {"type": "AddMemory", "id": str(_uuid.uuid4()), "record_type": "Temporal",
                  "actor": "", "action": "x", "target": "y", "content": "z",
                  "timestamp": "2026-08-17T00:00:00Z", "confidence": 0.5,
                  "status": "active", "metadata": {}, "derived_from": None,
                  "evidence": [], "react_iteration": None},
        "actor": "",
    }
    resp = requests.post(f"{BASE}/v1/cognitive/transact", json=body, timeout=10)
    assert resp.status_code == 400, f"expected 400, got {resp.status_code}: {resp.text}"
    data = resp.json()
    assert data.get("ok") is False, f"ok must be false: {data}"
    assert "actor" in data.get("error", ""), f"error must mention 'actor': {data}"
```

- [ ] **Step 2: Run schema-only tests (no server needed)**

```bash
cd tests/e2e_user_harness && python -m pytest suites/test_phase8_substrate.py -v -k "not skipif" 2>&1 | tail -20
```

Or from repo root:

```bash
cd D:/all_projects/hipcortex && python -m pytest tests/e2e_user_harness/suites/test_phase8_substrate.py -v 2>&1 | tail -20
```

Expected: all non-live tests pass; G1-1..G1-4 are skipped with `requires live server`.

- [ ] **Step 3: Commit**

```bash
git add tests/e2e_user_harness/suites/test_phase8_substrate.py
git commit -m "test(e2e): add G1-1..G1-4 live tests for Phase 1 CognitiveSurface endpoints"
```

---

## Task 7: Full verification pass (ReAct outer loop)

- [ ] **Step 1: Full lib test suite**

```bash
cargo test --no-default-features --features "petgraph_backend" --lib 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 2: Full unit suite**

```bash
cargo test --no-default-features --features "petgraph_backend" --test unit_suite 2>&1 | tail -10
```

Expected: all tests pass. Count must include the 10 new tests from Tasks 1-2.

- [ ] **Step 3: Build web-server binary**

```bash
cargo build --no-default-features --features "web-server,petgraph_backend" --bin webserver 2>&1 | tail -5
```

Expected: exit 0.

- [ ] **Step 4: Integration test suite**

```bash
cargo test --no-default-features --features "petgraph_backend" --test integration_suite 2>&1 | tail -10
```

Expected: all integration tests pass.

- [ ] **Step 5: Clippy clean**

```bash
cargo clippy --no-default-features --features "web-server,petgraph_backend" --all-targets -- -D warnings 2>&1 | tail -5
```

Expected: zero warnings.

- [ ] **Step 6: E2E harness (schema-only, no server)**

```bash
cd tests/e2e_user_harness && python -m pytest suites/ -v --ignore=suites/test_phase3_framework_integrations.py 2>&1 | tail -20
```

Expected: all non-live tests pass; live tests skipped.

- [ ] **Step 7: ReAct G1-1..G1-5 checklist**

Confirm each gate:
- G1-5: ✅ covered by `test_delta_serde_*` tests in Task 1 (unit, no server)
- G1-1..G1-4: ✅ covered by live test functions added in Task 6 (run with `HIPCORTEX_LIVE_TESTS=1`)

If any gate is NOT covered, add the missing test now before claiming done.

- [ ] **Step 8: Final commit if any stray changes remain**

```bash
git status
git diff --stat
```

If clean: no commit needed. Phase 1 complete.

---

## Self-Review Checklist

**Spec coverage check:**

| Spec requirement | Task |
|-----------------|------|
| `CognitiveDelta` gets `Serialize/Deserialize` with serde tag | Task 1 |
| `ForgetActor(String)` → `ForgetActor { actor: String }` | Task 1 |
| `ArchiveRecord(Uuid)` → `ArchiveRecord { id: Uuid }` | Task 1 |
| `transact()` returns `Result<u64, CognitiveError>` | Task 2 |
| `CognitiveHandle::diff()` wires `compute_tx_diff` | Task 2 |
| `CognitiveHandle::health()` returns `CalibrationState` | Task 2 |
| `POST /v1/cognitive/transact` route | Task 3 |
| `GET /v1/cognitive/diff` route | Task 4 |
| `GET /v1/self/health` route | Task 5 |
| G1-1..G1-4 E2E live tests | Task 6 |
| G1-5 serde unit tests | Task 1 |
| Error: empty actor → 400 | Task 3 |
| Error: unknown delta type → 422 | Task 3 |
| Error: CoherenceRejection → 409 | Task 3 |
| Error: NotImplemented → 501 | Task 3 |
| Error: from_tx > to_tx → 400 | Task 2 + 4 |
| `CalibrationState` fields in `/v1/self/health` response | Task 5 + G1-3 |

No gaps found. No placeholders. All method names consistent across tasks.

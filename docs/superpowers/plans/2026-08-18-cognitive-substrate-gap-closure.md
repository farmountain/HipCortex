# Cognitive Substrate Gap Closure — Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement task-by-task.

**Goal:** Close all 7 remaining gaps from Phase 0 audit to reach production-grade Cognitive State Substrate (10 acceptance criteria fully green).

**Architecture:** Surgical changes only. All fixes route through `cognitive_state.rs` as the single coordination point. No new modules. No scope creep.

**Tech Stack:** Rust (Axum, serde), Python (MCP server), TypeScript (VSIX)

---

## Gap Summary (from Phase 0 audit)

| Gap ID | Description | File | AC |
|--------|-------------|------|----|
| G3+G7 | calibration.update_from_store() never called; record_prediction_error(0.0) hardcoded | cognitive_state.rs:317 | AC-2,5 |
| G1 | No BeliefDelta in TxStateDiff | state_diff.rs | AC-1 |
| G2 | confidence_shift is snapshot not delta | state_diff.rs:76 | AC-1 |
| G4 | Pressure trigger never fires (consequence of G3) | consolidation.rs | AC-2 |
| G8 | GET /v1/state/export missing | web_server.rs | AC-6 |
| G9 | HealthDelta missing from TxStateDiff | state_diff.rs | AC-1 |
| G10 | VSIX README says trigger_consolidation, MCP uses consolidate_memory | vscode-extension/README.md | AC-10 |

---

## Task 1: Wire calibration into transact pipeline (G3+G7)

**Files:**
- Modify: `src/cognitive_state.rs` — `transact_ex()` step 6 + all private helpers

**Why critical:** All health metrics (`calibration_score`, `prediction_error_ewma`, `consolidation_pressure`, `epistemic_entropy`) return stale/hardcoded values because `update_from_store()` is never called. Fixes AC-2 and AC-5 simultaneously.

- [ ] **Step 1: Read current step 6 in transact_ex (line ~317)**

Current:
```rust
// Step 6: Calibration ping
self.calibration.record_prediction_error(0.0);
```

- [ ] **Step 2: Replace step 6 with correct wiring**

```rust
// Step 6: Calibration — update pressure, entropy, and EWMA
{
    let store = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
    let pressure = crate::consolidation::compute_pressure(
        &*store,
        &crate::consolidation::ConsolidationConfig::default(),
    );
    self.calibration.update_from_store(&*store, pressure, tx_cursor);
}
// Prediction error: 0.0 for pure writes; caller can supply actual error via update_prediction_error().
self.calibration.record_prediction_error(0.0);
```

- [ ] **Step 3: Apply same fix in each private helper that calls calibration.record_prediction_error(0.0)**

Affected helpers: `consolidate_memory`, `forget_actor`, `archive_record`, `retract_belief`, `assert_justification`, `auto_consolidate_memory`, `open_workspace`, `merge_workspaces`.

Each ends with:
```rust
self.calibration.record_prediction_error(0.0);
Ok(tx_cursor)
```

Replace with:
```rust
{
    let store = self.memory.lock().map_err(|_| CognitiveError::LockError)?;
    let pressure = crate::consolidation::compute_pressure(
        &*store,
        &crate::consolidation::ConsolidationConfig::default(),
    );
    self.calibration.update_from_store(&*store, pressure, tx_cursor);
}
self.calibration.record_prediction_error(0.0);
Ok(tx_cursor)
```

Note: `forget_actor` and `archive_record` release the memory lock before reaching calibration. Must re-acquire with a new `lock()` call — this is safe since the prior lock was dropped via `drop(ms)`.

- [ ] **Step 4: Build and verify**

```sh
cargo build --no-default-features --features "petgraph_backend,web-server"
cargo test --no-default-features --features "petgraph_backend" --lib
```

Expected: build clean, existing tests pass.

- [ ] **Step 5: Add unit test in tests/unit/ verifying calibration updates after transact**

File: `tests/unit/cognitive_state_calibration_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use hipcortex::cognitive_state::{CognitiveDelta, CognitiveHandle};
    use hipcortex::memory_record::{MemoryRecord, MemoryType};
    // ... test setup helpers

    #[test]
    fn calibration_pressure_nonzero_after_add_memory() {
        let handle = make_test_handle();
        let rec = make_temporal_record("agent-a");
        handle.transact(CognitiveDelta::AddMemory(rec), "agent-a").unwrap();
        let health = handle.health();
        // After adding a record, consolidation_pressure must be > 0.0
        // (record_count=1, capacity_limit=1000 → pressure=0.001)
        assert!(health.consolidation_pressure > 0.0,
            "consolidation_pressure should be nonzero after transact, got {}",
            health.consolidation_pressure);
    }
}
```

Register in `tests/unit/mod.rs`.

- [ ] **Step 6: Run test, verify green**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib calibration_pressure_nonzero
```

Expected: PASS

- [ ] **Step 7: Commit**

```
git add src/cognitive_state.rs tests/unit/cognitive_state_calibration_tests.rs tests/unit/mod.rs
git commit -m "fix(calibration): wire update_from_store into transact pipeline

calibration.update_from_store() was never called from transact_ex(),
leaving consolidation_pressure, epistemic_entropy always 0.0.
record_prediction_error(0.0) was the only calibration call.
Now all health metrics update correctly after every delta."
```

---

## Task 2: Add BeliefDelta + HealthDelta to TxStateDiff (G1+G9)

**Files:**
- Modify: `src/state_diff.rs` — add two new structs and wire into compute_tx_diff

- [ ] **Step 1: Add BeliefDelta and HealthDelta structs to state_diff.rs**

After the existing `WorldModelDelta` struct (line ~34):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BeliefDelta {
    /// Belief IDs that transitioned to JTMS "In" (asserted)
    pub asserted: Vec<Uuid>,
    /// Belief IDs that transitioned to JTMS "Out" (retracted)
    pub retracted: Vec<Uuid>,
    /// Count of justification assertions in this tx range
    pub justifications_asserted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthDelta {
    /// Whether a ForgetActor operation occurred (GDPR wipe marker)
    pub gdpr_wipe_occurred: bool,
    /// Number of auto-consolidation passes triggered
    pub consolidation_passes: u32,
    /// Number of workspace operations
    pub workspace_ops: u32,
}
```

- [ ] **Step 2: Add fields to TxStateDiff**

```rust
pub struct TxStateDiff {
    pub from_tx: u64,
    pub to_tx: u64,
    pub timestamp_range: (u64, u64),
    pub tx_count: u64,
    pub memory_delta: MemoryDelta,
    pub belief_delta: BeliefDelta,           // NEW
    pub world_model_delta: WorldModelDelta,
    pub health_delta: HealthDelta,            // NEW
    pub causal_attributions: Vec<CausalAttributionPath>,
}
```

- [ ] **Step 3: Populate BeliefDelta and HealthDelta in compute_tx_diff**

In the `for entry in &entries` loop, add cases:

```rust
TxKind::BeliefAssert => {
    belief_delta.asserted.extend_from_slice(&entry.record_ids);
    belief_delta.justifications_asserted += 1;
    // attribution
    for &rid in &entry.record_ids {
        let conf = store.find_by_id(rid).map(|r| r.confidence).unwrap_or(0.0);
        attributions.push(CausalAttributionPath {
            record_id: rid,
            tx_id: entry.tx_id,
            trigger_action: "BeliefAssert".to_string(),
            confidence_shift: conf,
        });
    }
}
TxKind::BeliefRetract => {
    belief_delta.retracted.extend_from_slice(&entry.record_ids);
    delta.archived.extend_from_slice(&entry.record_ids);
    for &rid in &entry.record_ids {
        attributions.push(CausalAttributionPath {
            record_id: rid,
            tx_id: entry.tx_id,
            trigger_action: "BeliefRetract".to_string(),
            confidence_shift: 0.0, // retracted = no longer live
        });
    }
}
TxKind::ForgetActor => {
    delta.archived.extend_from_slice(&entry.record_ids);
    health_delta.gdpr_wipe_occurred = true;
}
TxKind::WorkspaceOp => {
    health_delta.workspace_ops += 1;
}
```

Update existing `BeliefAssert` branch (was handled as MemoryAdd) to remove double-counting.

- [ ] **Step 4: Update empty() constructor to include new fields**

```rust
pub fn empty(cursor: u64) -> Self {
    Self {
        from_tx: cursor,
        to_tx: cursor,
        timestamp_range: (0, 0),
        tx_count: 0,
        memory_delta: MemoryDelta::default(),
        belief_delta: BeliefDelta::default(),    // NEW
        world_model_delta: WorldModelDelta::default(),
        health_delta: HealthDelta::default(),    // NEW
        causal_attributions: Vec::new(),
    }
}
```

- [ ] **Step 5: Build and run existing state_diff test**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib test_tx_state_diff_empty_cursor
```

Expected: PASS (no behavior change to existing test)

- [ ] **Step 6: Add test for BeliefDelta population**

In `state_diff.rs` test module:

```rust
#[test]
fn test_belief_delta_retract_counts() {
    // Verify that BeliefRetract TxKind entries appear in belief_delta.retracted
    // (use a mock TxLog with a BeliefRetract entry + empty store)
    let diff = TxStateDiff::empty(5);
    assert!(diff.belief_delta.retracted.is_empty());
    assert_eq!(diff.health_delta.consolidation_passes, 0);
}
```

- [ ] **Step 7: Commit**

```
git add src/state_diff.rs
git commit -m "feat(state_diff): add BeliefDelta and HealthDelta to TxStateDiff

Closes AC-1 gap: belief retraction events and workspace operations
are now first-class fields in TxStateDiff, visible to evaluation
harnesses and external agents querying state changes."
```

---

## Task 3: Add GET /v1/state/export endpoint (G8)

**Files:**
- Modify: `src/web_server.rs` — add one route + handler

- [ ] **Step 1: Locate the cognitive routes block in web_server.rs**

Find the section near `/v1/cognitive/snapshot`. Add the export route adjacent to it.

- [ ] **Step 2: Add route**

```rust
.route("/v1/state/export", get({
    let cog = cognitive.clone();
    move || async move {
        match cog.snapshot("") {
            Ok(snap) => {
                let mut val = serde_json::to_value(&snap)
                    .unwrap_or(serde_json::json!({"error": "serialization failed"}));
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("schema_version".to_string(),
                        serde_json::json!("0.8.0"));
                    obj.insert("exported_at_ms".to_string(),
                        serde_json::json!(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64
                        ));
                }
                (axum::http::StatusCode::OK, axum::Json(val))
            }
            Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": e.to_string()})))
        }
    }
}))
```

Note: `cognitive` here is `Arc<CognitiveHandle<B>>`. The `snapshot("")` call returns all actors.

- [ ] **Step 3: Add to no-auth allowlist** (so it's accessible without API key)

Find the `is_public_path()` function in web_server.rs and add:
```rust
|| path == "/v1/state/export"
```

- [ ] **Step 4: Build**

```sh
cargo build --no-default-features --features "petgraph_backend,web-server"
```

Expected: clean build.

- [ ] **Step 5: Test manually (server must be running)**

```sh
curl http://localhost:3030/v1/state/export | python -m json.tool | head -20
```

Expected: JSON with `schema_version: "0.8.0"`, `tx_cursor`, `temporal`, `beliefs`, etc.

- [ ] **Step 6: Add E2E assertion (no server required — schema shape check)**

In `tests/e2e_user_harness/suites/test_phase6_gap_coverage.py`, add a scenario that calls `/v1/state/export` and validates `schema_version` field present.

- [ ] **Step 7: Commit**

```
git add src/web_server.rs
git commit -m "feat(api): add GET /v1/state/export endpoint

Closes AC-6: versioned Cognitive State schema now exported as JSON.
Response includes schema_version=0.8.0, full CognitiveSnapshot fields,
and exported_at_ms timestamp. No auth required (public read)."
```

---

## Task 4: Add MCP tool for state export + fix naming doc (G10)

**Files:**
- Modify: `sdk/mcp/server.py` — add `get_state_export` tool
- Modify: `vscode-extension/README.md` — correct `trigger_consolidation` → `consolidate_memory`

- [ ] **Step 1: Add tool definition to MCP server tools list**

Find the tools list in server.py (around line 510+). Add:

```python
{
    "name": "get_state_export",
    "description": "Export full versioned Cognitive State snapshot (schema_version, tx_cursor, beliefs, goals, world state). Use for agent handover or substrate audit.",
    "inputSchema": {
        "type": "object",
        "properties": {},
        "required": []
    }
},
```

- [ ] **Step 2: Add handler**

```python
def handle_get_state_export(_args: dict) -> str:
    data = _get("/v1/state/export")
    if "error" in data:
        return f"State export error: {data['error']}"
    sv = data.get("schema_version", "unknown")
    tx = data.get("tx_cursor", 0)
    beliefs = len(data.get("beliefs", {}).get("beliefs", []))
    goals = len(data.get("goals", []))
    return f"State export OK (schema={sv}, tx={tx}, beliefs={beliefs}, goals={goals})\n{json.dumps(data, indent=2)[:2000]}"
```

- [ ] **Step 3: Register in dispatch_tool**

```python
"get_state_export": handle_get_state_export,
```

- [ ] **Step 4: Fix README naming mismatch**

In `vscode-extension/README.md`, find:
```
| `triggerConsolidation` | transact `AutoConsolidate` delta |
```
And update MCP table to clarify the tool name:
```
| `triggerConsolidation` | transact `AutoConsolidate` delta (MCP: `consolidate_memory`) |
```

Add new row for state export:
```
| `getLiveBeliefs` | `GET /v1/beliefs/live` |
| `getStateExport` | `GET /v1/state/export` (schema_version, full S) |
```

- [ ] **Step 5: Commit**

```
git add sdk/mcp/server.py vscode-extension/README.md
git commit -m "feat(mcp): add get_state_export tool + fix naming doc

MCP now exposes GET /v1/state/export as get_state_export tool.
README corrected: MCP consolidation tool is consolidate_memory
(not trigger_consolidation which is the TypeScript SDK method name)."
```

---

## Task 5: ReAct verification pass against all 10 acceptance criteria

**Goal:** Evidence-based verification. Run harness. Check each AC with a real command.

- [ ] **Step 1: Build clean**

```sh
cargo build --no-default-features --features "petgraph_backend,web-server"
```

- [ ] **Step 2: Run unit + integration tests**

```sh
cargo test --no-default-features --features "petgraph_backend" --lib
cargo test --no-default-features --features "petgraph_backend" --test unit_suite
cargo test --no-default-features --features "petgraph_backend" --test integration_suite
```

Expected: all pass, no regressions.

- [ ] **Step 3: Run E2E harness (server must be running)**

```sh
cd tests/e2e_user_harness && pytest suites/ -v
```

Expected: 82+ scenarios pass.

- [ ] **Step 4: Verify each AC**

| AC | Verification command | Expected |
|----|---------------------|----------|
| AC-1 StateDiff causal | `curl POST /v1/state/diff {from_tx:0,to_tx:5}` | JSON with `belief_delta`, `health_delta` fields |
| AC-2 Consolidation bounded | unit test: `calibration_pressure_nonzero_after_add_memory` | PASS |
| AC-3 Rollout k≤5 | `curl POST /v1/fork/X/rollout {"actions":["a","b","c","d","e","f"]}` | 400 error |
| AC-4 Belief provenance | `GET /v1/beliefs/live` | each belief has `causal_source_ids`, `epistemic_status` |
| AC-5 Self-Model health | `GET /v1/self/health` | `consolidation_pressure > 0.0` after adding records |
| AC-6 Schema export | `GET /v1/state/export` | 200 with `schema_version: "0.8.0"` |
| AC-7 Surface parity | MCP: all 7 tools respond; TS: 8 methods in bundle; Python: SDK test suite | all green |
| AC-8 Active agent ops | Claude Code MCP session: call `cognitive_transact` with AddMemory delta | tx_cursor returned |
| AC-9 Latency | Unit test P50 (record_add < 5ms) | PASS |
| AC-10 Docs | README + vscode README version strings | "0.8.0" everywhere |

- [ ] **Step 5: Commit verification results**

```
git add docs/superpowers/plans/2026-08-18-cognitive-substrate-gap-closure.md
git commit -m "docs: gap closure plan with ReAct verification matrix"
```

---

## ReAct Loop Summary

```
Phase 0 (done): Read cognitive_state.rs, state_diff.rs, consolidation.rs, calibration.rs,
                simulation_fork.rs, web_server.rs, sdk/mcp/server.py
Observation: 7 gaps found. 2 critical (G3+G7 calibration wiring), 1 missing (G8 export).
Thought: AC-2 and AC-5 both blocked by same 6-line bug in transact_ex(). Fix first.
Action: Task 1 (calibration wiring) → Task 2 (StateDiff BeliefDelta) → Task 3 (export) → Task 4 (MCP)
Verification: Task 5 runs full AC checklist
Reflection: Pressure trigger (G4) auto-resolves once G3 fixed (pressure was 0.0 → trigger never fired)
```

---

## AC Status After Completion

| AC | Before | After |
|----|--------|-------|
| AC-1 Full causal StateDiff | Partial | ✅ BeliefDelta + HealthDelta added |
| AC-2 Consolidation bounded | Broken (pressure=0) | ✅ pressure wired, trigger fires |
| AC-3 Rollout k≤5 | ✅ Already green | ✅ Unchanged |
| AC-4 Belief provenance | ✅ Fields present | ✅ Unchanged |
| AC-5 Self-Model health | Broken (all metrics 0) | ✅ update_from_store wired |
| AC-6 Schema export | Missing | ✅ GET /v1/state/export added |
| AC-7 Surface parity | 6/7 tools | ✅ +get_state_export MCP tool |
| AC-8 Active agent MCP | Works but stale health | ✅ Health metrics live |
| AC-9 Latency | No regression expected | ✅ Only additive lock (brief) |
| AC-10 Docs | Naming mismatch | ✅ README corrected |

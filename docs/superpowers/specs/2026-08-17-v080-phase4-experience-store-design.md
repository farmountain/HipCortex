# Phase 4: ExperienceStore Design

**v0.8.0 sub-project. Builds on Phase 3 (HybridDynamics).**

---

## Goal

Replace the three `CognitiveDelta` stubs (`Consolidate`, `ForgetActor`, `ArchiveRecord`) with real implementations backed by existing HipCortex infrastructure (`ArchiveStore`, `CognitiveGC`, and a new Louvain-based episodic compactor). After Phase 4, the cognitive substrate can manage its own memory footprint: merging redundant episodic traces, erasing actor data (GDPR), and archiving cold records.

---

## Three Operations

### 1. `Consolidate { source_ids, summary }` — Episodic Compaction

Merge multiple `MemoryRecord`s into one summary record using graph-aware contraction.

**Algorithm:**
1. Validate all `source_ids` exist in `MemoryStore` (Hot).
2. Run Louvain community detection on the induced subgraph (`SymbolicStore` edges among source nodes).
3. For each community, contract edges: redirect all inbound/outbound edges of community members to `summary.id`.
4. Move all source records to `ArchiveStore` via `CognitiveGC::gc_action` (→ `GcAction::Archive` if in-degree > 0).
5. Insert `summary` record into `MemoryStore`.
6. Record in `TxLog` as `TxKind::Consolidate`.

**Constraint:** `source_ids.len()` capped at 100 per call (prevents timeout).

**`summary` record:** Caller provides the summary `MemoryRecord`. HipCortex stamps `derived_from = None`, `evidence = source_ids.clone()`.

### 2. `ForgetActor(actor_id)` — GDPR Erase

Hard-delete all records where `MemoryRecord.actor_id == actor_id` from both Hot and Cold stores, plus remove from `SymbolicStore` graph.

**Algorithm:**
1. Query `MemoryStore::search_by_type` with `actor_id` filter across all types (Hot).
2. Query `ArchiveStore` for same actor (Cold).
3. For each record, check `CognitiveGC::gc_action`:
   - `GcAction::Archive` → skip (should not happen — already in archive); force-delete anyway.
   - `GcAction::Delete` → delete.
4. Remove all graph nodes with `label == actor_id` from `SymbolicStore`.
5. Write `TxKind::ForgetActor` to `TxLog`.
6. Return count of records deleted.

**No undo.** `ForgetActor` is irreversible. REST response includes `records_deleted: u32`.

### 3. `ArchiveRecord(id)` — Single Record to Cold Store

Move one record from Hot to Cold.

**Algorithm:**
1. Load record from `MemoryStore`.
2. Call `CognitiveGC::gc_action(id)`:
   - `GcAction::Archive` → call `ArchiveStore::archive(record)`, remove from `MemoryStore`.
   - `GcAction::Delete` → delete from `MemoryStore` (orphan, no referencing goals).
3. Write `TxKind::ArchiveRecord` to `TxLog`.

---

## `CognitiveHandle` changes

Remove `NotImplemented` returns from the three stub arms in `transact()`. Replace with real calls:

```rust
CognitiveDelta::Consolidate { source_ids, summary } => {
    self.consolidate(source_ids, summary, actor)?;
}
CognitiveDelta::ForgetActor(actor_id) => {
    self.forget_actor(&actor_id)?;
}
CognitiveDelta::ArchiveRecord(id) => {
    self.archive_record(id)?;
}
```

Three new private methods added to `CognitiveHandle`.

---

## REST Changes

No new endpoints — these all go through `POST /v1/cognitive/transact` established in Phase 1.

Phase-1 error table updated: 501 entries for these three delta types are removed. Any 501 after Phase 4 is a bug.

**`TransactResponse` additions for ForgetActor:**
```json
{ "ok": true, "tx_cursor": 17, "records_deleted": 43 }
```

Add `records_deleted: Option<u32>` to `TransactResponse`.

---

## Episodic Compaction Helper (`src/consolidation.rs` — new file)

```rust
/// Louvain community detection on a petgraph subgraph.
/// Returns Vec<Vec<Uuid>> — one community per inner Vec.
pub fn detect_communities(
    store: &SymbolicStore,
    node_ids: &[Uuid],
) -> Vec<Vec<Uuid>>;

/// Contract one community: redirect edges to summary node.
pub fn contract_community(
    store: &mut SymbolicStore,
    community: &[Uuid],
    summary_id: Uuid,
) -> Result<(), String>;
```

Louvain implementation: use `petgraph` modularity optimization (greedy, single-pass for Phase 4). Full multi-pass left for post-v0.8.0.

---

## Files Changed

| File | Change |
|------|--------|
| `src/cognitive_state.rs` | Replace 3 stub arms with real impl; add `consolidate()`, `forget_actor()`, `archive_record()` methods; add `records_deleted` to `TransactResponse` |
| `src/consolidation.rs` | New: `detect_communities`, `contract_community` |
| `src/lib.rs` | Register `pub mod consolidation;` |
| `tests/unit/consolidation_tests.rs` | Unit tests: community detection, contract, edge redirect |
| `tests/unit/cognitive_state_tests.rs` | Add Consolidate/ForgetActor/ArchiveRecord integration tests |
| `tests/e2e_user_harness/suites/test_phase8_substrate.py` | G4-1..G4-5 live tests |

---

## Acceptance Gates

| Gate | Test |
|------|------|
| G4-1 | `Consolidate` with 3 source records → summary in Hot, sources in Cold, edges redirected |
| G4-2 | `ForgetActor("test-actor")` → 0 records remain for that actor in Hot or Cold |
| G4-3 | `ArchiveRecord(id)` with referenced record → in Cold, not in Hot |
| G4-4 | `ArchiveRecord(id)` with orphan record → deleted (not archived) |
| G4-5 | `Consolidate` with `source_ids.len() > 100` → 400 error |

---

## Non-Goals (Phase 4)

- Multi-pass Louvain (single-pass only)
- Scheduled / automatic consolidation triggers (post-v0.8.0)
- Semantic embedding-based community grouping (post-v0.8.0)
- ForgetActor audit trail (GDPR erases audit entries too by design)

# HipCortex v0.7.0-substrate Design Spec

> **Status:** Approved — ready for implementation plan
> **Ships:** First (before v0.7.0-beliefs)
> **Index:** [v0.7.0 Master Index](2026-08-15-v070-index.md)

## Goal

Build the TX-Log foundation, semantic tx-indexed StateDiff operator, and tag+actor memory compactor. This is the foundational layer all other v0.7.0 features depend on.

## Problem Statement

v0.6.0 has no monotonic transaction index. Without it:
- StateDiff is record-level only (single pair comparison), not range-level (what changed between state A and state B)
- Memory has no auto-compaction — episodic count grows unboundedly in long sessions
- Downstream (beliefs, calibration) have no anchor point for provenance

## Architecture

### Layer 0: `src/tx_log.rs` (new)

Every `MemoryStore` mutation appends one `TxEntry`. The counter is an `AtomicU64` (in-process CAS), restored from the last JSONL line on startup. Same append-only JSONL pattern as `MemoryStore` and `ArchiveStore`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxKind {
    MemoryAdd,
    MemoryUpdate,
    MemoryArchive,
    MemoryDelete,
    BeliefAssert,
    BeliefRetract,
    WorldModelObserve,
    WorldModelUpdate,
    GoalCreate,
    GoalStatusChange,
    Consolidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxEntry {
    pub tx_id: u64,
    pub timestamp_ms: u64,
    pub kind: TxKind,
    pub record_ids: Vec<uuid::Uuid>,
    pub actor: String,
}

pub struct TxLog {
    counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
    path: std::path::PathBuf,
}

impl TxLog {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, String>;
    pub fn append(&self, kind: TxKind, record_ids: Vec<uuid::Uuid>, actor: &str) -> u64;
    pub fn query_range(&self, from_tx: u64, to_tx: u64) -> Result<Vec<TxEntry>, String>;
    pub fn current_tx(&self) -> u64;
}
```

**Integration:** `AppState` gains `tx_log: Option<Arc<TxLog>>`. Every `handle_add_memory`, `handle_goal_react`, and world-model observe handler appends after successful write. Passive capture (v0.6.0) fires → `MemoryStore::add()` → `TxLog::append()` atomically. Zero extra agent code needed.

### Layer 1: `src/state_diff.rs` (new file — `src/memory_diff.rs` stays unchanged)

`src/memory_diff.rs` is NOT renamed or modified. `src/state_diff.rs` is a separate new file containing only tx-level types. The existing per-record `compute_diff` and `diff_snapshots` remain in `memory_diff.rs` forever. New tx-level operator:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStateDiff {
    pub from_tx: u64,
    pub to_tx: u64,
    pub timestamp_range: (u64, u64),
    pub tx_count: u64,
    pub memory_delta: MemoryDelta,
    pub world_model_delta: WorldModelDelta,
    pub causal_attributions: Vec<CausalAttributionPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDelta {
    pub added: Vec<uuid::Uuid>,
    pub archived: Vec<uuid::Uuid>,
    pub updated: Vec<uuid::Uuid>,
    pub net_delta: i64,          // positive = growth, negative = compaction occurred
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldModelDelta {
    pub observations_added: u32,
    pub distributions_updated: u32,
    pub causal_edges_added: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAttributionPath {
    pub record_id: uuid::Uuid,
    pub tx_id: u64,
    pub trigger_action: String,   // TxKind as string
    pub confidence_shift: f32,    // 0.0 if unknown, computed from MemoryRecord.confidence delta
}

pub fn compute_tx_diff(
    log: &TxLog,
    from_tx: u64,
    to_tx: u64,
    store: &crate::memory_store::MemoryStore,
) -> Result<TxStateDiff, String>;
// O(range_size) log replay. REST/MCP enforce: to_tx - from_tx ≤ 10_000.
```

**REST:** `POST /v1/state/diff` body `{ "from_tx": u64, "to_tx": u64 }` → `TxStateDiff` JSON.
**MCP:** new tool `compute_state_diff` → hits `POST /v1/state/diff`.

### Layer 2: `src/consolidation.rs` (new)

Greedy tag+actor clustering. No external graph algorithm deps.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationConfig {
    pub pressure_threshold: f32,   // default 0.80
    pub capacity_limit: usize,     // default 10_000
    pub min_group_size: usize,     // default 3
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self { pressure_threshold: 0.80, capacity_limit: 10_000, min_group_size: 3 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub groups_formed: usize,
    pub records_collapsed: usize,
    pub records_remaining: usize,
    pub edges_reanchored: usize,
    pub consolidation_tx_id: u64,
    pub pressure_before: f32,
    pub pressure_after: f32,
}

pub fn compute_pressure(store: &crate::memory_store::MemoryStore, config: &ConsolidationConfig) -> f32;

pub fn consolidate(
    store: &mut crate::memory_store::MemoryStore,
    graph: &mut crate::modules::symbolic_store::SymbolicStore,
    tx_log: &TxLog,
    config: &ConsolidationConfig,
) -> Result<ConsolidationReport, String>;
```

**Algorithm:**

1. Compute group key per episodic `MemoryRecord`: `key = format!("{}:{}", actor, sorted_tags.join(","))`
2. Collect groups where `group.len() ≥ config.min_group_size`
3. For each group:
   - Create `SummaryRecord` (`MemoryType::Temporal`, `action = "consolidated"`, `metadata = { group_size, time_range_ms: [min_ts, max_ts], confidence_mean }`)
   - Call `ArchiveStore::archive()` for each original record (respects Hot/Cold rule — never set `status="archived"` directly)
   - Re-anchor edges in `SymbolicStore`: ∀ edge `(u→v)` where `u ∈ group_ids` → add `(summary_id→v)`, remove old edge
4. Append `TxEntry { kind: TxKind::Consolidate, record_ids: all_group_record_ids }` to TX-log

**Auto-trigger:** `handle_add_memory` checks `compute_pressure() > config.pressure_threshold` after successful write. If true, fires `consolidate()` in background (spawn blocking task — does not block the HTTP response).

**Manual trigger:** `POST /v1/memory/consolidate` (body optional `ConsolidationConfig` override).
**MCP:** new tool `consolidate_memory`.

## REST Surface

```
POST /v1/state/diff          { from_tx: u64, to_tx: u64 } → TxStateDiff
POST /v1/memory/consolidate  {} | ConsolidationConfig     → ConsolidationReport
GET  /v1/state/tx            {}                            → { current_tx: u64 }
```

All routes require `web-server` feature. Add to router in `src/web_server.rs` alongside existing `/goal/:id/react` block.

## MCP Surface (`sdk/mcp/server.py`)

Two new tools added to `TOOLS` list and dispatch table:

```python
{
    "name": "compute_state_diff",
    "description": "Compute semantic diff between two cognitive state snapshots by tx range.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "from_tx": { "type": "integer", "description": "Start transaction ID (inclusive)" },
            "to_tx":   { "type": "integer", "description": "End transaction ID (inclusive)" }
        },
        "required": ["from_tx", "to_tx"]
    }
}

{
    "name": "consolidate_memory",
    "description": "Trigger hierarchical memory compaction. Groups episodic records by actor+tags, collapses groups into summary records, re-anchors graph edges.",
    "inputSchema": { "type": "object", "properties": {} }
}
```

Both hit REST via `requests.post(base_url + "/v1/state/diff", ...)` pattern — same as all existing 22 tools.

## Module Registration

Add to `src/lib.rs`:
```rust
pub mod tx_log;
pub mod state_diff;
pub mod consolidation;
```

Both modules registered independently — no re-export, no rename:
```rust
pub mod memory_diff;   // existing — unchanged
pub mod state_diff;    // new — tx-level StateDiff only
pub mod tx_log;
pub mod consolidation;
```

## Acceptance Gates

### Gate 1: StateDiff Verifiability
```
ΔS(tx_i, tx_i) == TxStateDiff with all deltas empty (identity)
After add(r1), add(r2): ΔS(tx_before_r1, current_tx).memory_delta.added contains [r1.id, r2.id]
causal_attributions non-empty when WorldModelObserve TxKind present in range
```

### Gate 2: Bounded Memory
```
Insert 10,000 MemoryRecord { type: Temporal, actor: "test", tags: ["a","b"] }
→ compaction auto-fires (P_tx > 0.80 at ~8,000 records)
→ store.count_hot() ≤ 100 after consolidation
→ BFS from each surviving node reaches same terminal sinks as before compaction
   (test via: record reachable_before, run consolidate, record reachable_after,
    assert reachable_after ⊇ (reachable_before ∩ surviving_nodes) at ≥ 98%)
```

### Gate 5: P95 Latency
```
compute_tx_diff over tx_range of 1,000 entries
Criterion bench with 100 samples → P95 < 5ms
```

## Test Files

| File | What it tests |
|------|---------------|
| `tests/unit/tx_log_tests.rs` | append monotonicity, query_range correctness, counter restore on reopen |
| `tests/unit/state_diff_tests.rs` | identity property, completeness, causal attribution presence |
| `tests/unit/consolidation_tests.rs` | pressure compute, grouping algorithm, edge re-anchor, ArchiveStore call |
| `tests/integration/consolidation_gates_sit.rs` | Gate 2 — 10k records + reachability |
| `benches/temporal_state_diff_bench.rs` | Gate 5 — P95 latency |

All unit test files registered in `tests/unit_suite.rs`. Integration file registered in `tests/integration_suite.rs`. Bench registered as `[[bench]] name = "temporal_state_diff_bench" harness = false` in `Cargo.toml`.

## Constraints

- Never add Louvain, DBSCAN, or external graph algorithm deps.
- `consolidate()` never calls `MemoryRecord.status = "archived"` directly — always via `ArchiveStore::archive()`.
- `TxLog::append()` is infallible from caller's perspective — log errors to stderr but don't fail the main write.
- `compute_tx_diff` range cap: `to_tx - from_tx > 10_000` returns `Err("tx range too large — cap at 10,000")`.
- All new REST routes under `/v1/` prefix (versioned namespace, distinct from existing unversioned routes).
